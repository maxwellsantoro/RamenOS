// V-007 Phase 2: Framing protocol for store service IPC
//
// Implements length-prefixed binary messages over Unix domain sockets.
// Format: [4 bytes: length (little-endian u32)] [N bytes: payload]
//
// Security considerations:
// - MAX_MESSAGE_SIZE prevents memory exhaustion attacks
// - Length validation prevents buffer overflows
// - Message boundaries preserved (Unix domain sockets)

use anyhow::Context;
use std::io::{Read, Write};

const MAX_MESSAGE_SIZE: usize = 16 * 1024 * 1024; // 16MB

/// Read a length-prefixed message from a stream
///
/// # Format
/// - Reads 4 bytes (little-endian u32) for message length
/// - Reads N bytes for message payload
/// - Validates length against MAX_MESSAGE_SIZE
///
/// # Errors
/// - Returns error if read fails (connection closed, timeout, etc.)
/// - Returns error if message exceeds MAX_MESSAGE_SIZE
/// - Returns error if length is malformed
pub fn read_message<R: Read>(reader: &mut R) -> anyhow::Result<Vec<u8>> {
    // Read length prefix
    let mut len_bytes = [0u8; 4];
    reader
        .read_exact(&mut len_bytes)
        .context("failed to read message length")?;

    let len = u32::from_le_bytes(len_bytes) as usize;

    // Validate length
    if len > MAX_MESSAGE_SIZE {
        anyhow::bail!(
            "message too large: {} bytes (max {})",
            len,
            MAX_MESSAGE_SIZE
        );
    }

    // Read payload
    let mut payload = vec![0u8; len];
    reader
        .read_exact(&mut payload)
        .context("failed to read message payload")?;

    Ok(payload)
}

/// Write a length-prefixed message to a stream
///
/// # Format
/// - Writes 4 bytes (little-endian u32) for message length
/// - Writes N bytes for message payload
/// - Validates length against MAX_MESSAGE_SIZE
///
/// # Errors
/// - Returns error if write fails (connection closed, disk full, etc.)
/// - Returns error if message exceeds MAX_MESSAGE_SIZE
pub fn write_message<W: Write>(writer: &mut W, payload: &[u8]) -> anyhow::Result<()> {
    // Validate length
    if payload.len() > MAX_MESSAGE_SIZE {
        anyhow::bail!(
            "message too large: {} bytes (max {})",
            payload.len(),
            MAX_MESSAGE_SIZE
        );
    }

    // Write length prefix
    let len_bytes = (payload.len() as u32).to_le_bytes();
    writer
        .write_all(&len_bytes)
        .context("failed to write message length")?;

    // Write payload
    writer
        .write_all(payload)
        .context("failed to write message payload")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_and_read_round_trip() {
        let payload = b"Hello, store service!";
        let mut buffer = Vec::new();

        // Write message
        write_message(&mut buffer, payload).unwrap();

        // Verify format: [length (4 bytes LE)] [payload]
        assert_eq!(buffer.len(), 4 + payload.len());
        let len_bytes = &buffer[0..4];
        assert_eq!(
            u32::from_le_bytes(len_bytes.try_into().unwrap()),
            payload.len() as u32
        );
        assert_eq!(&buffer[4..], payload);

        // Read message
        let mut cursor = std::io::Cursor::new(buffer);
        let read_payload = read_message(&mut cursor).unwrap();

        assert_eq!(&read_payload[..], payload);
    }

    #[test]
    fn read_empty_message() {
        let empty: Vec<u8> = vec![0, 0, 0, 0]; // Length 0
        let mut cursor = std::io::Cursor::new(empty);

        let payload = read_message(&mut cursor).unwrap();
        assert_eq!(payload.len(), 0);
    }

    #[test]
    fn read_message_rejects_oversized_message() {
        let oversized_len = MAX_MESSAGE_SIZE + 1;
        let len_bytes = (oversized_len as u32).to_le_bytes();

        let mut cursor = std::io::Cursor::new(len_bytes.to_vec());
        let result = read_message(&mut cursor);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("message too large"));
    }

    #[test]
    fn write_message_rejects_oversized_payload() {
        let oversized = vec![0u8; MAX_MESSAGE_SIZE + 1];
        let mut buffer = Vec::new();

        let result = write_message(&mut buffer, &oversized);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("message too large"));
    }

    #[test]
    fn max_message_size_is_16mb() {
        assert_eq!(MAX_MESSAGE_SIZE, 16 * 1024 * 1024);
    }
}
