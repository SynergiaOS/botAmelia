use anyhow::Result;
use cerberus::monitoring::*;
use serde_json::json;
use std::time::Duration;
use tokio::test;

/// Unit tests for monitoring functionality

#[test]
fn test_system_metrics_creation() {
    let metrics = SystemMetrics::new();
    
    assert!(metrics.cpu_usage >= 0.0);
    assert!(metrics.cpu_usage <= 100.0);
    assert!(metrics.memory_usage_mb > 0);
    assert!(metrics.memory_total_mb > 0);
    assert!(metrics.disk_usage_mb >= 0);
    assert!(metrics.network_rx_bytes >= 0);
    assert!(metrics.network_tx_bytes >= 0);
    assert!(metrics.timestamp > 0);
}

#[test]
fn test_system_metrics_memory_percentage() {
    let metrics = SystemMetrics::new();
    let memory_percent = metrics.memory_usage_percentage();
    
    assert!(memory_percent >= 0.0);
    assert!(memory_percent <= 100.0);
}

#[test]
fn test_system_metrics_serialization() -> Result<()> {
    let metrics = SystemMetrics::new();
    
    // Test JSON serialization
    let json = serde_json::to_string(&metrics)?;
    assert!(!json.is_empty());
    
    // Test deserialization
    let deserialized: SystemMetrics = serde_json::from_str(&json)?;
    assert_eq!(metrics.cpu_usage, deserialized.cpu_usage);
    assert_eq!(metrics.memory_usage_mb, deserialized.memory_usage_mb);
    assert_eq!(metrics.timestamp, deserialized.timestamp);
    
    Ok(())
}

#[test]
fn test_health_status_ordering() {
    assert!(HealthStatus::Healthy > HealthStatus::Warning);
    assert!(HealthStatus::Warning > HealthStatus::Critical);
    assert!(HealthStatus::Critical > HealthStatus::Down);
    
    // Test equality
    assert_eq!(HealthStatus::Healthy, HealthStatus::Healthy);
    assert_ne!(HealthStatus::Healthy, HealthStatus::Warning);
}

#[test]
fn test_health_report_creation() {
    let report = HealthReport::new(
        HealthStatus::Healthy,
        "All systems operational".to_string(),
    );
    
    assert_eq!(report.status, HealthStatus::Healthy);
    assert_eq!(report.message, "All systems operational");
    assert!(report.timestamp > 0);
    assert!(report.checks.is_empty());
}

#[test]
fn test_health_report_with_checks() {
    let mut report = HealthReport::new(
        HealthStatus::Warning,
        "Some issues detected".to_string(),
    );
    
    report.add_check("database", HealthStatus::Healthy, "Database connection OK");
    report.add_check("cache", HealthStatus::Warning, "Cache hit rate low");
    report.add_check("api", HealthStatus::Healthy, "API responding normally");
    
    assert_eq!(report.checks.len(), 3);
    assert!(report.checks.contains_key("database"));
    assert!(report.checks.contains_key("cache"));
    assert!(report.checks.contains_key("api"));
    
    assert_eq!(report.checks["database"].status, HealthStatus::Healthy);
    assert_eq!(report.checks["cache"].status, HealthStatus::Warning);
}

#[test]
fn test_health_check_creation() {
    let check = HealthCheck::new(
        HealthStatus::Critical,
        "Service unavailable".to_string(),
    );
    
    assert_eq!(check.status, HealthStatus::Critical);
    assert_eq!(check.message, "Service unavailable");
    assert!(check.timestamp > 0);
    assert!(check.metadata.is_null());
}

#[test]
fn test_health_check_with_metadata() {
    let metadata = json!({
        "response_time": 150,
        "error_rate": 0.05,
        "last_error": "Connection timeout"
    });
    
    let check = HealthCheck::new(
        HealthStatus::Warning,
        "Performance degraded".to_string(),
    ).with_metadata(metadata.clone());
    
    assert_eq!(check.metadata, metadata);
}

#[tokio::test]
async fn test_health_monitor_creation() {
    let monitor = HealthMonitor::new(Duration::from_secs(30));
    
    // Initial health check should be healthy
    let report = monitor.get_health_report().await;
    assert_eq!(report.status, HealthStatus::Healthy);
}

#[tokio::test]
async fn test_health_monitor_check_registration() {
    let monitor = HealthMonitor::new(Duration::from_secs(30));
    
    // Register a health check
    monitor.register_check(
        "test_service".to_string(),
        Box::new(MockHealthChecker::new(HealthStatus::Healthy)),
    ).await;
    
    // Run health checks
    let report = monitor.run_health_checks().await;
    assert!(report.checks.contains_key("test_service"));
    assert_eq!(report.checks["test_service"].status, HealthStatus::Healthy);
}

