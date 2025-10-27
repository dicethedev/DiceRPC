use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{info, warn, debug};

#[allow(dead_code)]
/// Metrics collector for RPC server
#[derive(Debug)]
pub struct Metrics {
    /// Total requests received
    total_requests: AtomicU64,
    /// Total successful responses
    total_success: AtomicU64,
    /// Total error responses
    total_errors: AtomicU64,
    /// Average request duration in microseconds
    avg_duration_us: Arc<RwLock<u64>>,
    /// Request counts per method
    method_counts: Arc<RwLock<std::collections::HashMap<String, u64>>>,
}

#[allow(dead_code)]
impl Metrics {
    pub fn new() -> Self {
        Self {
            total_requests: AtomicU64::new(0),
            total_success: AtomicU64::new(0),
            total_errors: AtomicU64::new(0),
            avg_duration_us: Arc::new(RwLock::new(0)),
            method_counts: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Record a request
    pub fn record_request(&self) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a successful response
    pub fn record_success(&self) {
        self.total_success.fetch_add(1, Ordering::Relaxed);
    }

    /// Record an error response
    pub fn record_error(&self) {
        self.total_errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Record request duration
    pub async fn record_duration(&self, duration: Duration) {
        let mut avg = self.avg_duration_us.write().await;
        let new_duration = duration.as_micros() as u64;
        
        // Simple moving average
        *avg = if *avg == 0 {
            new_duration
        } else {
            (*avg * 9 + new_duration) / 10
        };
    }

    /// Record method call
    pub async fn record_method(&self, method: &str) {
        let mut counts = self.method_counts.write().await;
        *counts.entry(method.to_string()).or_insert(0) += 1;
    }

    /// Get current metrics snapshot
    pub async fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            total_requests: self.total_requests.load(Ordering::Relaxed),
            total_success: self.total_success.load(Ordering::Relaxed),
            total_errors: self.total_errors.load(Ordering::Relaxed),
            avg_duration_us: *self.avg_duration_us.read().await,
            method_counts: self.method_counts.read().await.clone(),
        }
    }

    /// Reset all metrics
    pub async fn reset(&self) {
        self.total_requests.store(0, Ordering::Relaxed);
        self.total_success.store(0, Ordering::Relaxed);
        self.total_errors.store(0, Ordering::Relaxed);
        *self.avg_duration_us.write().await = 0;
        self.method_counts.write().await.clear();
    }
}

#[allow(dead_code)]
impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Snapshot of metrics at a point in time
#[derive(Debug, Clone, serde::Serialize)]
pub struct MetricsSnapshot {
    pub total_requests: u64,
    pub total_success: u64,
    pub total_errors: u64,
    pub avg_duration_us: u64,
    pub method_counts: std::collections::HashMap<String, u64>,
}

#[allow(dead_code)]
/// Request tracer for logging and timing
pub struct RequestTracer {
    method: String,
    start: Instant,
    metrics: Arc<Metrics>,
}

#[allow(dead_code)]
impl RequestTracer {
    pub fn new(method: impl Into<String>, metrics: Arc<Metrics>) -> Self {
        let method = method.into();
        debug!("Starting request: {}", method);
        metrics.record_request();
        
        Self {
            method,
            start: Instant::now(),
            metrics,
        }
    }

    /// Record successful completion
    pub async fn success(self) {
        let duration = self.start.elapsed();
        info!(
            "Request completed: {} ({}ms)",
            self.method,
            duration.as_millis()
        );
        
        self.metrics.record_success();
        self.metrics.record_duration(duration).await;
        self.metrics.record_method(&self.method).await;
    }

    /// Record error completion
    pub async fn error(self, error: &str) {
        let duration = self.start.elapsed();
        warn!(
            "Request failed: {} - {} ({}ms)",
            self.method,
            error,
            duration.as_millis()
        );
        
        self.metrics.record_error();
        self.metrics.record_duration(duration).await;
        self.metrics.record_method(&self.method).await;
    }
}

#[allow(dead_code)]
/// Initialize logging with tracing
pub fn init_logging() {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "dice_rpc=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}

#[allow(dead_code)]
/// Log server startup
pub fn log_startup(addr: &str, transport: &str) {
    info!("╔══════════════════════════════════════╗");
    info!("║       DiceRPC Server Started         ║");
    info!("╚══════════════════════════════════════╝");
    info!("Transport: {}", transport);
    info!("Address: {}", addr);
    info!("Ready to accept connections");
}

#[allow(dead_code)]
/// Log server shutdown
pub fn log_shutdown() {
    info!("╔══════════════════════════════════════╗");
    info!("║     DiceRPC Server Shutting Down     ║");
    info!("╚══════════════════════════════════════╝");
}
