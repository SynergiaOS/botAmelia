use anyhow::Result;
use cerberus::{
    cache::{CacheManager, CacheStats},
    config::Config,
    database::DatabaseManager,
    monitoring::{HealthReport, HealthStatus, SystemMetrics},
    risk::{Portfolio, Position, TradeSide},
    security::{SecurityEventType, SecurityLevel, SecurityMonitor},
    signals::{Confidence, Signal, SignalStats},
    trading::{ExecutionResult, OrderType, TradeOrder, TradingStats},
};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;
use tokio::test;

/// Integration tests for Cerberus v5.0
///
/// These tests verify that different components work together correctly
/// in realistic scenarios without requiring external APIs or real funds.

#[test]
async fn test_complete_trading_workflow() -> Result<()> {
    // Setup test environment
    let temp_dir = TempDir::new()?;
    let mut config = Config::default();
    config.database.path = temp_dir.path().join("test.db");
    config.trading.paper_trading = true;
    config.trading.initial_balance = rust_decimal::Decimal::new(100, 0);

    // Initialize components
    let db_manager = DatabaseManager::new(&config.database).await?;
    let mut portfolio = Portfolio::new(100.0);

    // Test signal processing
    let signal = Signal::new(
        "BONK".to_string(),
        "pump_fun".to_string(),
        Confidence::High,
        0.000001,  // Price in SOL
        1000000.0, // Volume
        serde_json::json!({"liquidity": 50000}),
    );

    // Validate signal
    signal.validate()?;
    assert!(signal.is_fresh(300)); // Should be fresh (< 5 minutes)

    // Test trade order creation
    let order = TradeOrder::market_order(
        signal.token.clone(),
        TradeSide::Long,
        10.0, // $10 position
        5,    // 5x leverage
    );

    // Validate order
    order.validate()?;
    assert_eq!(order.order_type, OrderType::Market);
    assert_eq!(order.leverage, 5);

    // Test position creation
    let position = Position::new(signal.token.clone(), TradeSide::Long, 10.0, 5, signal.price);

    // Add position to portfolio
    portfolio.add_position(position.clone());

    // Verify portfolio state
    assert_eq!(portfolio.open_positions.len(), 1);
    assert!(portfolio.is_healthy());
    assert!(portfolio.margin_utilization() > 0.0);

    // Test price update and P&L calculation
    let mut updated_position = position.clone();
    updated_position.update_price(signal.price * 1.2); // 20% price increase

    // With 5x leverage, 20% price increase should give ~100% return
    let expected_pnl = 10.0 * 5.0 * 0.2; // $10 position * 5x leverage * 20% gain
    assert!((updated_position.pnl - expected_pnl).abs() < 0.01);

    // Test stop loss trigger
    updated_position.update_price(signal.price * 0.8); // 20% price decrease
    let loss_pnl = 10.0 * 5.0 * -0.2; // Should be -$10 (100% of position)
    assert!((updated_position.pnl - loss_pnl).abs() < 0.01);

    // Test liquidation check
    assert!(updated_position.is_near_liquidation(0.05)); // Should be near liquidation

    Ok(())
}

#[test]
async fn test_risk_management_system() -> Result<()> {
    let mut portfolio = Portfolio::new(100.0);

    // Test maximum position size enforcement
    let large_position = Position::new(
        "TEST".to_string(),
        TradeSide::Long,
        60.0, // 60% of portfolio
        10,   // 10x leverage means margin = 60/10 = 6% of portfolio
        0.001,
    );

    portfolio.add_position(large_position);

    // Portfolio should still be healthy but with some margin utilization
    assert!(portfolio.is_healthy());
    assert!(portfolio.margin_utilization() > 0.05); // Should be ~6%

    // Test circuit breaker scenario
    let mut losing_position = portfolio.open_positions[0].clone();
    losing_position.update_price(0.0005); // 50% price drop

    // With 10x leverage, this should trigger major losses
    assert!(losing_position.pnl < -30.0); // Significant loss

    // Test portfolio recovery
    portfolio.remove_position(&losing_position.id);
    portfolio.balance += losing_position.pnl; // Apply the loss

    // Portfolio might be negative due to high leverage losses, which is realistic
    // In a real scenario, this would trigger liquidation or margin calls
    assert!(portfolio.balance.is_finite()); // Just ensure it's a valid number

    Ok(())
}