#[tokio::test]
async fn test_health_monitor_multiple_checks() {
    let monitor = HealthMonitor::new(Duration::from_secs(30));
    
    // Register multiple health checks
    monitor.register_check(
        "database".to_string(),
        Box::new(MockHealthChecker::new(HealthStatus::Healthy)),
    ).await;
    
    monitor.register_check(
        "cache".to_string(),
        Box::new(MockHealthChecker::new(HealthStatus::Warning)),
    ).await;
    
    monitor.register_check(
        "api".to_string(),
        Box::new(MockHealthChecker::new(HealthStatus::Critical)),
    ).await;
    
    // Run health checks
    let report = monitor.run_health_checks().await;
    
    assert_eq!(report.checks.len(), 3);
    assert_eq!(report.checks["database"].status, HealthStatus::Healthy);
    assert_eq!(report.checks["cache"].status, HealthStatus::Warning);
    assert_eq!(report.checks["api"].status, HealthStatus::Critical);
    
    // Overall status should be the worst status (Critical)
    assert_eq!(report.status, HealthStatus::Critical);
}

#[tokio::test]
async fn test_health_monitor_automatic_checks() {
    let monitor = HealthMonitor::new(Duration::from_millis(100)); // Very short interval for testing
    
    monitor.register_check(
        "test".to_string(),
        Box::new(MockHealthChecker::new(HealthStatus::Healthy)),
    ).await;
    
    // Start automatic health checks
    monitor.start_automatic_checks().await;
    
    // Wait a bit for checks to run
    tokio::time::sleep(Duration::from_millis(200)).await;
    
    // Stop automatic checks
    monitor.stop_automatic_checks().await;
    
    let report = monitor.get_health_report().await;
    assert!(report.checks.contains_key("test"));
}

#[tokio::test]
async fn test_metrics_collector_creation() {
    let collector = MetricsCollector::new(Duration::from_secs(60));
    
    // Should start with no metrics
    let metrics = collector.get_latest_metrics().await;
    assert!(metrics.is_none());
}

#[tokio::test]
async fn test_metrics_collector_collection() {
    let collector = MetricsCollector::new(Duration::from_secs(60));
    
    // Collect metrics
    collector.collect_metrics().await;
    
    // Should now have metrics
    let metrics = collector.get_latest_metrics().await;
    assert!(metrics.is_some());
    
    let metrics = metrics.unwrap();
    assert!(metrics.cpu_usage >= 0.0);
    assert!(metrics.memory_usage_mb > 0);
}

