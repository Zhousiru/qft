use std::{
  collections::HashSet, env, io::Cursor, net::SocketAddr, path::PathBuf, str::FromStr, sync::Arc,
  time::Duration,
};

use anyhow::{Context, Result};
use qft::common::{
  erasure::encode_file,
  flags::{
    FLAG_FILE_DECODE_OK, FLAG_HEARTBEAT, FLAG_REQUEST_ID, FLAG_UPLOAD_COMPLETE, FLAG_UPLOAD_PACKET,
  },
};
use tokio::{
  fs,
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
    None => 0,
  };
  if let Some(addr) = args.get(3) {
    server_addr = addr
      .parse()
      .context("Failed to parse listen address")
      .unwrap();
  }

  println!("Start encode file");
  let (config, packets) = encode_file(file_path, 0.1)
    .await
    .context("Failed to encode file")
    .unwrap();

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

  let (mut send, mut recv) = connection.open_bi().await.unwrap();
  send.write_u8(FLAG_REQUEST_ID).await.unwrap();
  send
    .write_u32(
      packets
        .len()
        .try_into()
        .context("Too many packets")
        .unwrap(),
    )
    .await
    .unwrap();
  send.write_u64(config.transfer_length()).await.unwrap();
  send.write_u16(config.symbol_size()).await.unwrap();
  send.write_u8(config.source_blocks()).await.unwrap();
  send.write_u16(config.sub_blocks()).await.unwrap();
  send.write_u8(config.symbol_alignment()).await.unwrap();
  let uuid = recv.read_u128().await.unwrap();
  println!("Get upload UUID: {}", uuid.to_string());

  let mut missing: Option<HashSet<u32>> = None;

  loop {
    let total = if let Some(ref map) = missing {
      map.len()
    } else {
      packets.len()
    };

    let mut interval = if qps != 0 {
      println!("Upload {} packets with QPS {}", total, qps);
      Some(time::interval(Duration::from_micros(1000000 / qps)))
    } else {
      println!("Upload {} packets without QPS limitation", total);
      None
    };

    for (index, packet) in packets.iter().enumerate() {
      if let Some(ref map) = missing {
        if !map.contains(&(index as u32 + 1)) {
          continue;
        }
      }

      let packet_datagram: Vec<u8> = vec![];
      let mut cur = Cursor::new(packet_datagram);
      cur.write_u8(FLAG_UPLOAD_PACKET).await.unwrap();
      cur.write_u128(uuid).await.unwrap();
      cur.write_u32(index as u32 + 1).await.unwrap();
      cur.write_all(&packet).await.unwrap();

      connection.send_datagram(cur.into_inner().into()).unwrap();

      if let Some(ref mut interval) = interval {
        interval.tick().await;
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

    missing = Some(HashSet::new());
    let missing_count = recv.read_u32().await.unwrap();
    println!(
      "Server failed to decode. Missing {} packets. Retry",
      missing_count
    );
    if let Some(ref mut map) = missing {
      for _ in 1..=missing_count {
        map.insert(recv.read_u32().await.unwrap());
      }
    }
  }

  connection.close(0u32.into(), b"");

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
