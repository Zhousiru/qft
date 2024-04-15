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
  erasure::{decode_block, BLOCK_SIZE, DATA_PACKET_COUNT_PER_BLOCK},
  flags::{
    FLAG_FILE_DECODE_ERROR, FLAG_FILE_DECODE_OK, FLAG_HEARTBEAT, FLAG_REQUEST_ID,
    FLAG_UPLOAD_COMPLETE, FLAG_UPLOAD_PACKET,
  },
};
use tokio::{
  fs::{self, File},
  io::{self, AsyncReadExt, AsyncWriteExt},
  sync::Mutex,
};
use uuid::Uuid;

use crate::cert::get_self_signed_cert;

static TASKS: Lazy<Mutex<HashMap<u128, Task>>> = Lazy::new(|| Mutex::new(HashMap::new()));

struct Task {
  filename: String,
  file_size: u64,
  recv_blocks: HashMap<u32, HashSet<bytes::Bytes>>,
  rebuilt_blocks: HashSet<u32>,
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
  let block_id = cur.read_u32().await?;
  let packet = datagram.slice((cur.position() as usize)..datagram.len());

  let mut tasks = TASKS.lock().await;

  if let Some(task) = tasks.get_mut(&uuid) {
    if task.rebuilt_blocks.contains(&block_id) {
      return Ok(());
    }

    if task.recv_blocks.contains_key(&block_id) {
      let recv_map = task.recv_blocks.get_mut(&block_id).unwrap();
      let inserted = recv_map.insert(packet);

      if (recv_map.len() >= DATA_PACKET_COUNT_PER_BLOCK) && inserted {
        let result = decode_block(
          uuid,
          block_id,
          task.file_size,
          Vec::from_iter(recv_map.iter().map(|x| x.clone())),
        )
        .await;
        match result {
          Ok(()) => {
            task.rebuilt_blocks.insert(block_id);
            task.recv_blocks.remove(&block_id);
          }
          Err(_) => {
            println!(
              "Failed to decode block {}: {}/{}",
              block_id,
              recv_map.len(),
              DATA_PACKET_COUNT_PER_BLOCK
            )
          }
        }
      }
    } else {
      let mut recv = HashSet::new();
      recv.insert(packet);
      task.recv_blocks.insert(block_id, recv);
    }
  }

  Ok(())
}

async fn handle_stream((mut send, mut recv): (quinn::SendStream, quinn::RecvStream)) -> Result<()> {
  let flag = recv.read_u8().await?;

  match flag {
    FLAG_REQUEST_ID => {
      let file_size = recv.read_u64().await?;
      let filename = String::from_utf8(recv.read_to_end(1024).await?)?;

      let uuid = Uuid::new_v4();

      let mut tasks = TASKS.lock().await;
      tasks.insert(
        uuid.as_u128(),
        Task {
          filename,
          file_size,
          rebuilt_blocks: HashSet::new(),
          recv_blocks: HashMap::new(),
        },
      );

      send.write_all(&uuid.into_bytes()).await?;
      Ok(())
    }

    FLAG_UPLOAD_COMPLETE => {
      let uuid = recv.read_u128().await?;

      let mut tasks = TASKS.lock().await;
      let task = tasks.get(&uuid).context("Invalid ID")?;

      let total_blocks = (task.file_size as f32 / BLOCK_SIZE as f32).ceil() as usize;

      println!(
        "Client upload completed. Rebuilt blocks: {}/{}",
        task.rebuilt_blocks.len(),
        total_blocks
      );

      if task.rebuilt_blocks.len() == total_blocks {
        send.write_u8(FLAG_FILE_DECODE_OK).await?;
        println!("Received successfully");

        let base_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("recv");
        let tmp_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
          .join("tmp")
          .join(uuid.to_string());
        fs::create_dir_all(&base_path).await?;
        let mut output = File::create(base_path.join(&task.filename)).await?;
        for block_id in 0..total_blocks {
          let mut input = File::open(tmp_path.join(block_id.to_string())).await?;
          io::copy(&mut input, &mut output).await?;
        }

        println!("Merged successfully");

        fs::remove_dir_all(tmp_path).await?;
        tasks.remove(&uuid).unwrap();
        return Ok(());
      }

      send.write_u8(FLAG_FILE_DECODE_ERROR).await?;

      let mut missing: Vec<u32> = vec![];
      for i in 0..total_blocks {
        if !task.rebuilt_blocks.contains(&(i as u32)) {
          missing.push(i as u32)
        }
      }

      send.write_u32(missing.len() as u32).await?;
      for i in missing {
        send.write_u32(i).await?;
      }

      Ok(())
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
