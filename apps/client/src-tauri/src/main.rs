// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod client;
mod commands;
mod event;

use tokio::sync::RwLock;

pub use crate::commands::{connect_to_server, send_file};

pub struct ConnectionState(RwLock<Option<quinn::Connection>>);

fn main() {
  tauri::Builder::default()
    .manage(ConnectionState(RwLock::new(None)))
    .invoke_handler(tauri::generate_handler![connect_to_server, send_file])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
