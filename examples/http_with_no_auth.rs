/// Example: Simple HTTP RPC Server (No Auth)
/// 
/// Run with:
/// cargo run --example http_simple --feature http

use dice_rpc::*;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create server
    let server = Arc::new(RpcServer::new());
    
    // Register handlers
    rpc::register_default_handlers(&server).await;
    
    println!("╔══════════════════════════════════════════════╗");
    println!("║     DiceRPC HTTP Server with no Auth           ║");
    println!("╚══════════════════════════════════════════════╝");
    println!();
    println!("Simple HTTP RPC server at: http://127.0.0.1:3000");
    println!();
    println!("Test with curl:");
    println!(r#"curl -X POST http://127.0.0.1:3000/rpc \"#);
    println!(r#"  -H "Content-Type: application/json" \"#);
    println!(r#"  -d '{{"jsonrpc":"2.0","method":"ping","params":{{}},"id":1}}'"#);

    // Start server (no auth)
    transport::HttpTransport::new(server)
        .serve("127.0.0.1:3000")
        .await?;

    Ok(())
}