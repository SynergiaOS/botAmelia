#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(clippy::all)]

/// Test utilities and helpers for Cerberus v5.0 testing
///
/// This module provides common utilities, mocks, and helpers
/// used across different test suites.
use anyhow::Result;
use cerberus::{
    cache::CacheManager,
    config::Config,
    risk::{Portfolio, Position, TradeSide},
    security::SecurityMonitor,
    signals::{Confidence, Signal, SignalSource},
    trading::{ExecutionResult, TradeOrder},
};
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;
use tokio::time::{sleep, Duration};

/// Mock signal source for testing
pub struct MockSignalSource {
    pub name: String,
    pub signals: Vec<Signal>,
    pub connected: bool,
    pub error_rate: f64, // 0.0 = no errors, 1.0 = always error
    pub call_count: Arc<Mutex<usize>>,
}

impl MockSignalSource {
    pub fn new(name: String) -> Self {
        Self {
            name,
            signals: Vec::new(),
            connected: false,
            error_rate: 0.0,
            call_count: Arc::new(Mutex::new(0)),
        }
    }

    pub fn with_signals(mut self, signals: Vec<Signal>) -> Self {
        self.signals = signals;
        self
    }

    pub fn with_error_rate(mut self, error_rate: f64) -> Self {
        self.error_rate = error_rate.clamp(0.0, 1.0);
        self
    }

    pub fn add_signal(&mut self, signal: Signal) {
        self.signals.push(signal);
    }

    pub fn get_call_count(&self) -> usize {
        *self.call_count.lock().unwrap()
    }

    fn should_error(&self) -> bool {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        rng.gen::<f64>() < self.error_rate
    }
}

#[async_trait::async_trait]
impl SignalSource for MockSignalSource {
    async fn get_signals(&self) -> Result<Vec<Signal>> {
        let mut count = self.call_count.lock().unwrap();
        *count += 1;
        drop(count);

        if self.should_error() {
            return Err(anyhow::anyhow!("Mock error from {}", self.name));
        }

        if !self.connected {
            return Err(anyhow::anyhow!("Not connected"));
        }

        // Simulate network delay
        sleep(Duration::from_millis(10)).await;

        Ok(self.signals.clone())
    }

    fn source_name(&self) -> &str {
        &self.name
    }

    fn is_connected(&self) -> bool {
        self.connected
    }

    async fn connect(&mut self) -> Result<()> {
        // Simulate connection delay
        sleep(Duration::from_millis(50)).await;

        if self.should_error() {
            return Err(anyhow::anyhow!("Connection failed"));
        }

        self.connected = true;
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.connected = false;
        Ok(())
    }
}

/// Mock trading executor for testing
pub struct MockTradingExecutor {
    pub success_rate: f64,
    pub execution_delay_ms: u64,
    pub executed_orders: Arc<Mutex<Vec<TradeOrder>>>,
    pub balance: Arc<Mutex<f64>>,
}

impl MockTradingExecutor {
    pub fn new(success_rate: f64) -> Self {
        Self {
            success_rate: success_rate.clamp(0.0, 1.0),
            execution_delay_ms: 50,
            executed_orders: Arc::new(Mutex::new(Vec::new())),
            balance: Arc::new(Mutex::new(10000.0)),
        }
    }

    pub fn with_delay(mut self, delay_ms: u64) -> Self {
        self.execution_delay_ms = delay_ms;
        self
    }

    pub fn with_balance(mut self, balance: f64) -> Self {
        *self.balance.lock().unwrap() = balance;
        self
    }

    pub async fn execute_order(&self, order: TradeOrder) -> Result<ExecutionResult> {
        // Simulate execution delay
        sleep(Duration::from_millis(self.execution_delay_ms)).await;

        // Store executed order
        self.executed_orders.lock().unwrap().push(order.clone());

        // Determine if execution should succeed
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let should_succeed = rng.gen::<f64>() < self.success_rate;

        if !should_succeed {
            return Ok(ExecutionResult::error("Mock execution failure".to_string()));
        }

        // Check balance
        let balance = *self.balance.lock().unwrap();
        let required_margin = order.size / order.leverage as f64;

        if balance < required_margin {
            return Ok(ExecutionResult::error("Insufficient balance".to_string()));
        }

        // Simulate successful execution
        let executed_price = 0.001 + rng.gen::<f64>() * 0.0001; // Random price around 0.001
        let fees = order.size * 0.001; // 0.1% fee

        // Update balance
        *self.balance.lock().unwrap() -= required_margin + fees;

        Ok(ExecutionResult::success(
            format!("mock_tx_{}", uuid::Uuid::new_v4()),
            executed_price,
            order.size,
            fees,
        ))
    }

    pub fn get_executed_orders(&self) -> Vec<TradeOrder> {
        self.executed_orders.lock().unwrap().clone()
    }

