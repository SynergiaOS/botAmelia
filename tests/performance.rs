use anyhow::Result;
use cerberus::{
    cache::{CacheEntry, CacheStats},
    risk::{Portfolio, Position, TradeSide},
    signals::{Confidence, Signal},
    trading::{OrderType, TradeOrder},
};
use serde_json::json;
use std::time::{Duration, Instant};

/// Performance tests for latency-critical operations
/// These tests ensure the system meets performance requirements

const SIGNAL_PROCESSING_MAX_MS: u128 = 10;
const TRADE_EXECUTION_MAX_MS: u128 = 100;
const DATABASE_QUERY_MAX_MS: u128 = 50;
const PORTFOLIO_UPDATE_MAX_MS: u128 = 5;

#[test]
fn test_signal_creation_performance() {
    let start = Instant::now();

    for i in 0..1000 {
        let _signal = Signal::new(
            format!("TOKEN_{}", i),
            "performance_test".to_string(),
            Confidence::High,
            0.001 + (i as f64 * 0.0001),
            100000.0 + (i as f64 * 1000.0),
            json!({"iteration": i}),
        );
    }

    let duration = start.elapsed();
    let avg_time_ms = duration.as_millis() / 1000;

    println!(
        "Signal creation: {} signals in {}ms (avg: {}ms per signal)",
        1000,
        duration.as_millis(),
        avg_time_ms
    );

    // Each signal creation should be very fast
    assert!(
        avg_time_ms < 1,
        "Signal creation too slow: {}ms average",
        avg_time_ms
    );
}

#[test]
fn test_signal_validation_performance() {
    // Create test signals
    let signals: Vec<Signal> = (0..1000)
        .map(|i| {
            Signal::new(
                format!("TOKEN_{}", i),
                "performance_test".to_string(),
                Confidence::Medium,
                0.001,
                100000.0,
                json!({"test": true}),
            )
        })
        .collect();

    let start = Instant::now();

    for signal in &signals {
        let _ = signal.validate();
    }

    let duration = start.elapsed();
    let avg_time_ms = duration.as_millis() / 1000;

    println!(
        "Signal validation: {} validations in {}ms (avg: {}ms per validation)",
        1000,
        duration.as_millis(),
        avg_time_ms
    );

    assert!(
        avg_time_ms < 1,
        "Signal validation too slow: {}ms average",
        avg_time_ms
    );
}

#[test]
fn test_signal_hash_calculation_performance() {
    let signal = Signal::new(
        "PERFORMANCE_TEST".to_string(),
        "test".to_string(),
        Confidence::High,
        0.001,
        1000000.0,
        json!({"large_metadata": "x".repeat(1000)}),
    );

    let start = Instant::now();

    for _ in 0..10000 {
        let _hash = signal.calculate_hash();
    }

    let duration = start.elapsed();
    let avg_time_ns = duration.as_nanos() / 10000;

    println!(
        "Hash calculation: 10000 hashes in {}ms (avg: {}ns per hash)",
        duration.as_millis(),
        avg_time_ns
    );

    // Hash calculation should be very fast (< 1ms for 10k operations)
    assert!(
        duration.as_millis() < 100,
        "Hash calculation too slow: {}ms for 10k operations",
        duration.as_millis()
    );
}

#[test]
fn test_portfolio_update_performance() {
    let mut portfolio = Portfolio::new(10000.0);

    // Add many positions
    for i in 0..100 {
        let position = Position::new(
            format!("TOKEN_{}", i),
            if i % 2 == 0 {
                TradeSide::Long
            } else {
                TradeSide::Short
            },
            100.0,
            5,
            0.001 + (i as f64 * 0.0001),
        );
        portfolio.add_position(position);
    }

    let start = Instant::now();

    // Update portfolio multiple times
    for _ in 0..1000 {
        portfolio.update();
    }

    let duration = start.elapsed();
    let avg_time_ms = duration.as_millis() / 1000;

    println!(
        "Portfolio update: 1000 updates with 100 positions in {}ms (avg: {}ms per update)",
        duration.as_millis(),
        avg_time_ms
    );

    assert!(
        avg_time_ms < PORTFOLIO_UPDATE_MAX_MS,
        "Portfolio update too slow: {}ms average (max: {}ms)",
        avg_time_ms,
        PORTFOLIO_UPDATE_MAX_MS
    );
}

