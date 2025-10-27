use tokio::io::{AsyncReadExt, AsyncWriteExt};
use anyhow::{Result, anyhow};

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
        W: AsyncWriteExt + Unpin,
    {
        if data.len() > u32::MAX as usize {
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
        R: AsyncReadExt + Unpin,
    {
        // Read 4-byte length prefix
        let mut len_bytes = [0u8; 4];
        reader.read_exact(&mut len_bytes).await?;
        
        let len = u32::from_be_bytes(len_bytes) as usize;
        
        // Sanity check: prevent extremely large allocations
        if len > 10_000_000 { // 10MB max
            return Err(anyhow!("Frame too large: {} bytes", len));
        }
        
        // Read payload
        let mut payload = vec![0u8; len];
        reader.read_exact(&mut payload).await?;
        
        Ok(payload)
    }
}