    pub fn get_balance(&self) -> f64 {
        *self.balance.lock().unwrap()
    }

    pub fn reset(&self) {
        self.executed_orders.lock().unwrap().clear();
        *self.balance.lock().unwrap() = 10000.0;
    }
}

/// Test data generators
pub struct TestDataGenerator;

impl TestDataGenerator {
    /// Generate test signals with various characteristics
    pub fn generate_signals(count: usize) -> Vec<Signal> {
        let mut signals = Vec::new();

        for i in 0..count {
            let confidence = match i % 4 {
                0 => Confidence::Low,
                1 => Confidence::Medium,
                2 => Confidence::High,
                3 => Confidence::Extreme,
                _ => Confidence::Medium,
            };

            let signal = Signal::new(
                format!("TEST_TOKEN_{}", i),
                format!("test_source_{}", i % 3),
                confidence,
                0.001 + (i as f64 * 0.0001),
                100000.0 + (i as f64 * 10000.0),
                json!({
                    "test_id": i,
                    "volume_spike": 2.0 + (i as f64 * 0.5),
                    "sentiment": 0.5 + (i as f64 * 0.1) % 1.0
                }),
            );

            signals.push(signal);
        }

        signals
    }

    /// Generate test portfolio with positions
    pub fn generate_portfolio_with_positions(position_count: usize) -> Portfolio {
        let mut portfolio = Portfolio::new(10000.0);

        for i in 0..position_count {
            let position = Position::new(
                format!("POS_TOKEN_{}", i),
                if i % 2 == 0 {
                    TradeSide::Long
                } else {
                    TradeSide::Short
                },
                100.0 + (i as f64 * 50.0),
                5 + (i % 10) as u8,
                0.001 + (i as f64 * 0.0001),
            );

            portfolio.add_position(position);
        }

        portfolio
    }

    /// Generate test orders
    pub fn generate_orders(count: usize) -> Vec<TradeOrder> {
        let mut orders = Vec::new();

        for i in 0..count {
            let order = TradeOrder::market_order(
                format!("ORDER_TOKEN_{}", i),
                if i % 2 == 0 {
                    TradeSide::Long
                } else {
                    TradeSide::Short
                },
                100.0 + (i as f64 * 25.0),
                5 + (i % 10) as u8,
            );

            orders.push(order);
        }

        orders
    }
}

/// Test environment setup utilities
pub struct TestEnvironment {
    pub temp_dir: TempDir,
    pub config: Config,
    pub cache_manager: CacheManager,
    pub security_monitor: SecurityMonitor,
}

impl TestEnvironment {
    pub async fn new() -> Result<Self> {
        let temp_dir = TempDir::new()?;

        let mut config = Config::default();
        config.database.path = temp_dir.path().join("test.db");
        config.trading.paper_trading = true;
        config.trading.initial_balance = rust_decimal::Decimal::new(1000, 0);

        let cache_manager = CacheManager::new(true);
        let security_monitor = SecurityMonitor::new(true, 100);

        Ok(Self {
            temp_dir,
            config,
            cache_manager,
            security_monitor,
        })
    }

    pub fn config_path(&self) -> std::path::PathBuf {
        self.temp_dir.path().join("test_config.toml")
    }

    pub fn database_path(&self) -> std::path::PathBuf {
        self.config.database.path.clone()
    }
}

/// Performance measurement utilities
pub struct PerformanceMeasurer {
    measurements: HashMap<String, Vec<Duration>>,
}

impl PerformanceMeasurer {
    pub fn new() -> Self {
        Self {
            measurements: HashMap::new(),
        }
    }

    pub fn measure<F, R>(&mut self, name: &str, operation: F) -> R
    where
        F: FnOnce() -> R,
    {
        let start = std::time::Instant::now();
        let result = operation();
        let duration = start.elapsed();

        self.measurements
            .entry(name.to_string())
            .or_insert_with(Vec::new)
            .push(duration);

        result
    }

    pub async fn measure_async<F, Fut, R>(&mut self, name: &str, operation: F) -> R
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = R>,
    {
        let start = std::time::Instant::now();
        let result = operation().await;
        let duration = start.elapsed();

        self.measurements
            .entry(name.to_string())
            .or_insert_with(Vec::new)
            .push(duration);

        result
    }

    pub fn get_average(&self, name: &str) -> Option<Duration> {
        let measurements = self.measurements.get(name)?;
        if measurements.is_empty() {
            return None;
        }

        let total_nanos: u64 = measurements.iter().map(|d| d.as_nanos() as u64).sum();
        let avg_nanos = total_nanos / measurements.len() as u64;

        Some(Duration::from_nanos(avg_nanos))
    }

    pub fn get_percentile(&self, name: &str, percentile: f64) -> Option<Duration> {
        let mut measurements = self.measurements.get(name)?.clone();
        if measurements.is_empty() {
            return None;
        }

        measurements.sort();
        let index = ((measurements.len() as f64 - 1.0) * percentile / 100.0).round() as usize;
        Some(measurements[index])
    }

