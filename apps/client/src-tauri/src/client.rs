use std::time::Duration;

use anyhow::Result;
use rust_common::flags::FLAG_HEARTBEAT;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    time::sleep,
};

pub async fn handle_heartbeat_stream(
    mut send: quinn::SendStream,
    mut recv: quinn::RecvStream,
) -> Result<()> {
    loop {
        send.write_u8(FLAG_HEARTBEAT).await?;
        recv.read_u8().await?;

        sleep(Duration::new(5, 0)).await;
    }
}
