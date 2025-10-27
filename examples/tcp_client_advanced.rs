/// Advanced TCP client with framing support
/// 
/// Run with:
/// cargo run --example tcp_client_advanced --features tcp

use dice_rpc::transport::FrameCodec;
use serde_json::json;
use tokio::net::TcpStream;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let addr = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:4000".to_string());

    println!("Connecting to {}...", addr);
    let mut stream = TcpStream::connect(&addr).await?;
    println!("Connected!");
    println!();

    // Example 1: Simple ping
    println!("Testing ping...");
    let ping_req = json!({
        "jsonrpc": "2.0",
        "method": "ping",
        "params": {"api_key": "dev-secret-key"},
        "id": 1
    });
    
    let req_bytes = serde_json::to_vec(&ping_req)?;
    FrameCodec::write_frame(&mut stream, &req_bytes).await?;
    
    let resp_bytes = FrameCodec::read_frame(&mut stream).await?;
    let response: serde_json::Value = serde_json::from_slice(&resp_bytes)?;
    println!("Response: {}", serde_json::to_string_pretty(&response)?);
    println!();

    // Example 2: Get balance
    println!("Getting balance for 0xAlice...");
    let balance_req = json!({
        "jsonrpc": "2.0",
        "method": "get_balance",
        "params": {
            "address": "0xAlice",
            "api_key": "dev-secret-key"
        },
        "id": 2
    });
    
    let req_bytes = serde_json::to_vec(&balance_req)?;
    FrameCodec::write_frame(&mut stream, &req_bytes).await?;
    
    let resp_bytes = FrameCodec::read_frame(&mut stream).await?;
    let response: serde_json::Value = serde_json::from_slice(&resp_bytes)?;
    println!("Response: {}", serde_json::to_string_pretty(&response)?);
    println!();

    // Example 3: Batch request
    println!("Sending batch request...");
    let batch_req = json!([
        {
            "jsonrpc": "2.0",
            "method": "get_balance",
            "params": {"address": "0xAlice", "api_key": "dev-secret-key"},
            "id": 3
        },
        {
            "jsonrpc": "2.0",
            "method": "get_balance",
            "params": {"address": "0xBob", "api_key": "dev-secret-key"},
            "id": 4
        },
        {
            "jsonrpc": "2.0",
            "method": "list_accounts",
            "params": {"api_key": "dev-secret-key"},
            "id": 5
        }
    ]);
    
    let req_bytes = serde_json::to_vec(&batch_req)?;
    FrameCodec::write_frame(&mut stream, &req_bytes).await?;
    
    let resp_bytes = FrameCodec::read_frame(&mut stream).await?;
    let response: serde_json::Value = serde_json::from_slice(&resp_bytes)?;
    println!("Response: {}", serde_json::to_string_pretty(&response)?);
    println!();

    println!("All tests completed!");

    Ok(())
}