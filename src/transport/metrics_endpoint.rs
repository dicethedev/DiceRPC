#![cfg(feature = "http")]

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::get,
    Router,
};
use std::sync::Arc;
use crate::server::metrics::Metrics;

/// Add metrics endpoint to HTTP server
pub fn metrics_router(metrics: Arc<Metrics>) -> Router {
    Router::new()
        .route("/metrics", get(get_metrics))
        .route("/health", get(health_check))
        .with_state(metrics)
}

/// GET /metrics - Returns current metrics
async fn get_metrics(
    State(metrics): State<Arc<Metrics>>,
) -> impl IntoResponse {
    let snapshot = metrics.snapshot().await;
    (StatusCode::OK, Json(snapshot))
}

/// GET /health - Health check endpoint
async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({
        "status": "healthy",
        "service": "DiceRPC"
    })))
}