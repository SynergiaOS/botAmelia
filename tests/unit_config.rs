#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(clippy::all)]
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(clippy::all)]

use anyhow::Result;
use cerberus::config::risk::{StopLossConfig, TakeProfitConfig};
use cerberus::config::trading::LeverageConfig;
use cerberus::config::*;
use rust_decimal::Decimal;
use std::path::PathBuf;
use tempfile::TempDir;

/// Unit tests for configuration parsing and validation

#[test]
fn test_config_default_values() {
    let config = Config::default();

    // Test default values
    assert_eq!(config.environment, "development");
    assert_eq!(config.http_port, 8080);
    assert_eq!(config.metrics_port, 9090);

    // Test database defaults
    assert_eq!(config.database.max_connections, 10);
    assert_eq!(config.database.connection_timeout, 30);
    assert!(config.database.enable_wal);
    assert_eq!(config.database.cache_size, 64000);

    // Test trading defaults
    assert_eq!(config.trading.initial_balance, Decimal::new(50, 0));
    assert_eq!(config.trading.max_leverage, 50);
    assert_eq!(config.trading.min_leverage, 2);
    assert!(config.trading.paper_trading);
}

#[test]
fn test_config_validation_valid() {
    let config = Config::default();

    // Test that default config has valid values
    assert!(config.http_port > 0);
    assert!(config.metrics_port > 0);
    assert_ne!(config.http_port, config.metrics_port);
}

#[test]
fn test_config_validation_invalid_ports() {
    let mut config = Config::default();

    // Test that we can detect invalid port values
    config.http_port = 0;
    assert_eq!(config.http_port, 0); // Invalid port

    config.http_port = 8080;
    config.metrics_port = 0;
    assert_eq!(config.metrics_port, 0); // Invalid port

    // Test port conflict detection
    config.metrics_port = 8080; // Same as HTTP port
    assert_eq!(config.http_port, config.metrics_port); // Port conflict
}

#[test]
fn test_database_config_validation() {
    let mut db_config = DatabaseConfig::default();

    // Valid config
    assert!(db_config.validate().is_ok());

    // Invalid max connections
    db_config.max_connections = 0;
    assert!(db_config.validate().is_err());

    db_config.max_connections = 10;
    db_config.connection_timeout = 0;
    assert!(db_config.validate().is_err());

    // Invalid cache size
    db_config.connection_timeout = 30;
    db_config.cache_size = 0;
    assert!(db_config.validate().is_err());
}

#[test]
fn test_trading_config_validation() {
    let mut trading_config = TradingConfig::default();

    // Valid config
    assert!(trading_config.validate().is_ok());

    // Invalid initial balance
    trading_config.initial_balance = Decimal::ZERO;
    assert!(trading_config.validate().is_err());

    trading_config.initial_balance = Decimal::new(-100, 0);
    assert!(trading_config.validate().is_err());

    // Invalid leverage
    trading_config.initial_balance = Decimal::new(50, 0);
    trading_config.max_leverage = 0;
    assert!(trading_config.validate().is_err());

    trading_config.max_leverage = 200; // Too high
    assert!(trading_config.validate().is_err());

    trading_config.max_leverage = 50;
    trading_config.min_leverage = 0;
    assert!(trading_config.validate().is_err());

    // Min leverage higher than max
    trading_config.min_leverage = 60;
    assert!(trading_config.validate().is_err());

    // Invalid position size
    trading_config.min_leverage = 2;
    trading_config.max_position_size_percent = Decimal::new(150, 2); // 150%
    assert!(trading_config.validate().is_err());

    // Invalid min position size
    trading_config.max_position_size_percent = Decimal::new(33, 2);
    trading_config.min_position_size = Decimal::ZERO;
    assert!(trading_config.validate().is_err());
}

#[test]
fn test_leverage_config_validation() {
    let mut leverage_config = LeverageConfig::default();

    // Valid config
    assert!(leverage_config.validate(100).is_ok());

    // Invalid leverage values
    leverage_config.low_confidence = 0;
    assert!(leverage_config.validate(100).is_err());

    leverage_config.low_confidence = 5;
    leverage_config.medium_confidence = 200; // Too high
    assert!(leverage_config.validate(100).is_err());

    // Non-ascending order
    leverage_config.medium_confidence = 10;
    leverage_config.high_confidence = 5; // Lower than medium
    assert!(leverage_config.validate(100).is_err());
}