#[test]
fn test_position_pnl_calculation_performance() {
    let mut positions: Vec<Position> = (0..1000)
        .map(|i| {
            Position::new(
                format!("TOKEN_{}", i),
                if i % 2 == 0 {
                    TradeSide::Long
                } else {
                    TradeSide::Short
                },
                100.0,
                5,
                0.001,
            )
        })
        .collect();

    let start = Instant::now();

    // Update prices and calculate PnL
    for position in &mut positions {
        position.update_price(0.0012); // 20% price change
        let _pnl = position.calculate_pnl();
    }

    let duration = start.elapsed();
    let avg_time_ns = duration.as_nanos() / 1000;

    println!(
        "PnL calculation: 1000 calculations in {}ms (avg: {}ns per calculation)",
        duration.as_millis(),
        avg_time_ns
    );

    // PnL calculation should be very fast
    assert!(
        duration.as_millis() < 10,
        "PnL calculation too slow: {}ms for 1000 calculations",
        duration.as_millis()
    );
}

#[test]
fn test_trade_order_creation_performance() {
    let start = Instant::now();

    for i in 0..1000 {
        let _order = TradeOrder::market_order(
            format!("TOKEN_{}", i),
            if i % 2 == 0 {
                TradeSide::Long
            } else {
                TradeSide::Short
            },
            100.0,
            5,
        );
    }

    let duration = start.elapsed();
    let avg_time_ms = duration.as_millis() / 1000;

    println!(
        "Trade order creation: 1000 orders in {}ms (avg: {}ms per order)",
        duration.as_millis(),
        avg_time_ms
    );

    assert!(
        avg_time_ms < 1,
        "Trade order creation too slow: {}ms average",
        avg_time_ms
    );
}

#[test]
fn test_trade_order_validation_performance() {
    let orders: Vec<TradeOrder> = (0..1000)
        .map(|i| {
            TradeOrder::market_order(
                format!("TOKEN_{}", i),
                if i % 2 == 0 {
                    TradeSide::Long
                } else {
                    TradeSide::Short
                },
                100.0,
                5,
            )
        })
        .collect();

    let start = Instant::now();

    for order in &orders {
        let _ = order.validate();
    }

    let duration = start.elapsed();
    let avg_time_ms = duration.as_millis() / 1000;

    println!(
        "Trade order validation: 1000 validations in {}ms (avg: {}ms per validation)",
        duration.as_millis(),
        avg_time_ms
    );

    assert!(
        avg_time_ms < 1,
        "Trade order validation too slow: {}ms average",
        avg_time_ms
    );
}

#[test]
fn test_cache_entry_operations_performance() {
    let start = Instant::now();

    // Create many cache entries
    let mut entries = Vec::new();
    for i in 0..10000 {
        let entry = CacheEntry::new(format!("value_{}", i), 300);
        entries.push(entry);
    }

    let creation_time = start.elapsed();

    let start = Instant::now();

    // Test access operations
    for entry in &mut entries {
        entry.mark_accessed();
        let _expired = entry.is_expired();
        let _age = entry.age_seconds();
        let _ttl = entry.ttl_seconds();
    }

    let access_time = start.elapsed();

    println!(
        "Cache entry creation: 10000 entries in {}ms",
        creation_time.as_millis()
    );
    println!(
        "Cache entry access: 10000 operations in {}ms",
        access_time.as_millis()
    );

    assert!(
        creation_time.as_millis() < 100,
        "Cache entry creation too slow: {}ms",
        creation_time.as_millis()
    );
    assert!(
        access_time.as_millis() < 50,
        "Cache entry access too slow: {}ms",
        access_time.as_millis()
    );
}

