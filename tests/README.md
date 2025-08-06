# Cerberus v5.0 Testing Suite

Comprehensive testing framework for the Cerberus trading system, ensuring reliability, performance, and security.

## 🧪 Test Structure

```
tests/
├── README.md              # This file
├── mod.rs                 # Test module organization
├── test_utils.rs          # Common utilities and mocks
├── unit_config.rs         # Configuration testing
├── unit_cache.rs          # Cache system testing
├── unit_security.rs       # Security testing
├── unit_errors.rs         # Error handling testing
├── integration.rs         # Integration testing
└── performance.rs         # Performance benchmarks
```

## 🚀 Quick Start

### Run All Tests
```bash
make test-all
```

### Development Testing
```bash
make test-quick    # Fast unit tests only
make test-local    # Local development suite
```

### Specific Test Categories
```bash
make test-unit         # Unit tests
make test-integration  # Integration tests
make test-performance  # Performance tests
make test-security     # Security tests
```

## 📊 Test Categories

### 1. Unit Tests
Test individual components in isolation:

- **Configuration**: Parsing, validation, environment overrides
- **Cache**: Entry management, statistics, performance
- **Security**: Event monitoring, validation, encryption
- **Signals**: Creation, validation, processing
- **Risk Management**: Portfolio calculations, position sizing
- **Trading**: Order creation, validation, execution simulation
- **Error Handling**: Error types, conversions, reporting

### 2. Integration Tests
Test component interactions:

- **Complete Trading Workflow**: Signal → Analysis → Execution
- **Database Operations**: Persistence, transactions, backups
- **Cache Integration**: Signal caching, performance monitoring
- **Security Monitoring**: Event logging, threat detection
- **System Health**: Monitoring, metrics, alerts

### 3. Performance Tests
Ensure latency requirements are met:

- **Signal Processing**: < 10ms per signal
- **Trade Execution**: < 100ms end-to-end
- **Portfolio Updates**: < 5ms per update
- **Database Queries**: < 50ms average
- **Memory Usage**: < 512MB under normal load

### 4. Security Tests
Validate security measures:

- **Input Validation**: All external data sanitized
- **Authentication**: Token validation, password strength
- **Authorization**: Access control, IP filtering
- **Encryption**: Key handling, secure storage
- **Monitoring**: Security event detection

## 🛠️ Test Utilities

### Mock Components

#### MockSignalSource
```rust
let mut source = MockSignalSource::new("test".to_string())
    .with_signals(test_signals)
    .with_error_rate(0.1); // 10% error rate

source.connect().await?;
let signals = source.get_signals().await?;
```

#### MockTradingExecutor
```rust
let executor = MockTradingExecutor::new(0.95) // 95% success rate
    .with_delay(50) // 50ms execution delay
    .with_balance(10000.0);

let result = executor.execute_order(order).await?;
```

### Test Data Generation
```rust
// Generate test signals
let signals = TestDataGenerator::generate_signals(100);

// Generate portfolio with positions
let portfolio = TestDataGenerator::generate_portfolio_with_positions(10);

// Generate test orders
let orders = TestDataGenerator::generate_orders(50);
```

### Performance Measurement
```rust
let mut measurer = PerformanceMeasurer::new();

let result = measurer.measure("operation_name", || {
    // Your operation here
});

// Get statistics
let avg = measurer.get_average("operation_name");
let p95 = measurer.get_percentile("operation_name", 95.0);
```

### Test Environment
```rust
let env = TestEnvironment::new().await?;
// Provides: temp_dir, config, cache_manager, security_monitor
```

## 📈 Performance Requirements

### Latency Targets
- **Signal Processing**: < 10ms per signal
- **Trade Execution**: < 100ms end-to-end
- **Portfolio Updates**: < 5ms per update
- **Database Queries**: < 50ms average
- **Cache Operations**: < 1ms per operation

### Throughput Targets
- **Signal Processing**: > 1000 signals/second
- **Trade Orders**: > 100 orders/second
- **Database Operations**: > 500 queries/second
- **Cache Operations**: > 10,000 operations/second

### Resource Limits
- **Memory Usage**: < 512MB under normal load
- **CPU Usage**: < 50% average
- **Disk I/O**: < 100MB/s sustained
- **Network**: < 10MB/s sustained

## 🔒 Security Testing

