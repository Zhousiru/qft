#[derive(Clone, serde::Serialize)]
pub enum TaskStatus {
  #[serde(rename = "send")]
  Send,
  #[serde(rename = "done")]
  Done,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskEvent {
  pub filename: String,
  pub file_size: u64,
  pub pps: u64,
  pub uuid: String,
  pub block_count: u32,
  pub remain_block_count: u32,
  pub status: TaskStatus,
}
