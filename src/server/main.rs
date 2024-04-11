mod cert;

use std::{
  collections::{HashMap, HashSet},
  env,
  io::Cursor,
  net::SocketAddr,
  path::PathBuf,
  str::FromStr,
  sync::Arc,
};

use anyhow::{anyhow, Context, Result};
use once_cell::sync::Lazy;
use qft::common::{
  erasure::decode_file,
  flags::{
    FLAG_FILE_DECODE_ERROR, FLAG_FILE_DECODE_OK, FLAG_HEARTBEAT, FLAG_REQUEST_ID,
    FLAG_UPLOAD_COMPLETE, FLAG_UPLOAD_PACKET,
  },
};
use raptorq::ObjectTransmissionInformation;
use tokio::{
  fs::File,
  io::{AsyncReadExt, AsyncWriteExt},
  sync::Mutex,
};
use uuid::Uuid;

use crate::cert::get_self_signed_cert;

static TASKS: Lazy<Mutex<HashMap<u128, Task>>> = Lazy::new(|| Mutex::new(HashMap::new()));

struct Task {
  config: ObjectTransmissionInformation,
  packet_count: u32,
  recv_packets: Vec<(u32, Arc<bytes::Bytes>)>,
}

#[tokio::main]
async fn main() {
  let mut listen_addr = SocketAddr::from_str("127.0.0.1:23333").unwrap();
  let args: Vec<String> = env::args().collect();
  if let Some(addr) = args.get(1) {
    listen_addr = addr
      .parse()
      .context("Failed to parse listen address")
      .unwrap();
  }

  let (cert, key) = get_self_signed_cert().await.unwrap();

  let server_crypto = rustls::ServerConfig::builder()
    .with_safe_defaults()
    .with_no_client_auth()
    .with_single_cert(vec![cert], key)
    .unwrap();

  let server_config = quinn::ServerConfig::with_crypto(Arc::new(server_crypto));

  let endpoint = quinn::Endpoint::server(server_config, listen_addr).unwrap();
  println!("Listening on {}", listen_addr);

  while let Some(conn) = endpoint.accept().await {
    tokio::spawn(async move {
      let remote_addr = conn.remote_address();
      println!("Connection ({}) open", remote_addr);
      if let Err(e) = handle_connection(conn).await {
        println!("Connection ({}) failed: {}", remote_addr, e.to_string())
      }
    });
  }

  ()
}

async fn handle_connection(conn: quinn::Connecting) -> Result<()> {
  let c = conn.await.context("Failed to establish connection")?;
  let remote_addr = c.remote_address();

  let c_clone = c.clone();
  tokio::spawn(async move {
    loop {
      match c_clone.read_datagram().await {
        Err(e) => {
          println!(
            "Receive raw datagram ({}) failed: {}",
            remote_addr,
            e.to_string()
          );
          return;
        }
        Ok(datagram) => {
          tokio::spawn(async move {
            if let Err(e) = handle_raw_datagram(datagram).await {
              println!(
                "Handle raw datagram ({}) failed: {}",
                remote_addr,
                e.to_string()
              )
            }
          });
        }
      }
    }
  });

  loop {
    let stream = c.accept_bi().await;
    let stream = match stream {
      Err(e) => {
        return Err(e.into());
      }
      Ok(s) => s,
    };

    tokio::spawn(async move {
      if let Err(e) = handle_stream(stream).await {
        println!("Stream ({}) failed: {}", remote_addr, e.to_string())
      }
    });
  }
}

async fn handle_raw_datagram(datagram: bytes::Bytes) -> Result<()> {
  let datagram_clone = datagram.clone();
  let mut cur = Cursor::new(datagram_clone);

  let flag = cur.read_u8().await?;
  if flag != FLAG_UPLOAD_PACKET {
    return Err(anyhow!("invalid flag"));
  }

  let uuid = cur.read_u128().await?;
  let packet_id = cur.read_u32().await?;

  let packet = datagram.slice((cur.position() as usize)..datagram.len());

  let mut tasks = TASKS.lock().await;

  if let Some(task) = tasks.get_mut(&uuid) {
    task.recv_packets.push((packet_id, Arc::new(packet)))
  }

  Ok(())
}

async fn handle_stream((mut send, mut recv): (quinn::SendStream, quinn::RecvStream)) -> Result<()> {
  let flag = recv.read_u8().await?;

  match flag {
    FLAG_REQUEST_ID => {
      let packet_count = recv.read_u32().await?;
      let config = ObjectTransmissionInformation::new(
        recv.read_u64().await?,
        recv.read_u16().await?,
        recv.read_u8().await?,
        recv.read_u16().await?,
        recv.read_u8().await?,
      );

      let uuid = Uuid::new_v4();

      let mut tasks = TASKS.lock().await;
      tasks.insert(
        uuid.as_u128(),
        Task {
          config,
          packet_count,
          recv_packets: vec![],
        },
      );

      send.write_all(&uuid.into_bytes()).await?;
      Ok(())
    }

    FLAG_UPLOAD_COMPLETE => {
      let uuid = recv.read_u128().await?;

      let mut tasks = TASKS.lock().await;
      let task = tasks.get(&uuid).context("invalid id")?;

      let packets: Vec<Arc<bytes::Bytes>> =
        task.recv_packets.iter().map(|x| x.1.to_owned()).collect();

      println!(
        "Upload complete. Orig: {}, Recv: {}",
        task.packet_count,
        task.recv_packets.len()
      );

      // Decode and encode using spwan blocking
      match decode_file(task.config, packets).await {
        Ok(data) => {
          send.write_u8(FLAG_FILE_DECODE_OK).await?;
          tasks.remove(&uuid);

          let save_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(uuid.to_string());
          let mut file = File::create(save_path).await?;
          file.write_all_buf(&mut data.as_slice()).await?;

          Ok(())
        }
        Err(_) => {
          send.write_u8(FLAG_FILE_DECODE_ERROR).await?;

          let mut recv_ids: HashSet<u32> = HashSet::new();
          recv_ids.extend(task.recv_packets.iter().map(|x| x.0));

          let mut missing = vec![];
          for i in 1..=task.packet_count {
            if !recv_ids.contains(&i) {
              missing.push(i)
            }
          }

          send.write_u32(missing.len() as u32).await?;
          for i in missing {
            send.write_u32(i).await?;
          }

          Ok(())
        }
      }
    }

    FLAG_HEARTBEAT => loop {
      send.write_u8(FLAG_HEARTBEAT).await?;
      loop {
        recv.read_u8().await?;
        send.write_u8(FLAG_HEARTBEAT).await?;
      }
    },

    _ => Err(anyhow!("invalid flag")),
  }
}
