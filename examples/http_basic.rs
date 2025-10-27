/// Basic HTTP RPC server example
/// 
/// Run with:
/// cargo run --example http_basic --features http

use dice_rpc::*;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("╔══════════════════════════════════════╗");
    println!("║     DiceRPC Basic HTTP Server        ║");
    println!("╚══════════════════════════════════════╝");
    println!();

    // Create server
    let server = Arc::new(RpcServer::new());
    
    // Register default handlers
    rpc::register_default_handlers(&server).await;

    let addr = "127.0.0.1:3000";
    println!("Server listening on http://{}", addr);
    println!();
    println!("Test with curl:");
    println!(r#"curl -X POST http://127.0.0.1:3000/rpc \"#);
    println!(r#"  -H "Content-Type: application/json" \"#);
    println!(r#"  -d '{{"jsonrpc":"2.0","method":"ping","params":{{}},"id":1}}'"#);
    println!();

    // Run server (no auth)
    transport::HttpTransport::new(server)
        .serve(addr)
        .await?;

    Ok(())
}