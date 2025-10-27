/// Full-featured HTTP server with all features
/// 
/// Run with:
/// cargo run --example http_full_featured --features http

use dice_rpc::*;
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    server::metrics::init_logging();

    println!("╔══════════════════════════════════════════════╗");
    println!("║  DiceRPC Full-Featured HTTP Server          ║");
    println!("╚══════════════════════════════════════════════╝");
    println!();

    // Create all components
    let server = Arc::new(RpcServer::new());
    let state = Arc::new(state::StateStore::new());
    let metrics = Arc::new(dice_rpc::Metrics::new());

    // Initialize demo accounts
    println!("Initializing demo accounts...");
    state.set_balance("0xAlice", 100000).await;
    state.set_balance("0xBob", 50000).await;
    state.set_balance("0xCharlie", 75000).await;
    state.set_balance("0xDiana", 25000).await;
    println!("Accounts initialized");
    println!();

    // Register handlers
    server::handlers::register_stateful_handlers(&server, state.clone()).await;

    // Setup authentication
    let auth = Arc::new(middleware::AuthMiddleware::new(
        middleware::AuthStrategy::ApiKeyInParams
    ));
    
    // Load keys from environment or use defaults
    if let Ok(keys) = std::env::var("API_KEYS") {
        for key in keys.split(',') {
            auth.add_key(key.trim()).await;
        }
        println!("Loaded API keys from environment");
    } else {
        auth.add_key("dev-secret-key").await;
        auth.add_key("prod-secret-key").await;
        println!("Using default API keys: dev-secret-key, prod-secret-key");
    }
    println!();

    // Spawn metrics reporter
    let metrics_clone = metrics.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(30)).await;
            let snapshot = metrics_clone.snapshot().await;
            tracing::info!("Metrics Report");
            tracing::info!("Total Requests: {}", snapshot.total_requests);
            tracing::info!("Successful: {}", snapshot.total_success);
            tracing::info!("Errors: {}", snapshot.total_errors);
            tracing::info!("Avg Duration: {}μs", snapshot.avg_duration_us);
            tracing::info!("Method Counts: {:?}", snapshot.method_counts);
        }
    });

    // Spawn account monitor
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
            let accounts = state.get_all_accounts().await;
            tracing::info!("💰 Account Balances:");
            for acc in accounts {
                tracing::info!("  {}: {}", acc.address, acc.balance);
            }
        }
    });

    let addr = std::env::var("HTTP_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:3000".to_string());

    server::metrics::log_startup(&addr, "HTTP");
    println!();
    println!("Features enabled:");
    println!("HTTP/REST transport");
    println!("Authentication (API key in params)");
    println!("Metrics collection");
    println!("Persistent state");
    println!("Batch request support");
    println!();
    println!("Endpoints:");
    println!("  POST http://{}/", addr);
    println!("  POST http://{}/rpc", addr);
    println!();
    println!("Available methods:");
    println!("  ping, get_balance, set_balance, transfer,");
    println!("  get_transaction, confirm_transaction,");
    println!("  get_transactions, list_accounts");
    println!();
    println!("Example requests:");
    println!();
    println!("Single request:");
    println!(r#"curl -X POST http://{}/rpc \"#, addr);
    println!(r#"  -H "Content-Type: application/json" \"#);
    println!(r#"  -d '{{"jsonrpc":"2.0","method":"ping","params":{{"api_key":"dev-secret-key"}},"id":1}}'"#);
    println!();
    println!("Batch request:");
    println!(r#"curl -X POST http://{}/rpc \"#, addr);
    println!(r#"  -H "Content-Type: application/json" \"#);
    println!(r#"  -d '[{{"jsonrpc":"2.0","method":"ping","params":{{"api_key":"dev-secret-key"}},"id":1}},{{"jsonrpc":"2.0","method":"list_accounts","params":{{"api_key":"dev-secret-key"}},"id":2}}]'"#);
    println!();
    println!("Press Ctrl+C for graceful shutdown");
    println!();

    // Run server
    transport::HttpTransport::new(server)
        .with_auth(auth)
        .serve(&addr)
        .await?;

    server::metrics::log_shutdown();
    
    Ok(())
}