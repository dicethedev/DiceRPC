/// Full-featured TCP server with all bells and whistles
/// 
/// Run with:
/// cargo run --example tcp_full_featured --features tcp

use dice_rpc::*;
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    server::metrics::init_logging();

    println!("╔══════════════════════════════════════════════╗");
    println!("║  DiceRPC Full-Featured TCP Server           ║");
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
            tracing::info!("Account Balances:");
            for acc in accounts {
                tracing::info!("  {}: {}", acc.address, acc.balance);
            }
        }
    });

    // Configure server
    let addr = std::env::var("BIND_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:4000".to_string());
    
    let config = transport::tcp::TcpServerConfig::new(&addr, server)
        .with_auth(auth)
        .with_metrics(metrics);

    server::metrics::log_startup(&addr, "TCP (Framed)");
    println!();
    println!("Features enabled:");
    println!("Length-prefixed framing");
    println!("Authentication (API key in params)");
    println!("Metrics collection");
    println!("Persistent state");
    println!("Graceful shutdown (Ctrl+C)");
    println!("Batch request support");
    println!();
    println!("Available methods:");
    println!("  ping, get_balance, set_balance, transfer,");
    println!("  get_transaction, confirm_transaction,");
    println!("  get_transactions, list_accounts");
    println!();
    println!("Press Ctrl+C for graceful shutdown");
    println!();

    // Run server
    transport::tcp::run_with_framing(config).await?;

    server::metrics::log_shutdown();
    
    Ok(())
}