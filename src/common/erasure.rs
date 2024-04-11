use std::sync::Arc;

use anyhow::{anyhow, Result};
use raptorq::{Decoder, Encoder, EncodingPacket, ObjectTransmissionInformation};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use tokio::{fs::File, io::AsyncReadExt, task, time::Instant};

const MAX_PACKET_SIZE: u16 = 1024;

pub async fn encode_file(
  path: &str,
  parity_rate: f32,
) -> Result<(ObjectTransmissionInformation, Vec<Vec<u8>>)> {
  let mut file = File::open(path).await?;
  let mut data = Vec::new();
  file.read_to_end(&mut data).await?;

  let start = Instant::now();

  task::spawn_blocking(move || {
    let encoder = Encoder::with_defaults(&data, MAX_PACKET_SIZE);
    let config = encoder.get_config();
    let parity_per_block =
      ((data.len() / MAX_PACKET_SIZE as usize / config.source_blocks() as usize) as f32
        * parity_rate)
        .round() as u32;

    let packets: Vec<Vec<u8>> = encoder
      .get_encoded_packets(parity_per_block)
      .par_iter()
      .map(|packet| packet.serialize())
      .collect();

    println!("Encoded in {:?}", start.elapsed());

    Ok((config, packets))
  })
  .await?
}

pub async fn decode_file(
  config: ObjectTransmissionInformation,
  mut packets: Vec<Arc<bytes::Bytes>>,
) -> Result<Vec<u8>> {
  task::spawn_blocking(move || {
    let start = Instant::now();

    let mut decoder = Decoder::new(config);
    let mut result;
    while !packets.is_empty() {
      result = decoder.decode(EncodingPacket::deserialize(&packets.pop().unwrap()));
      if let Some(decoded) = result {
        println!("Decoded in {:?}", start.elapsed());
        return Ok(decoded);
      }
    }
    Err(anyhow!("failed to decode file"))
  })
  .await?
}
