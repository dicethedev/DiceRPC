use anyhow::{Context, bail};
use dice_rpc::*;
use std::sync::Arc;
use std::time::Duration;

/// Environment-configured HTTP server with authentication and resource limits.
///
/// Required:
/// API_KEYS=key-one,key-two cargo run --example production_http
///
/// Optional: HTTP_ADDR, MAX_BODY_BYTES, MAX_BATCH_SIZE, MAX_CONCURRENCY,
/// REQUEST_TIMEOUT_SECS.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    server::metrics::init_logging();

    let server = Arc::new(RpcServer::new());
    let state = Arc::new(state::StateStore::new());
    let metrics = Arc::new(Metrics::new());
    server::handlers::register_stateful_handlers(&server, state).await;

    let auth = Arc::new(middleware::AuthMiddleware::new(
        middleware::AuthStrategy::ApiKeyInHeader,
    ));
    let keys = std::env::var("API_KEYS")
        .context("API_KEYS must contain a comma-separated list of secrets")?;
    let mut key_count = 0usize;
    for key in keys.split(',').map(str::trim).filter(|key| !key.is_empty()) {
        key_count += usize::from(auth.add_key(key).await);
    }
    if key_count == 0 {
        bail!("API_KEYS must contain at least one non-empty key");
    }
    tracing::info!("Loaded {key_count} API key(s)");

    let metrics_reporter = metrics.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
            tracing::info!("Metrics: {:?}", metrics_reporter.snapshot().await);
        }
    });

    let addr = std::env::var("HTTP_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".to_string());
    let max_body_size = env_usize("MAX_BODY_BYTES", 1024 * 1024)?;
    let max_batch_size = env_usize("MAX_BATCH_SIZE", 100)?;
    let max_concurrency = env_usize("MAX_CONCURRENCY", 256)?;
    let timeout_secs = env_u64("REQUEST_TIMEOUT_SECS", 30)?;

    server::metrics::log_startup(&addr, "HTTP");
    transport::HttpTransport::new(server)
        .with_auth(auth)
        .with_metrics(metrics)
        .with_max_body_size(max_body_size)
        .with_max_batch_size(max_batch_size)
        .with_max_concurrency(max_concurrency)
        .with_request_timeout(Duration::from_secs(timeout_secs))
        .serve(&addr)
        .await
}

fn env_usize(name: &str, default: usize) -> anyhow::Result<usize> {
    match std::env::var(name) {
        Ok(value) => value
            .parse::<usize>()
            .with_context(|| format!("{name} must be a positive integer"))
            .map(|value| value.max(1)),
        Err(_) => Ok(default),
    }
}

fn env_u64(name: &str, default: u64) -> anyhow::Result<u64> {
    match std::env::var(name) {
        Ok(value) => value
            .parse::<u64>()
            .with_context(|| format!("{name} must be a positive integer"))
            .map(|value| value.max(1)),
        Err(_) => Ok(default),
    }
}
