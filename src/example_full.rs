/// Complete example using all DiceRPC features
/// 
/// This demonstrates:
/// - Length-prefixed framing
/// - Batch requests
/// - Authentication
/// - HTTP and TCP transports
/// - Persistent state
/// - Logging and metrics
/// - Graceful shutdown

use dice_rpc::*;
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    metrics::init_logging();

    // Create server and state
    let server = Arc::new(rpc::RpcServer::new());
    let state = Arc::new(state::StateStore::new());
    let metrics = Arc::new(metrics::Metrics::new());

    // Register handlers with state
    handlers::register_stateful_handlers(&server, state.clone()).await;

    // Setup authentication
    let auth = Arc::new(auth::AuthMiddleware::new(auth::AuthStrategy::ApiKeyInParams));
    auth.add_key("dev-key-12345").await;
    auth.add_key("prod-key-67890").await;

    // Setup graceful shutdown
    let shutdown = Arc::new(shutdown::ShutdownCoordinator::new());
    let shutdown_clone = shutdown.clone();
    
    // Spawn signal handler
    tokio::spawn(async move {
        shutdown_clone.wait_for_signal().await;
    });

    // Spawn metrics reporter
    let metrics_clone = metrics.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(30)).await;
            let snapshot = metrics_clone.snapshot().await;
            tracing::info!("Metrics: {:?}", snapshot);
        }
    });

    // Choose transport
    let transport_mode = std::env::var("TRANSPORT").unwrap_or_else(|_| "tcp".to_string());

    match transport_mode.as_str() {
        "http" => {
            // HTTP transport
            tracing::info!("Starting HTTP transport");
            let http_transport = http_transport::HttpTransport::new(server.clone())
                .with_auth(auth.clone());
            
            tokio::select! {
                result = http_transport.serve("127.0.0.1:3000") => {
                    result?;
                }
                _ = shutdown::wait_for_shutdown(shutdown.subscribe()) => {
                    tracing::info!("Shutting down HTTP server");
                }
            }
        }
        "tcp" | _ => {
            // TCP transport with framing
            tracing::info!("Starting TCP transport (framed)");
            let tcp_config = server_enhanced::TcpServerConfig::new("127.0.0.1:4000", server.clone())
                .with_auth(auth.clone())
                .with_metrics(metrics.clone());
            
            tokio::select! {
                result = server_enhanced::run_with_framing(tcp_config) => {
                    result?;
                }
                _ = shutdown::wait_for_shutdown(shutdown.subscribe()) => {
                    tracing::info!("Shutting down TCP server");
                }
            }
        }
    }

    // Cleanup
    tracing::info!("Performing cleanup...");
    let final_metrics = metrics.snapshot().await;
    tracing::info!("Final metrics: {:?}", final_metrics);

    metrics::log_shutdown();

    Ok(())
}

// Example client usage with new features
