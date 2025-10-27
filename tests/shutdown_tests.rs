
use dice_rpc::transport::shutdown::ShutdownCoordinator;
use dice_rpc::transport::shutdown::shutdown_with_timeout;
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