#[test]
fn test_cache_stats_update_performance() {
    let mut stats = CacheStats::default();

    let start = Instant::now();

    for _ in 0..10000 {
        stats.record_hit();
        stats.record_miss();
        let _hit_rate = stats.hit_rate;
    }

    let duration = start.elapsed();

    println!(
        "Cache stats updates: 20000 operations in {}ms",
        duration.as_millis()
    );

    assert!(
        duration.as_millis() < 50,
        "Cache stats updates too slow: {}ms",
        duration.as_millis()
    );
}

#[test]
fn test_liquidation_price_calculation_performance() {
    let start = Instant::now();

    for i in 0..10000 {
        let price = 0.001 + (i as f64 * 0.0001);
        let leverage = (i % 50) + 1; // 1-50x leverage

        let _long_liq =
            Position::calculate_liquidation_price(&TradeSide::Long, price, leverage as u8);
        let _short_liq =
            Position::calculate_liquidation_price(&TradeSide::Short, price, leverage as u8);
    }

    let duration = start.elapsed();
    let avg_time_ns = duration.as_nanos() / 20000; // 2 calculations per iteration

    println!(
        "Liquidation price calculation: 20000 calculations in {}ms (avg: {}ns per calculation)",
        duration.as_millis(),
        avg_time_ns
    );

    assert!(
        duration.as_millis() < 10,
        "Liquidation price calculation too slow: {}ms",
        duration.as_millis()
    );
}

#[test]
#[ignore] // Only run with --ignored flag for stress testing
fn stress_test_concurrent_signal_processing() {
    use std::sync::{Arc, Mutex};
    use std::thread;

    let processed_count = Arc::new(Mutex::new(0));
    let mut handles = vec![];

    let start = Instant::now();

    // Spawn multiple threads to simulate concurrent processing
    for thread_id in 0..4 {
        let count = Arc::clone(&processed_count);

        let handle = thread::spawn(move || {
            for i in 0..250 {
                // 250 signals per thread = 1000 total
                let signal = Signal::new(
                    format!("THREAD_{}_TOKEN_{}", thread_id, i),
                    "stress_test".to_string(),
                    Confidence::High,
                    0.001,
                    100000.0,
                    json!({"thread": thread_id, "iteration": i}),
                );

                let _ = signal.validate();
                let _ = signal.calculate_hash();

                let mut count = count.lock().unwrap();
                *count += 1;
            }
        });

        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }

    let duration = start.elapsed();
    let processed = *processed_count.lock().unwrap();

    println!(
        "Stress test: {} signals processed in {}ms across 4 threads",
        processed,
        duration.as_millis()
    );

    assert_eq!(processed, 1000);
    assert!(
        duration.as_millis() < 1000,
        "Stress test too slow: {}ms",
        duration.as_millis()
    );
}

#[test]
#[ignore] // Only run with --ignored flag for memory testing
fn memory_usage_test() {
    // Test memory usage with large number of objects
    let mut signals = Vec::new();
    let mut portfolios = Vec::new();
    let mut positions = Vec::new();

    // Create many objects to test memory usage
    for i in 0..10000 {
        let signal = Signal::new(
            format!("MEMORY_TEST_{}", i),
            "memory_test".to_string(),
            Confidence::Medium,
            0.001,
            100000.0,
            json!({"large_data": "x".repeat(100)}),
        );
        signals.push(signal);

        if i % 100 == 0 {
            let portfolio = Portfolio::new(1000.0);
            portfolios.push(portfolio);
        }

        let position = Position::new(
            format!("POS_{}", i),
            if i % 2 == 0 {
                TradeSide::Long
            } else {
                TradeSide::Short
            },
            100.0,
            5,
            0.001,
        );
        positions.push(position);
    }

    println!(
        "Created {} signals, {} portfolios, {} positions",
        signals.len(),
        portfolios.len(),
        positions.len()
    );

    // Test that objects are still accessible
    assert_eq!(signals.len(), 10000);
    assert_eq!(portfolios.len(), 100);
    assert_eq!(positions.len(), 10000);

    // Test random access performance
    let start = Instant::now();

    for i in (0..1000).step_by(10) {
        let _ = &signals[i];
        let _ = &positions[i];
        if i < portfolios.len() {
            let _ = &portfolios[i];
        }
    }

    let access_time = start.elapsed();

    println!(
        "Random access test completed in {}ms",
        access_time.as_millis()
    );
    assert!(
        access_time.as_millis() < 10,
        "Random access too slow: {}ms",
        access_time.as_millis()
    );
}

