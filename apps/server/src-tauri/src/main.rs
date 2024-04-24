// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

use crate::commands::gen_cert;

fn main() {
  tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![gen_cert])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