#[tokio::test]
async fn test_metrics_collector_history() {
    let collector = MetricsCollector::new(Duration::from_secs(60));
    
    // Collect multiple metrics
    for _ in 0..5 {
        collector.collect_metrics().await;
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    
    // Get metrics history
    let history = collector.get_metrics_history(10).await;
    assert_eq!(history.len(), 5);
    
    // Metrics should be in chronological order (newest first)
    for i in 1..history.len() {
        assert!(history[i-1].timestamp >= history[i].timestamp);
    }
}

#[tokio::test]
async fn test_metrics_collector_automatic_collection() {
    let collector = MetricsCollector::new(Duration::from_millis(50)); // Very short interval
    
    // Start automatic collection
    collector.start_automatic_collection().await;
    
    // Wait for some metrics to be collected
    tokio::time::sleep(Duration::from_millis(150)).await;
    
    // Stop automatic collection
    collector.stop_automatic_collection().await;
    
    // Should have collected multiple metrics
    let history = collector.get_metrics_history(10).await;
    assert!(history.len() >= 2);
}

#[test]
fn test_performance_metrics_creation() {
    let metrics = PerformanceMetrics::new();
    
    assert_eq!(metrics.requests_per_second, 0.0);
    assert_eq!(metrics.average_response_time_ms, 0.0);
    assert_eq!(metrics.error_rate, 0.0);
    assert_eq!(metrics.active_connections, 0);
    assert!(metrics.timestamp > 0);
}

#[test]
fn test_performance_metrics_update() {
    let mut metrics = PerformanceMetrics::new();
    
    metrics.update_request_rate(100.0);
    metrics.update_response_time(50.0);
    metrics.update_error_rate(0.02);
    metrics.update_connections(25);
    
    assert_eq!(metrics.requests_per_second, 100.0);
    assert_eq!(metrics.average_response_time_ms, 50.0);
    assert_eq!(metrics.error_rate, 0.02);
    assert_eq!(metrics.active_connections, 25);
}

#[test]
fn test_alert_creation() {
    let alert = Alert::new(
        AlertLevel::Warning,
        "High memory usage".to_string(),
        "Memory usage is above 80%".to_string(),
    );
    
    assert_eq!(alert.level, AlertLevel::Warning);
    assert_eq!(alert.title, "High memory usage");
    assert_eq!(alert.message, "Memory usage is above 80%");
    assert!(alert.timestamp > 0);
    assert!(!alert.id.is_empty());
}

#[test]
fn test_alert_levels() {
    assert!(AlertLevel::Critical > AlertLevel::Warning);
    assert!(AlertLevel::Warning > AlertLevel::Info);
    
    let levels = vec![
        AlertLevel::Info,
        AlertLevel::Warning,
        AlertLevel::Critical,
    ];
    
    for level in levels {
        let alert = Alert::new(
            level.clone(),
            "Test alert".to_string(),
            "Test message".to_string(),
        );
        assert_eq!(alert.level, level);
    }
}

#[tokio::test]
async fn test_alert_manager_creation() {
    let manager = AlertManager::new();
    
    // Should start with no alerts
    let alerts = manager.get_recent_alerts(10).await;
    assert_eq!(alerts.len(), 0);
}

#[tokio::test]
async fn test_alert_manager_alert_creation() {
    let manager = AlertManager::new();
    
    manager.create_alert(
        AlertLevel::Warning,
        "Test Alert".to_string(),
        "This is a test alert".to_string(),
    ).await;
    
    let alerts = manager.get_recent_alerts(10).await;
    assert_eq!(alerts.len(), 1);
    assert_eq!(alerts[0].title, "Test Alert");
    assert_eq!(alerts[0].level, AlertLevel::Warning);
}

#[tokio::test]
async fn test_alert_manager_filtering() {
    let manager = AlertManager::new();
    
    // Create alerts of different levels
    manager.create_alert(
        AlertLevel::Info,
        "Info Alert".to_string(),
        "Information".to_string(),
    ).await;
    
    manager.create_alert(
        AlertLevel::Warning,
        "Warning Alert".to_string(),
        "Warning message".to_string(),
    ).await;
    
    manager.create_alert(
        AlertLevel::Critical,
        "Critical Alert".to_string(),
        "Critical issue".to_string(),
    ).await;
    
    // Test filtering by level
    let warning_alerts = manager.get_alerts_by_level(AlertLevel::Warning).await;
    assert_eq!(warning_alerts.len(), 1);
    assert_eq!(warning_alerts[0].title, "Warning Alert");
    
    let critical_alerts = manager.get_alerts_by_level(AlertLevel::Critical).await;
    assert_eq!(critical_alerts.len(), 1);
    assert_eq!(critical_alerts[0].title, "Critical Alert");
}

// Mock health checker for testing
struct MockHealthChecker {
    status: HealthStatus,
}

impl MockHealthChecker {
    fn new(status: HealthStatus) -> Self {
        Self { status }
    }
}

#[async_trait::async_trait]
impl HealthChecker for MockHealthChecker {
    async fn check_health(&self) -> HealthCheck {
        HealthCheck::new(
            self.status.clone(),
            format!("Mock health check: {:?}", self.status),
        )
    }
    
    fn name(&self) -> &str {
        "mock_checker"
    }
}

#[test]
fn test_mock_health_checker() {
    let checker = MockHealthChecker::new(HealthStatus::Healthy);
    assert_eq!(checker.name(), "mock_checker");
}

#[tokio::test]
async fn test_mock_health_checker_check() {
    let checker = MockHealthChecker::new(HealthStatus::Warning);
    let check = checker.check_health().await;
    
    assert_eq!(check.status, HealthStatus::Warning);
    assert!(check.message.contains("Mock health check"));
}

#[test]
fn test_monitoring_serialization() -> Result<()> {
    let metrics = SystemMetrics::new();
    let health_check = HealthCheck::new(
        HealthStatus::Healthy,
        "All good".to_string(),
    );
    let alert = Alert::new(
        AlertLevel::Info,
        "Test".to_string(),
        "Test message".to_string(),
    );
    
    // Test serialization of all monitoring types
    let metrics_json = serde_json::to_string(&metrics)?;
    let check_json = serde_json::to_string(&health_check)?;
    let alert_json = serde_json::to_string(&alert)?;
    
    assert!(!metrics_json.is_empty());
    assert!(!check_json.is_empty());
    assert!(!alert_json.is_empty());
    
    // Test deserialization
    let _: SystemMetrics = serde_json::from_str(&metrics_json)?;
    let _: HealthCheck = serde_json::from_str(&check_json)?;
    let _: Alert = serde_json::from_str(&alert_json)?;
    
    Ok(())
}
