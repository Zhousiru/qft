// Prevents additional console window on Windows in release, DO NOT REMOVE!!
// #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod event;
mod server;

use crate::commands::{gen_cert, start_server};

fn main() {
  tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![gen_cert, start_server])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
