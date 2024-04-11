pub const FLAG_LENGTH: usize = 8;

pub const FLAG_OK: u8 = 0b00000000;
pub const FLAG_ERROR: u8 = 0b00000001;

/// Request a unique ID for uploading. Next is the u32 packet count, u64 `transfer_length`, u16 `symbol_size`, u8 `num_source_blocks`, u16 `num_sub_blocks`, and u8 `symbol_alignment`
/// Response with u128 ID.
pub const FLAG_REQUEST_ID: u8 = 0b00000010;

/// Upload a file packet. Next is the u128 ID, u32 packet ID and packet content.
/// Response with OK.
pub const FLAG_UPLOAD_PACKET: u8 = 0b00000100;

/// Inform server upload complete. Next is the u128 ID.
pub const FLAG_UPLOAD_COMPLETE: u8 = 0b00001000;

/// Server decoded the file successfully.
pub const FLAG_FILE_DECODE_OK: u8 = FLAG_OK;

/// Server decoded the file failed. Next is the u32 missing packet length, and N u32 packet ID.
pub const FLAG_FILE_DECODE_ERROR: u8 = FLAG_ERROR;

/// Heartbeat.
/// Response with Heartbeat.
pub const FLAG_HEARTBEAT: u8 = 0b10000000;