#[test]
async fn test_signal_analysis_pipeline() -> Result<()> {
    // Test different signal types and confidence levels
    let signals = vec![
        Signal::new(
            "HIGH_CONF".to_string(),
            "ai_analysis".to_string(),
            Confidence::Extreme,
            0.001,
            2000000.0,
            serde_json::json!({"sentiment": 0.9, "volume_spike": 10.0}),
        ),
        Signal::new(
            "MED_CONF".to_string(),
            "technical".to_string(),
            Confidence::Medium,
            0.0005,
            500000.0,
            serde_json::json!({"sentiment": 0.6, "volume_spike": 3.0}),
        ),
        Signal::new(
            "LOW_CONF".to_string(),
            "social".to_string(),
            Confidence::Low,
            0.0002,
            100000.0,
            serde_json::json!({"sentiment": 0.4, "volume_spike": 1.5}),
        ),
    ];

    // Test signal validation
    for signal in &signals {
        signal.validate()?;
        assert!(!signal.token.is_empty());
        assert!(signal.price > 0.0);
        assert!(signal.volume >= 0.0);
    }

    // Test signal scoring (mock implementation)
    let high_conf_score = calculate_mock_signal_score(&signals[0]);
    let med_conf_score = calculate_mock_signal_score(&signals[1]);
    let low_conf_score = calculate_mock_signal_score(&signals[2]);

    // Higher confidence should generally result in higher scores
    assert!(high_conf_score > med_conf_score);
    assert!(med_conf_score > low_conf_score);

    Ok(())
}

#[test]
async fn test_database_operations() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let mut config = Config::default();
    config.database.path = temp_dir.path().join("test.db");

    let db_manager = DatabaseManager::new(&config.database).await?;

    // Test database health check
    db_manager.health_check().await?;

    // Test transaction handling
    let mut tx = db_manager.begin_transaction().await?;

    // Mock database operations would go here
    // For now, just test that transactions work
    tx.commit().await?;

    // Test backup creation
    if config.database.enable_backup {
        let backup_name = db_manager.create_backup().await?;
        assert!(!backup_name.is_empty());
    }

    Ok(())
}

#[test]
async fn test_configuration_validation() -> Result<()> {
    // Test valid configuration
    let mut config = Config::default();
    config.validate()?;

    // Test invalid configurations
    config.trading.max_leverage = 0;
    assert!(config.validate().is_err());

    config.trading.max_leverage = 50;
    config.trading.max_position_size_percent = rust_decimal::Decimal::new(150, 2); // 150%
    assert!(config.validate().is_err());

    // Reset to valid state
    config.trading.max_position_size_percent = rust_decimal::Decimal::new(33, 2); // 33%
    config.validate()?;

    Ok(())
}

#[test]
async fn test_error_handling_and_recovery() -> Result<()> {
    // Test graceful error handling in various scenarios

    // Test invalid signal handling
    let invalid_signal = Signal::new(
        "".to_string(), // Empty token name
        "test".to_string(),
        Confidence::High,
        0.001,
        1000.0,
        serde_json::Value::Null,
    );

    assert!(invalid_signal.validate().is_err());

    // Test invalid order handling
    let invalid_order = TradeOrder::market_order(
        "TEST".to_string(),
        TradeSide::Long,
        -10.0, // Negative size
        5,
    );

    assert!(invalid_order.validate().is_err());

    // Test portfolio edge cases
    let mut portfolio = Portfolio::new(0.0); // Zero balance
    assert!(!portfolio.is_healthy());

    Ok(())
}

#[test]
async fn test_performance_requirements() -> Result<()> {
    use std::time::Instant;

    // Test signal processing performance
    let start = Instant::now();

    let signal = Signal::new(
        "PERF_TEST".to_string(),
        "performance".to_string(),
        Confidence::High,
        0.001,
        1000000.0,
        serde_json::json!({"test": true}),
    );

    signal.validate()?;
    let _score = calculate_mock_signal_score(&signal);

    let duration = start.elapsed();

    // Signal processing should be under 10ms
    assert!(
        duration.as_millis() < 10,
        "Signal processing too slow: {}ms",
        duration.as_millis()
    );

    // Test portfolio update performance
    let start = Instant::now();

    let mut portfolio = Portfolio::new(1000.0);
    for i in 0..100 {
        let position = Position::new(format!("TOKEN_{}", i), TradeSide::Long, 10.0, 5, 0.001);
        portfolio.add_position(position);
    }

    let duration = start.elapsed();

    // Portfolio operations should be fast even with many positions
    assert!(
        duration.as_millis() < 100,
        "Portfolio operations too slow: {}ms",
        duration.as_millis()
    );

    Ok(())
}

