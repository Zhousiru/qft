use std::{
  env, io::Cursor, net::SocketAddr, path::PathBuf, str::FromStr, sync::Arc, time::Duration,
};

use anyhow::{Context, Result};
use qft::common::{
  erasure::{encode_block, BLOCK_SIZE},
  flags::{
    FLAG_FILE_DECODE_OK, FLAG_HEARTBEAT, FLAG_REQUEST_ID, FLAG_UPLOAD_COMPLETE, FLAG_UPLOAD_PACKET,
  },
};
use tokio::{
  fs::{self, File},
  io::{AsyncReadExt, AsyncWriteExt},
  time::{self, sleep},
};

#[tokio::main]
async fn main() {
  let mut server_addr = SocketAddr::from_str("127.0.0.1:23333").unwrap();
  let args: Vec<String> = env::args().collect();
  let file_path = args.get(1).context("No file path provided").unwrap();
  let qps: u64 = match args.get(2) {
    Some(v) => v.parse().context("Invalid QPS").unwrap(),
    None => 180000,
  };
  if let Some(addr) = args.get(3) {
    server_addr = addr
      .parse()
      .context("Failed to parse listen address")
      .unwrap();
  }

  println!("Setup client");
  let base_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
  let cert = fs::read(base_path.join("cert").join("cert.der"))
    .await
    .unwrap();
  let cert = rustls::Certificate(cert);

  let mut root_store = rustls::RootCertStore::empty();
  root_store.add(&cert).unwrap();

  let client_crypto = rustls::ClientConfig::builder()
    .with_safe_defaults()
    .with_root_certificates(root_store)
    .with_no_client_auth();
  let client_config = quinn::ClientConfig::new(Arc::new(client_crypto));

  let mut endpoint = quinn::Endpoint::client("0.0.0.0:0".parse().unwrap()).unwrap();
  endpoint.set_default_client_config(client_config);

  let connection = endpoint
    .connect(server_addr, "qft-server")
    .unwrap()
    .await
    .unwrap();

  let connection_clone = connection.clone();
  tokio::spawn(async move {
    println!("Open heartbeat stream");
    let (send, recv) = connection_clone.open_bi().await.unwrap();

    match handle_heartbeat_stream(send, recv).await {
      Err(e) => {
        println!("Heartbeat failed: {}", e)
      }
      _ => {}
    }
  });

  let path_buf = PathBuf::from(file_path);
  let file = File::open(&path_buf).await.unwrap();

  let file_size = file.metadata().await.unwrap().len();
  let block_count: u32 = (file_size as f32 / BLOCK_SIZE as f32).ceil() as u32;

  let (mut send, mut recv) = connection.open_bi().await.unwrap();
  send.write_u8(FLAG_REQUEST_ID).await.unwrap();
  send.write_u64(file_size).await.unwrap();
  send
    .write_all(
      path_buf
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string()
        .as_bytes(),
    )
    .await
    .unwrap();
  send.finish().await.unwrap();

  let uuid = recv.read_u128().await.unwrap();
  println!("Get upload UUID: {}", uuid.to_string());

  let mut missing: Vec<u32> = (0..block_count).collect();

  loop {
    let mut interval = if qps != 0 {
      println!("Upload {} blocks with QPS {}", missing.len(), qps);
      Some(time::interval(Duration::from_micros(1000000 / qps)))
    } else {
      println!("Upload {} blocks without QPS limitation", missing.len());
      None
    };

    for block_id in missing.iter() {
      let packets = encode_block(&file, *block_id, 0.1).await.unwrap();
      for packet in packets {
        let packet_datagram: Vec<u8> = vec![];
        let mut cur = Cursor::new(packet_datagram);
        cur.write_u8(FLAG_UPLOAD_PACKET).await.unwrap();
        cur.write_u128(uuid).await.unwrap();
        cur.write_u32(*block_id).await.unwrap();
        cur.write_all(&packet).await.unwrap();

        connection.send_datagram(cur.into_inner().into()).unwrap();

        if let Some(ref mut interval) = interval {
          interval.tick().await;
        }
      }
    }

    println!("Upload complete");
    let (mut send, mut recv) = connection.open_bi().await.unwrap();
    send.write_u8(FLAG_UPLOAD_COMPLETE).await.unwrap();
    send.write_u128(uuid).await.unwrap();

    if recv.read_u8().await.unwrap() == FLAG_FILE_DECODE_OK {
      println!("Server confirmed decoded successfully");
      break;
    }

    let missing_block_count = recv.read_u32().await.unwrap();
    println!(
      "Server failed to decode. Missing {} blocks. Retry",
      missing_block_count
    );

    missing.clear();
    for _ in 0..missing_block_count {
      missing.push(recv.read_u32().await.unwrap());
    }
  }

  connection.close(0u32.into(), b"ok");

  ()
}

async fn handle_heartbeat_stream(
  mut send: quinn::SendStream,
  mut recv: quinn::RecvStream,
) -> Result<()> {
  loop {
    send.write_u8(FLAG_HEARTBEAT).await?;
    recv.read_u8().await?;

    sleep(Duration::new(5, 0)).await;
  }
}
