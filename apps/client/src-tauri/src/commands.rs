use std::{io::Cursor, net::SocketAddr, path::PathBuf, str::FromStr, sync::Arc, time::Duration};

use rust_common::{
  erasure::{encode_block, BLOCK_SIZE},
  flags::{FLAG_FILE_DECODE_OK, FLAG_REQUEST_ID, FLAG_UPLOAD_COMPLETE, FLAG_UPLOAD_PACKET},
};
use tauri::{AppHandle, Manager};
use tokio::{
  fs::{self, File},
  io::{AsyncReadExt, AsyncWriteExt},
  time,
};

use crate::{
  client::handle_heartbeat_stream,
  event::{TaskEvent, TaskStatus},
  ConnectionState,
};

#[tauri::command]
pub async fn connect_to_server(
  app_handle: AppHandle,
  state: tauri::State<'_, ConnectionState>,
  addr: String,
) -> Result<(), ()> {
  let server_addr = SocketAddr::from_str(&addr).unwrap();

  println!("Setup client");
  let base_path = app_handle.path_resolver().app_data_dir().unwrap();
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

  let mut connection_state_gurad = state.0.write().await;
  *connection_state_gurad = Some(connection);

  Ok(())
}

#[tauri::command]
pub async fn send_file(
  app_handle: AppHandle,
  state: tauri::State<'_, ConnectionState>,
  path: String,
  pps: u64,
) -> Result<(), ()> {
  let connection_state_gurad = state.0.read().await;
  let connection = connection_state_gurad.as_ref().unwrap().clone();

  tokio::spawn(async move {
    let path_buf = PathBuf::from(path);
    let file = File::open(&path_buf).await.unwrap();

    let filename = path_buf.file_name().unwrap().to_string_lossy().to_string();
    let file_size = file.metadata().await.unwrap().len();
    let block_count: u32 = (file_size as f32 / BLOCK_SIZE as f32).ceil() as u32;

    let (mut send, mut recv) = connection.open_bi().await.unwrap();
    send.write_u8(FLAG_REQUEST_ID).await.unwrap();
    send.write_u64(file_size).await.unwrap();
    send.write_all(filename.clone().as_bytes()).await.unwrap();
    send.finish().await.unwrap();

    let uuid = recv.read_u128().await.unwrap();
    println!("Get upload UUID: {}", uuid.to_string());

    let mut missing: Vec<u32> = (0..block_count).collect();

    let start_time = time::Instant::now();
    let mut interval = time::interval(Duration::from_micros(1000000 / pps));

    loop {
      app_handle
        .emit_all(
          "task",
          TaskEvent {
            filename: filename.clone(),
            file_size,
            pps,
            uuid: uuid.to_string(),
            block_count,
            remain_block_count: missing.len() as u32,
            status: TaskStatus::Send,
          },
        )
        .unwrap();

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

          interval.tick().await;
        }
      }

      println!("Upload complete");
      let (mut send, mut recv) = connection.open_bi().await.unwrap();
      send.write_u8(FLAG_UPLOAD_COMPLETE).await.unwrap();
      send.write_u128(uuid).await.unwrap();

      if recv.read_u8().await.unwrap() == FLAG_FILE_DECODE_OK {
        println!("Server confirmed decoded successfully");
        let elapsed = time::Instant::elapsed(&start_time);
        println!(
          "Done in {:?}. Average speed: {:.2} MiB/s",
          elapsed,
          file_size as f64 / 1024.0 / 1024.0 / elapsed.as_secs_f64()
        );

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

    app_handle
      .emit_all(
        "task",
        TaskEvent {
          filename: filename.clone(),
          file_size,
          pps,
          uuid: uuid.to_string(),
          block_count,
          remain_block_count: 0,
          status: TaskStatus::Done,
        },
      )
      .unwrap();
  });

  Ok(())
}
