
use dice_rpc::*;
use std::sync::Arc;


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    server::metrics::init_logging();

    // Create components
    let server = Arc::new(RpcServer::new());
    let state = Arc::new(state::StateStore::new());
    let metrics = Arc::new(dice_rpc::Metrics::new());

    // Register handlers
    dice_rpc::server::handlers::register_stateful_handlers(&server, state.clone()).await;

    // Setup authentication
    let auth = Arc::new(middleware::AuthMiddleware::new(
        middleware::AuthStrategy::ApiKeyInParams
    ));
    
    // Load API keys from environment
    if let Ok(keys) = std::env::var("API_KEYS") {
        for key in keys.split(',') {
            auth.add_key(key.trim()).await;
            tracing::info!("Loaded API key: {}...", &key[..8]);
        }
    } else {
        // Default development keys
        auth.add_key("dev-key-123").await;
    }

    // Spawn metrics reporter
    let metrics_clone = metrics.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            let snapshot = metrics_clone.snapshot().await;
            tracing::info!("Metrics: {:?}", snapshot);
        }
    });

    // Get configuration from environment
    let addr = std::env::var("HTTP_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:3000".to_string());

    server::metrics::log_startup(&addr, "HTTP");

    // Start HTTP server
    transport::HttpTransport::new(server)
        .with_auth(auth)
        .serve(&addr)
        .await?;

    Ok(())
}