    pub fn print_summary(&self) {
        println!("\n=== Performance Summary ===");
        for (name, measurements) in &self.measurements {
            if let Some(avg) = self.get_average(name) {
                let p50 = self.get_percentile(name, 50.0).unwrap_or(avg);
                let p95 = self.get_percentile(name, 95.0).unwrap_or(avg);

                println!(
                    "{}: {} samples, avg: {:?}, p50: {:?}, p95: {:?}",
                    name,
                    measurements.len(),
                    avg,
                    p50,
                    p95
                );
            }
        }
    }
}

/// Assertion helpers for tests
pub struct TestAssertions;

impl TestAssertions {
    /// Assert that a value is within a percentage of expected
    pub fn assert_within_percent(actual: f64, expected: f64, percent: f64) {
        let tolerance = expected.abs() * percent / 100.0;
        let diff = (actual - expected).abs();
        assert!(
            diff <= tolerance,
            "Value {} is not within {}% of expected {} (diff: {}, tolerance: {})",
            actual,
            percent,
            expected,
            diff,
            tolerance
        );
    }

    /// Assert that a duration is within expected bounds
    pub fn assert_duration_within(actual: Duration, max_duration: Duration) {
        assert!(
            actual <= max_duration,
            "Duration {:?} exceeds maximum {:?}",
            actual,
            max_duration
        );
    }

    /// Assert that a collection has expected size
    pub fn assert_collection_size<T>(collection: &[T], expected_size: usize) {
        assert_eq!(
            collection.len(),
            expected_size,
            "Collection has {} items, expected {}",
            collection.len(),
            expected_size
        );
    }

    /// Assert that a result is successful
    pub fn assert_success<T, E: std::fmt::Debug>(result: &Result<T, E>) {
        assert!(
            result.is_ok(),
            "Expected success but got error: {:?}",
            result.err()
        );
    }

    /// Assert that a result is an error
    pub fn assert_error<T: std::fmt::Debug, E>(result: &Result<T, E>) {
        assert!(
            result.is_err(),
            "Expected error but got success: {:?}",
            result.as_ref().ok()
        );
    }
}

/// Test cleanup utilities
pub struct TestCleanup {
    cleanup_functions: Vec<Box<dyn FnOnce() + Send>>,
}

impl TestCleanup {
    pub fn new() -> Self {
        Self {
            cleanup_functions: Vec::new(),
        }
    }

    pub fn add_cleanup<F>(&mut self, cleanup: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.cleanup_functions.push(Box::new(cleanup));
    }

    pub fn cleanup(self) {
        for cleanup_fn in self.cleanup_functions {
            cleanup_fn();
        }
    }
}

impl Drop for TestCleanup {
    fn drop(&mut self) {
        // Cleanup any remaining functions
        while let Some(cleanup_fn) = self.cleanup_functions.pop() {
            cleanup_fn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_signal_source() {
        let mut source = MockSignalSource::new("test".to_string());

        // Test connection
        assert!(!source.is_connected());
        source.connect().await.unwrap();
        assert!(source.is_connected());

        // Test signal generation
        let signals = TestDataGenerator::generate_signals(5);
        source = source.with_signals(signals);

        let retrieved_signals = source.get_signals().await.unwrap();
        assert_eq!(retrieved_signals.len(), 5);
    }

    #[tokio::test]
    async fn test_mock_trading_executor() {
        let executor = MockTradingExecutor::new(1.0); // 100% success rate

        let order = TradeOrder::market_order("TEST".to_string(), TradeSide::Long, 100.0, 5);

        let result = executor.execute_order(order).await.unwrap();
        assert!(result.success);

        let executed_orders = executor.get_executed_orders();
        assert_eq!(executed_orders.len(), 1);
    }

    #[test]
    fn test_performance_measurer() {
        let mut measurer = PerformanceMeasurer::new();

        // Measure a simple operation
        let result = measurer.measure("test_operation", || {
            std::thread::sleep(Duration::from_millis(10));
            42
        });

        assert_eq!(result, 42);

        let avg = measurer.get_average("test_operation").unwrap();
        assert!(avg >= Duration::from_millis(10));
    }

    #[test]
    fn test_assertions() {
        TestAssertions::assert_within_percent(100.0, 95.0, 10.0); // Should pass
        TestAssertions::assert_duration_within(
            Duration::from_millis(50),
            Duration::from_millis(100),
        );
        TestAssertions::assert_collection_size(&vec![1, 2, 3], 3);

        let success_result: Result<i32, &str> = Ok(42);
        TestAssertions::assert_success(&success_result);

        let error_result: Result<i32, &str> = Err("error");
        TestAssertions::assert_error(&error_result);
    }
}
