use anyhow::{Result, anyhow};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub const DEFAULT_MAX_FRAME_SIZE: usize = 1024 * 1024;

/// Frame format: 4-byte length prefix (big-endian) + message payload
/// This is more robust than newline delimiting and handles binary data properly
pub struct FrameCodec;

impl FrameCodec {
    /// Writes a length-prefixed frame to the writer
    ///
    /// Format: [4-byte length][payload]
    /// Length is the size of the payload in bytes (u32, big-endian)
    pub async fn write_frame<W>(writer: &mut W, data: &[u8]) -> Result<()>
    where
        W: AsyncWrite + Unpin,
    {
        Self::write_frame_with_limit(writer, data, DEFAULT_MAX_FRAME_SIZE).await
    }

    pub async fn write_frame_with_limit<W>(
        writer: &mut W,
        data: &[u8],
        max_frame_size: usize,
    ) -> Result<()>
    where
        W: AsyncWrite + Unpin,
    {
        if data.len() > max_frame_size || data.len() > u32::MAX as usize {
            return Err(anyhow!("Message too large: {} bytes", data.len()));
        }

        let len = data.len() as u32;
        let len_bytes = len.to_be_bytes();

        // Write length prefix
        writer.write_all(&len_bytes).await?;
        // Write payload
        writer.write_all(data).await?;

        Ok(())
    }

    /// Reads a length-prefixed frame from the reader
    ///
    /// Returns the payload bytes or an error if EOF or invalid frame
    pub async fn read_frame<R>(reader: &mut R) -> Result<Vec<u8>>
    where
        R: AsyncRead + Unpin,
    {
        Self::read_frame_with_limit(reader, DEFAULT_MAX_FRAME_SIZE).await
    }

    pub async fn read_frame_with_limit<R>(reader: &mut R, max_frame_size: usize) -> Result<Vec<u8>>
    where
        R: AsyncRead + Unpin,
    {
        // Read 4-byte length prefix
        let mut len_bytes = [0u8; 4];
        reader.read_exact(&mut len_bytes).await?;

        let len = u32::from_be_bytes(len_bytes) as usize;

        // Sanity check: prevent extremely large allocations
        if len > max_frame_size {
            return Err(anyhow!(
                "Frame too large: {len} bytes (maximum {max_frame_size})"
            ));
        }

        // Read payload
        let mut payload = vec![0u8; len];
        reader.read_exact(&mut payload).await?;

        Ok(payload)
    }
}