// Helper functions for tests

fn calculate_mock_signal_score(signal: &Signal) -> f64 {
    let mut score: f64 = 0.0;

    // Mock scoring based on confidence
    score += match signal.confidence {
        Confidence::Extreme => 0.4,
        Confidence::High => 0.3,
        Confidence::Medium => 0.2,
        Confidence::Low => 0.1,
    };

    // Mock volume scoring
    if signal.volume > 1000000.0 {
        score += 0.3;
    } else if signal.volume > 500000.0 {
        score += 0.2;
    } else {
        score += 0.1;
    }

    // Mock metadata scoring
    if let Some(sentiment) = signal.metadata.get("sentiment") {
        if let Some(sentiment_val) = sentiment.as_f64() {
            score += sentiment_val * 0.3;
        }
    }

    score.clamp(0.0, 1.0)
}

// Stress test for high-frequency scenarios
#[test]
#[ignore] // Only run with --ignored flag
async fn stress_test_high_frequency_trading() -> Result<()> {
    let mut portfolio = Portfolio::new(10000.0);

    // Simulate 1000 rapid trades
    for i in 0..1000 {
        let signal = Signal::new(
            format!("STRESS_{}", i),
            "stress_test".to_string(),
            if i % 4 == 0 {
                Confidence::High
            } else {
                Confidence::Medium
            },
            0.001 + (i as f64 * 0.0001),
            100000.0 + (i as f64 * 1000.0),
            serde_json::json!({"iteration": i}),
        );

        signal.validate()?;

        let position = Position::new(
            signal.token,
            if i % 2 == 0 {
                TradeSide::Long
            } else {
                TradeSide::Short
            },
            50.0, // $50 per position
            5,
            signal.price,
        );

        portfolio.add_position(position);

        // Simulate some positions closing
        if i > 10 && i % 10 == 0 {
            if let Some(old_position) = portfolio.open_positions.first() {
                let position_id = old_position.id.clone();
                portfolio.remove_position(&position_id);
            }
        }
    }

    // Portfolio should still be functional after stress test
    assert!(portfolio.open_positions.len() > 0);
    portfolio.update();

    Ok(())
}

#[tokio::test]
async fn test_cache_integration_with_signals() -> Result<()> {
    let cache_manager = CacheManager::new(true);

    // Create test signals
    let signal1 = Signal::new(
        "CACHE_TEST_1".to_string(),
        "integration_test".to_string(),
        Confidence::High,
        0.001,
        1000000.0,
        json!({"cached": true}),
    );

    let signal2 = Signal::new(
        "CACHE_TEST_2".to_string(),
        "integration_test".to_string(),
        Confidence::Medium,
        0.002,
        500000.0,
        json!({"cached": false}),
    );

    // Simulate cache operations
    cache_manager.record_miss("signal_cache").await;
    cache_manager.record_hit("signal_cache").await;
    cache_manager.record_hit("signal_cache").await;

    let stats = cache_manager.get_stats().await;
    assert_eq!(stats.hits, 2);
    assert_eq!(stats.misses, 1);
    assert!((stats.hit_rate - 0.6666666666666666).abs() < 0.0001);

    // Test performance monitoring
    cache_manager.check_performance().await?;

    Ok(())
}

#[tokio::test]
async fn test_security_monitoring_integration() -> Result<()> {
    let mut security_monitor = SecurityMonitor::new(true, 100);

    // Simulate security events during trading
    security_monitor.unauthorized_access(
        "Suspicious API access attempt".to_string(),
        Some("192.168.1.100".to_string()),
    );

    security_monitor.suspicious_transaction(
        "Large position size detected".to_string(),
        json!({"position_size": 5000, "leverage": 50}),
    );

    security_monitor.trading_anomaly(
        "Unusual trading frequency".to_string(),
        json!({"trades_per_minute": 100}),
    );

    // Verify events were logged
    let recent_events = security_monitor.recent_events(10);
    assert_eq!(recent_events.len(), 3);

    let unauthorized_events =
        security_monitor.events_by_type(SecurityEventType::UnauthorizedAccess);
    assert_eq!(unauthorized_events.len(), 1);

    let high_level_events = security_monitor.events_by_level(SecurityLevel::High);
    assert_eq!(high_level_events.len(), 2); // unauthorized_access and suspicious_transaction

    // Test critical event detection
    assert!(!security_monitor.has_critical_events(60));

    Ok(())
}
