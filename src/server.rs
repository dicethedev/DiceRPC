use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use crate::rpc::{RpcServer, parse_rpc_request, register_default_handlers};
use anyhow::Result;
use std::sync::Arc;

pub async fn run(addr: &str) -> Result<()> {
    let listener = TcpListener::bind(addr).await?;
    println!("DiceRPC server listening on {}", addr);

    // create server and register handlers
    let server = Arc::new(RpcServer::new());
    register_default_handlers(&server).await;

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
    let mut line = String::new();

    loop {
        line.clear();
        let n = br.read_line(&mut line).await?;
        if n == 0 {
            // EOF
            break;
        }

        // trim newline
        let raw = line.trim_end();
        if raw.is_empty() {
            continue;
        }

        match parse_rpc_request(raw) {
            Ok(req) => {
                let resp = server.handle_request(req).await;
                let resp_text = serde_json::to_string(&resp).unwrap();
                writer.write_all(resp_text.as_bytes()).await?;
                writer.write_all(b"\n").await?;
            }
            Err(e) => {
                // return parse error
                let err_resp = crate::rpc::RpcResponse::with_error(serde_json::Value::Null, -32700, format!("Parse error: {}", e));
                let resp_text = serde_json::to_string(&err_resp).unwrap();
                writer.write_all(resp_text.as_bytes()).await?;
                writer.write_all(b"\n").await?;
            }
        }
    }

    Ok(())
}
