use std::path::PathBuf;

use tauri::AppHandle;
use tokio::fs;

use crate::server::server_thread;

#[tauri::command]
pub async fn gen_cert(app_handle: AppHandle) {
  let base_path = PathBuf::from(app_handle.path_resolver().app_data_dir().unwrap()).join("cert");
  let cert_path = base_path.join("cert.der");
  let key_path = base_path.join("key.der");

  let gen_cert = rcgen::generate_simple_self_signed(vec!["qft-server".into()]).unwrap();

  let cert = gen_cert.serialize_der().unwrap();
  let key = gen_cert.serialize_private_key_der();

  fs::create_dir_all(base_path).await.unwrap();
  fs::write(&cert_path, &cert).await.unwrap();
  fs::write(&key_path, &key).await.unwrap();
}

#[tauri::command]
pub async fn start_server(app_handle: AppHandle) {
  tokio::spawn(async move { server_thread(app_handle).await });
}