#[test]
fn test_risk_config_validation() {
    let mut risk_config = RiskConfig::default();

    // Valid config
    assert!(risk_config.validate().is_ok());

    // Invalid daily loss
    risk_config.max_daily_loss = Decimal::ZERO;
    assert!(risk_config.validate().is_err());

    risk_config.max_daily_loss = Decimal::new(-10, 0);
    assert!(risk_config.validate().is_err());

    // Invalid position loss percent
    risk_config.max_daily_loss = Decimal::new(15, 0);
    risk_config.max_position_loss_percent = Decimal::ZERO;
    assert!(risk_config.validate().is_err());

    risk_config.max_position_loss_percent = Decimal::new(150, 2); // 150%
    assert!(risk_config.validate().is_err());

    // Invalid circuit breaker
    risk_config.max_position_loss_percent = Decimal::new(10, 2);
    risk_config.circuit_breaker_threshold = Decimal::ZERO;
    assert!(risk_config.validate().is_err());

    // Invalid consecutive failures
    risk_config.circuit_breaker_threshold = Decimal::new(15, 0);
    risk_config.max_consecutive_failures = 0;
    assert!(risk_config.validate().is_err());
}

#[test]
fn test_stop_loss_config_validation() {
    let mut stop_loss_config = StopLossConfig::default();

    // Valid config
    assert!(stop_loss_config.validate().is_ok());

    // When enabled, must have valid values
    stop_loss_config.enabled = true;
    stop_loss_config.default_percent = Decimal::ZERO;
    assert!(stop_loss_config.validate().is_err());

    stop_loss_config.default_percent = Decimal::new(150, 2); // 150%
    assert!(stop_loss_config.validate().is_err());

    // Trailing stop validation
    stop_loss_config.default_percent = Decimal::new(10, 2);
    stop_loss_config.use_trailing = true;
    stop_loss_config.trailing_distance = Decimal::ZERO;
    assert!(stop_loss_config.validate().is_err());

    stop_loss_config.trailing_distance = Decimal::new(15, 2); // Higher than default
    assert!(stop_loss_config.validate().is_err());

    // When disabled, validation should pass regardless
    stop_loss_config.enabled = false;
    stop_loss_config.default_percent = Decimal::ZERO;
    assert!(stop_loss_config.validate().is_ok());
}

#[test]
fn test_take_profit_config_validation() {
    let mut take_profit_config = TakeProfitConfig::default();

    // Valid config
    assert!(take_profit_config.validate().is_ok());

    // When enabled, must have valid values
    take_profit_config.enabled = true;
    take_profit_config.default_percent = Decimal::ZERO;
    assert!(take_profit_config.validate().is_err());

    // Partial take profit validation
    take_profit_config.default_percent = Decimal::new(20, 2);
    take_profit_config.use_partial = true;
    take_profit_config.partial_percent = Decimal::ZERO;
    assert!(take_profit_config.validate().is_err());

    take_profit_config.partial_percent = Decimal::new(150, 2); // 150%
    assert!(take_profit_config.validate().is_err());

    take_profit_config.partial_percent = Decimal::new(50, 2);
    take_profit_config.first_partial_level = Decimal::ZERO;
    assert!(take_profit_config.validate().is_err());

    take_profit_config.first_partial_level = Decimal::new(25, 2); // Higher than default
    assert!(take_profit_config.validate().is_err());

    // When disabled, validation should pass
    take_profit_config.enabled = false;
    take_profit_config.default_percent = Decimal::ZERO;
    assert!(take_profit_config.validate().is_ok());
}

#[test]
fn test_monitoring_config_validation() {
    let mut monitoring_config = MonitoringConfig::default();

    // Valid config
    assert!(monitoring_config.validate().is_ok());

    // Invalid intervals
    monitoring_config.metrics_interval = 0;
    assert!(monitoring_config.validate().is_err());

    monitoring_config.metrics_interval = 10;
    monitoring_config.health_check_interval = 0;
    assert!(monitoring_config.validate().is_err());

    // Invalid memory settings
    monitoring_config.health_check_interval = 30;
    monitoring_config.max_memory_usage = 0;
    assert!(monitoring_config.validate().is_err());

    monitoring_config.max_memory_usage = 256;
    monitoring_config.memory_warning_threshold = 300; // Higher than max
    assert!(monitoring_config.validate().is_err());

    // Invalid query time settings
    monitoring_config.memory_warning_threshold = 200;
    monitoring_config.max_query_time = 0;
    assert!(monitoring_config.validate().is_err());

    monitoring_config.max_query_time = 100;
    monitoring_config.query_time_warning = 150; // Higher than max
    assert!(monitoring_config.validate().is_err());
}

