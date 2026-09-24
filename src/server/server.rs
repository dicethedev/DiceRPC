use crate::rpc::{RpcServer, parse_rpc_request, register_default_handlers};
use anyhow::{Result, anyhow};
use std::sync::Arc;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

const MAX_LINE_SIZE: usize = 1024 * 1024;

pub async fn run(addr: &str) -> Result<()> {
    let server = Arc::new(RpcServer::new());
    register_default_handlers(&server).await;
    serve(addr, server).await
}

/// Serve a caller-configured RPC registry over newline-delimited TCP.
pub async fn serve(addr: &str, server: Arc<RpcServer>) -> Result<()> {
    let listener = TcpListener::bind(addr).await?;
    println!("DiceRPC server listening on {}", addr);

    loop {
        let (socket, _) = listener.accept().await?;
        let server = server.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_connection(server, socket).await {
                eprintln!("connection error: {:?}", e);
            }
        });
    }
}

async fn handle_connection(server: Arc<RpcServer>, stream: TcpStream) -> Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut br = BufReader::new(reader);

    loop {
        let Some(line) = read_bounded_line(&mut br, MAX_LINE_SIZE).await? else {
            break;
        };
        let raw = std::str::from_utf8(&line)?.trim_end_matches('\r');
        if raw.is_empty() {
            continue;
        }

        match parse_rpc_request(raw) {
            Ok(req) => {
                let resp = server.handle_request(req).await;
                let resp_text = serde_json::to_string(&resp)?;
                writer.write_all(resp_text.as_bytes()).await?;
                writer.write_all(b"\n").await?;
            }
            Err(e) => {
                // return parse error
                let err_resp = crate::rpc::RpcResponse::with_error(
                    serde_json::Value::Null,
                    -32700,
                    format!("Parse error: {}", e),
                );
                let resp_text = serde_json::to_string(&err_resp)?;
                writer.write_all(resp_text.as_bytes()).await?;
                writer.write_all(b"\n").await?;
            }
        }
    }

    Ok(())
}

async fn read_bounded_line<R>(reader: &mut R, max_len: usize) -> Result<Option<Vec<u8>>>
where
    R: AsyncBufRead + Unpin,
{
    let mut line = Vec::new();
    loop {
        let buffer = reader.fill_buf().await?;
        if buffer.is_empty() {
            return if line.is_empty() {
                Ok(None)
            } else {
                Ok(Some(line))
            };
        }

        let newline = buffer.iter().position(|byte| *byte == b'\n');
        let consumed = newline.map_or(buffer.len(), |index| index + 1);
        let content_len = newline.unwrap_or(consumed);

        if line.len().saturating_add(content_len) > max_len {
            return Err(anyhow!("Request line exceeds {max_len} bytes"));
        }

        line.extend_from_slice(&buffer[..content_len]);
        reader.consume(consumed);

        if newline.is_some() {
            return Ok(Some(line));
        }
    }
}
