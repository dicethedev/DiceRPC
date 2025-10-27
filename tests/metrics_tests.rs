use tokio::time::Duration;

#[tokio::test]
async fn test_metrics_recording() {
    let metrics = dice_rpc::Metrics::new();

    metrics.record_request();
    metrics.record_request();
    metrics.record_success();

    let snapshot = metrics.snapshot().await;
    assert_eq!(snapshot.total_requests, 2);
    assert_eq!(snapshot.total_success, 1);
    assert_eq!(snapshot.total_errors, 0);
}

#[tokio::test]
async fn test_method_counts() {
    let metrics = dice_rpc::Metrics::new();

    metrics.record_method("ping").await;
    metrics.record_method("ping").await;
    metrics.record_method("get_balance").await;

    let snapshot = metrics.snapshot().await;
    assert_eq!(snapshot.method_counts.get("ping"), Some(&2));
    assert_eq!(snapshot.method_counts.get("get_balance"), Some(&1));
}

#[tokio::test]
async fn test_duration_recording() {
    let metrics = dice_rpc::Metrics::new();

    metrics.record_duration(Duration::from_millis(100)).await;

    let snapshot = metrics.snapshot().await;
    assert!(snapshot.avg_duration_us > 0);
}
