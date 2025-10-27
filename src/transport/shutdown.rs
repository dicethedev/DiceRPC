use tokio::signal;
use tokio::sync::broadcast;
use tracing::{info, warn};

/// Graceful shutdown coordinator
pub struct ShutdownCoordinator {
    tx: broadcast::Sender<()>,
}


/// Example server with graceful shutdown
/// 
/// ```rust
/// use dice_rpc::shutdown::*;
/// use std::time::Duration;
/// 
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let coordinator = ShutdownCoordinator::new();
///     let shutdown_rx = coordinator.subscribe();
///     
///     // Spawn signal handler
///     tokio::spawn(async move {
///         coordinator.wait_for_signal().await;
///     });
///     
///     // Run server
///     tokio::select! {
///         result = run_server() => {
///             result?;
///         }
///         _ = wait_for_shutdown(shutdown_rx) => {
///             println!("Shutting down gracefully...");
///         }
///     }
///     
///     // Cleanup
///     perform_cleanup().await;
///     
///     Ok(())
/// }
/// ```

impl ShutdownCoordinator {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(1);
        Self { tx }
    }

    /// Subscribe to shutdown signal
    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        self.tx.subscribe()
    }

    /// Trigger shutdown
    pub fn shutdown(&self) {
        let _ = self.tx.send(());
    }

    /// Wait for OS shutdown signals (CTRL+C, SIGTERM)
    pub async fn wait_for_signal(&self) {
        #[cfg(unix)]
        {
            use signal::unix::{signal, SignalKind};
            
            let mut sigterm = signal(SignalKind::terminate())
                .expect("Failed to register SIGTERM handler");
            let mut sigint = signal(SignalKind::interrupt())
                .expect("Failed to register SIGINT handler");

            tokio::select! {
                _ = sigterm.recv() => {
                    info!("Received SIGTERM");
                }
                _ = sigint.recv() => {
                    info!("Received SIGINT (CTRL+C)");
                }
            }
        }

        #[cfg(not(unix))]
        {
            signal::ctrl_c()
                .await
                .expect("Failed to listen for CTRL+C");
            info!("Received CTRL+C");
        }

        info!("Initiating graceful shutdown...");
        self.shutdown();
    }
}

impl Default for ShutdownCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to create a future that completes when shutdown is triggered
pub async fn wait_for_shutdown(mut rx: broadcast::Receiver<()>) {
    let _ = rx.recv().await;
}

/// Graceful shutdown with timeout
pub async fn shutdown_with_timeout<F>(
    shutdown_rx: broadcast::Receiver<()>,
    cleanup: F,
    timeout: std::time::Duration,
) where
    F: std::future::Future<Output = ()>,
{
    // Wait for shutdown signal
    wait_for_shutdown(shutdown_rx).await;
    
    info!("Running cleanup tasks...");
    
    // Run cleanup with timeout
    match tokio::time::timeout(timeout, cleanup).await {
        Ok(_) => {
            info!("Cleanup completed successfully");
        }
        Err(_) => {
            warn!("Cleanup timed out after {:?}", timeout);
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_shutdown_coordination() {
        let coordinator = ShutdownCoordinator::new();
        let mut rx = coordinator.subscribe();
        
        // Trigger shutdown
        coordinator.shutdown();
        
        // Should receive signal
        assert!(rx.try_recv().is_ok());
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let coordinator = ShutdownCoordinator::new();
        let mut rx1 = coordinator.subscribe();
        let mut rx2 = coordinator.subscribe();
        
        coordinator.shutdown();
        
        assert!(rx1.try_recv().is_ok());
        assert!(rx2.try_recv().is_ok());
    }

    #[tokio::test]
    async fn test_shutdown_with_timeout() {
        let coordinator = ShutdownCoordinator::new();
        let rx = coordinator.subscribe();
        
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            coordinator.shutdown();
        });
        
        shutdown_with_timeout(
            rx,
            async {
                // Simulate cleanup
                tokio::time::sleep(Duration::from_millis(50)).await;
            },
            Duration::from_secs(1),
        )
        .await;
    }
}