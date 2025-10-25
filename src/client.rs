use clap::Parser;
use serde_json::json;
use tokio::net::TcpStream;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[derive(Parser, Debug)]
pub struct ClientArgs {
    /// Server address like 127.0.0.1:4000
    #[arg(short, long, default_value = "127.0.0.1:4000")]
    pub addr: String,

    /// Method to call (ping|get_balance|send_tx)
    #[arg(short, long)]
    pub method: String,

    /// Params as JSON string, e.g. '{"address":"0xabc"}'
    #[arg(short, long, default_value = "{}")]
    pub params: String,
}


pub async fn run_client(args: ClientArgs) -> anyhow::Result<()> {
    let stream = TcpStream::connect(&args.addr).await?;
    
    // Split the stream into read and write halves
    let (read_half, mut write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half);

    // Build the JSON-RPC request
    let id = serde_json::Value::Number(serde_json::Number::from(1u64));
    let params_value: serde_json::Value = serde_json::from_str(&args.params)?;
    let req = json!({
        "jsonrpc": "2.0",
        "method": args.method,
        "params": params_value,
        "id": id
    });

    let req_text = serde_json::to_string(&req)? + "\n";
    write_half.write_all(req_text.as_bytes()).await?;

    // Read response
    let mut line = String::new();
    reader.read_line(&mut line).await?;
    println!("Response: {}", line.trim_end());

    Ok(())
}