### Test Categories
1. **Input Validation**: SQL injection, XSS, buffer overflows
2. **Authentication**: Token validation, session management
3. **Authorization**: Access control, privilege escalation
4. **Encryption**: Key management, data protection
5. **Monitoring**: Security event detection, alerting

### Security Test Data
```rust
// Test invalid inputs
let invalid_inputs = vec![
    "'; DROP TABLE users; --",
    "<script>alert('xss')</script>",
    "A".repeat(10000), // Buffer overflow attempt
];

// Test authentication
SecurityValidator::validate_auth_token("invalid_token");
SecurityValidator::check_password_strength("weak");

// Test monitoring
security_monitor.unauthorized_access("Suspicious activity", Some("192.168.1.1"));
```

## 🎯 Test Configuration

### Environment Variables
```bash
CERBERUS_ENVIRONMENT=test
CERBERUS_TRADING_PAPER_TRADING=true
RUST_LOG=debug
RUST_BACKTRACE=1
```

### Test-Specific Config
```toml
[test]
paper_trading = true
initial_balance = 1000.0
max_leverage = 10
enable_monitoring = true
```

## 📋 Test Execution

### Local Development
```bash
# Quick feedback loop
make test-quick

# Full local testing
make test-local

# Watch mode for continuous testing
make test-watch

# Test specific module
make test-module MODULE=signals
```

### CI/CD Pipeline
```bash
# Complete CI pipeline
make test-ci

# Coverage reporting
make test-coverage

# Security audit
make test-audit
```

### Performance Testing
```bash
# Standard performance tests
make test-performance

# Stress testing (long running)
make test-stress

# Memory leak detection
make test-memory
```

## 📊 Coverage Requirements

### Minimum Coverage Targets
- **Overall**: 80% line coverage
- **Critical Components**: 90% line coverage
- **Security Functions**: 95% line coverage
- **Error Handling**: 85% line coverage

### Coverage Exclusions
- Test utilities and mocks
- Generated code
- External dependencies
- Platform-specific code

## 🐛 Debugging Tests

### Enable Detailed Logging
```bash
RUST_LOG=trace cargo test test_name -- --nocapture
```

### Run Single Test
```bash
cargo test test_specific_function -- --nocapture
```

### Debug with GDB
```bash
cargo test --no-run
gdb target/debug/deps/cerberus-*
```

### Memory Debugging
```bash
valgrind --tool=memcheck cargo test
```

## 🔄 Continuous Integration

### GitHub Actions Integration
```yaml
- name: Run Tests
  run: make test-ci

- name: Upload Coverage
  uses: codecov/codecov-action@v3
  with:
    file: coverage/cobertura.xml
```

### Test Artifacts
- Coverage reports (HTML/XML)
- Performance benchmarks
- Security scan results
- Test logs and traces

## 📝 Writing New Tests

### Unit Test Template
```rust
#[test]
fn test_component_functionality() {
    // Arrange
    let input = create_test_input();
    
    // Act
    let result = component.process(input);
    
    // Assert
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), expected_output);
}
```

### Integration Test Template
```rust
#[tokio::test]
async fn test_component_integration() -> Result<()> {
    // Setup
    let env = TestEnvironment::new().await?;
    
    // Test workflow
    let signal = create_test_signal();
    let result = process_signal(signal, &env).await?;
    
    // Verify
    assert!(result.is_successful());
    Ok(())
}
```

### Performance Test Template
```rust
#[test]
fn test_performance_requirement() {
    let start = Instant::now();
    
    // Operation under test
    perform_operation();
    
    let duration = start.elapsed();
    assert!(duration < Duration::from_millis(10));
}
```

## 🎯 Best Practices

1. **Test Isolation**: Each test should be independent
2. **Paper Trading**: Always use paper trading mode
3. **Deterministic**: Tests should be repeatable
4. **Fast Feedback**: Unit tests should run quickly
5. **Comprehensive**: Cover happy path and edge cases
6. **Readable**: Clear test names and structure
7. **Maintainable**: Keep tests simple and focused

## 🚨 Safety Considerations

⚠️ **CRITICAL**: All tests must run in paper trading mode only!

- Never use real API keys in tests
- Never execute real trades
- Always validate test environment
- Use mock data and services
- Isolate test databases

## 📞 Support

For testing issues or questions:
- Check test logs in `logs/test/`
- Review coverage reports in `coverage/`
- Consult the main README.md
- Open GitHub issues for bugs

---

**Remember**: Comprehensive testing is crucial for a trading system. When in doubt, add more tests! 🧪
