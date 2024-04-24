use rust_common::erasure::BLOCK_SIZE;
use tauri::{AppHandle, Manager};

use crate::server::Task;

#[derive(Clone, serde::Serialize)]
pub enum TaskStatus {
  #[serde(rename = "recv")]
  Recv,
  #[serde(rename = "merge")]
  Merge,
  #[serde(rename = "done")]
  Done,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskEvent {
  pub filename: String,
  pub file_size: u64,
  pub uuid: String,
  pub block_count: u32,
  pub done_block_count: u32,
  pub status: TaskStatus,
}

pub fn emit_task_event(app_handle: &AppHandle, uuid: u128, task: &Task, status: TaskStatus) {
  app_handle
    .emit_all(
      "task",
      TaskEvent {
        filename: task.filename.clone(),
        file_size: task.file_size,
        uuid: uuid.to_string(),
        block_count: (task.file_size as f32 / BLOCK_SIZE as f32).ceil() as u32,
        done_block_count: task.rebuilt_blocks.len() as u32,
        status,
      },
    )
    .unwrap();
}