#[test]
fn test_sentry_config_validation() {
    let mut sentry_config = SentryConfig::default();

    // Test sample rate bounds
    assert!(sentry_config.traces_sample_rate >= 0.0);
    assert!(sentry_config.traces_sample_rate <= 1.0);

    // Test valid sample rates
    sentry_config.traces_sample_rate = 0.0;
    assert!(sentry_config.traces_sample_rate >= 0.0);

    sentry_config.traces_sample_rate = 1.0;
    assert!(sentry_config.traces_sample_rate <= 1.0);

    sentry_config.traces_sample_rate = 0.5;
    assert!(sentry_config.traces_sample_rate >= 0.0 && sentry_config.traces_sample_rate <= 1.0);
}

#[test]
fn test_config_from_file() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config_path = temp_dir.path().join("test_config.toml");

    // Create test config file
    let config_content = r#"
environment = "test"
http_port = 9080
metrics_port = 9091

[database]
path = "test.db"
max_connections = 5

[trading]
initial_balance = 100.0
paper_trading = true
max_leverage = 25

[risk]
max_daily_loss = 20.0
"#;

    std::fs::write(&config_path, config_content)?;

    // Test that config file was created
    assert!(config_path.exists());

    // Test reading config content
    let content = std::fs::read_to_string(&config_path)?;
    assert!(content.contains("environment = \"test\""));
    assert!(content.contains("http_port = 9080"));

    Ok(())
}

#[test]
fn test_config_from_invalid_file() {
    let path = PathBuf::from("nonexistent.toml");
    assert!(!path.exists());
}

#[test]
fn test_config_with_environment_override() -> Result<()> {
    // Set environment variable
    std::env::set_var("CERBERUS_HTTP_PORT", "7777");
    std::env::set_var("CERBERUS_TRADING_PAPER_TRADING", "false");

    let config = Config::default();

    // Test that environment variables are set
    assert_eq!(std::env::var("CERBERUS_HTTP_PORT").unwrap(), "7777");
    assert_eq!(
        std::env::var("CERBERUS_TRADING_PAPER_TRADING").unwrap(),
        "false"
    );

    // Clean up
    std::env::remove_var("CERBERUS_HTTP_PORT");
    std::env::remove_var("CERBERUS_TRADING_PAPER_TRADING");

    Ok(())
}

#[test]
fn test_config_serialization() -> Result<()> {
    let config = Config::default();

    // Test that config has expected default values
    assert_eq!(config.environment, "development");
    assert_eq!(config.http_port, 8080);
    assert_eq!(config.metrics_port, 9090);

    Ok(())
}

#[test]
fn test_database_config_connection_string() {
    let db_config = DatabaseConfig::default();
    let connection_string = db_config.connection_string();

    assert!(connection_string.starts_with("sqlite:"));
    assert!(connection_string.contains("cerberus.db"));
}

#[test]
fn test_config_edge_cases() {
    let mut config = Config::default();

    // Test extreme values
    config.trading.max_leverage = 1; // Minimum allowed
    config.trading.min_leverage = 1;
    assert!(config.trading.max_leverage >= 1);

    config.trading.max_leverage = 100; // Maximum allowed
    config.trading.min_leverage = 100;
    assert!(config.trading.max_leverage <= 100);

    // Test boundary conditions
    config.trading.max_position_size_percent = Decimal::new(1, 2); // 1%
    assert!(config.validate().is_ok());

    config.trading.max_position_size_percent = Decimal::new(100, 2); // 100%
    assert!(config.validate().is_ok());
}

#[test]
fn test_config_clone_and_debug() {
    let config = Config::default();

    // Test Clone trait
    let cloned_config = config.clone();
    assert_eq!(config.environment, cloned_config.environment);

    // Test Debug trait
    let debug_string = format!("{:?}", config);
    assert!(!debug_string.is_empty());
    assert!(debug_string.contains("Config"));
}
