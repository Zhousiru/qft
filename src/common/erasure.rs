use std::io::SeekFrom;

use anyhow::{anyhow, Context, Result};
use once_cell::sync::Lazy;
use raptorq::{Decoder, Encoder, EncodingPacket, ObjectTransmissionInformation};
use tokio::{
  fs::File,
  io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt},
  task,
};

pub const BLOCK_SIZE: u64 = 1 * 1024 * 1024;
pub const MAX_PACKET_SIZE: u16 = 1024;

pub const TRANSFER_LENGTH: u64 = BLOCK_SIZE;
pub const SYMBOL_SIZE: u16 = 1024;
pub const SOURCE_BLOCKS: u8 = 1;
pub const SUB_BLOCKS: u16 = 1;
pub const ALIGNMENT: u8 = 8;

pub const DATA_PACKET_COUNT_PER_BLOCK: usize = BLOCK_SIZE as usize / MAX_PACKET_SIZE as usize;

static ENCODE_CONFIG: Lazy<ObjectTransmissionInformation> = Lazy::new(|| {
  ObjectTransmissionInformation::new(
    TRANSFER_LENGTH,
    SYMBOL_SIZE,
    SOURCE_BLOCKS,
    SUB_BLOCKS,
    ALIGNMENT,
  )
});

pub async fn encode_block(file: &File, block_id: u32, parity_rate: f32) -> Result<Vec<Vec<u8>>> {
  let mut file = file.try_clone().await?;

  file
    .seek(SeekFrom::Start(block_id as u64 * BLOCK_SIZE))
    .await?;

  let mut block_data = bytes::BytesMut::zeroed(BLOCK_SIZE as usize);
  let mut reader = file.take(BLOCK_SIZE);

  let mut read_bytes: usize = 0;
  while read_bytes < BLOCK_SIZE as usize {
    match reader.read(&mut block_data[read_bytes..]).await? {
      0 => break,
      n => read_bytes += n,
    }
  }

  let parity_per_block = (parity_rate * (BLOCK_SIZE as f32 / MAX_PACKET_SIZE as f32)) as u32;

  let packets = task::spawn_blocking(move || {
    let encoder = Encoder::new(&block_data, ENCODE_CONFIG.clone());
    let packets: Vec<Vec<u8>> = encoder
      .get_encoded_packets(parity_per_block)
      .iter()
      .map(|packet| packet.serialize())
      .collect();

    packets
  })
  .await
  .context("Failed to encode block")
  .unwrap();

  Ok(packets)
}

pub async fn decode_block(
  file: &File,
  block_id: u32,
  file_size: u64,
  packets: &Vec<bytes::Bytes>,
) -> Result<()> {
  let mut file = file.try_clone().await?;
  let mut packets = packets.clone();

  let data = task::spawn_blocking(move || {
    let mut decoder = Decoder::new(ENCODE_CONFIG.clone());
    let mut result;
    while !packets.is_empty() {
      result = decoder.decode(EncodingPacket::deserialize(&packets.pop().unwrap()));
      if let Some(decoded) = result {
        return Ok(decoded);
      }
    }
    Err(anyhow!("failed to decode file"))
  })
  .await??;

  let cursor_start = BLOCK_SIZE * block_id as u64;
  let data_end = if cursor_start + BLOCK_SIZE < file_size {
    BLOCK_SIZE as usize
  } else {
    (file_size - cursor_start) as usize
  };

  file.seek(SeekFrom::Start(cursor_start)).await?;
  file.write_all(&data[0..data_end]).await?;

  Ok(())
}