#[test]
fn test_serialization_performance() -> Result<()> {
    // Create test objects
    let signal = Signal::new(
        "SERIALIZATION_TEST".to_string(),
        "test".to_string(),
        Confidence::High,
        0.001,
        1000000.0,
        json!({"complex": {"nested": {"data": [1, 2, 3, 4, 5]}}}),
    );

    let position = Position::new("TEST_TOKEN".to_string(), TradeSide::Long, 1000.0, 10, 0.001);

    // Test JSON serialization performance
    let start = Instant::now();

    for _ in 0..1000 {
        let _signal_json = serde_json::to_string(&signal)?;
        let _position_json = serde_json::to_string(&position)?;
    }

    let serialization_time = start.elapsed();

    // Test JSON deserialization performance
    let signal_json = serde_json::to_string(&signal)?;
    let position_json = serde_json::to_string(&position)?;

    let start = Instant::now();

    for _ in 0..1000 {
        let _signal: Signal = serde_json::from_str(&signal_json)?;
        let _position: Position = serde_json::from_str(&position_json)?;
    }

    let deserialization_time = start.elapsed();

    println!(
        "Serialization: 2000 objects in {}ms",
        serialization_time.as_millis()
    );
    println!(
        "Deserialization: 2000 objects in {}ms",
        deserialization_time.as_millis()
    );

    assert!(
        serialization_time.as_millis() < 100,
        "Serialization too slow: {}ms",
        serialization_time.as_millis()
    );
    assert!(
        deserialization_time.as_millis() < 100,
        "Deserialization too slow: {}ms",
        deserialization_time.as_millis()
    );

    Ok(())
}

#[test]
fn benchmark_critical_path_operations() {
    // Simulate the critical path: Signal -> Validation -> Risk Assessment -> Order Creation
    let start = Instant::now();

    for i in 0..100 {
        // 1. Signal creation and validation
        let signal = Signal::new(
            format!("CRITICAL_PATH_{}", i),
            "benchmark".to_string(),
            Confidence::High,
            0.001,
            1000000.0,
            json!({"benchmark": true}),
        );

        let _validation_result = signal.validate();

        // 2. Portfolio check (simplified)
        let portfolio = Portfolio::new(1000.0);
        let _is_healthy = portfolio.is_healthy();

        // 3. Order creation
        let _order = TradeOrder::market_order(signal.token.clone(), TradeSide::Long, 100.0, 5);

        // 4. Order validation
        let _order_validation = _order.validate();
    }

    let duration = start.elapsed();
    let avg_time_ms = duration.as_millis() / 100;

    println!(
        "Critical path benchmark: 100 complete cycles in {}ms (avg: {}ms per cycle)",
        duration.as_millis(),
        avg_time_ms
    );

    // Critical path should be very fast (< 10ms per complete cycle)
    assert!(
        avg_time_ms < SIGNAL_PROCESSING_MAX_MS,
        "Critical path too slow: {}ms average (max: {}ms)",
        avg_time_ms,
        SIGNAL_PROCESSING_MAX_MS
    );
}
