# Cerberus v5.0 Testing Makefile
# Comprehensive test automation for the trading system

.PHONY: help test test-unit test-integration test-performance test-security test-all
.PHONY: test-coverage test-docs test-clippy test-fmt test-audit test-bench
.PHONY: test-quick test-ci test-local test-stress clean-test setup-test

# Default target
help:
	@echo "Cerberus v5.0 Testing Commands"
	@echo "=============================="
	@echo ""
	@echo "Basic Testing:"
	@echo "  test-quick      - Run quick unit tests only"
	@echo "  test-unit       - Run all unit tests"
	@echo "  test-integration- Run integration tests"
	@echo "  test-performance- Run performance tests"
	@echo "  test-security   - Run security tests"
	@echo "  test-all        - Run all test suites"
	@echo ""
	@echo "Quality Assurance:"
	@echo "  test-coverage   - Generate test coverage report"
	@echo "  test-clippy     - Run Clippy linter"
	@echo "  test-fmt        - Check code formatting"
	@echo "  test-audit      - Run security audit"
	@echo "  test-docs       - Test documentation"
	@echo ""
	@echo "CI/CD:"
	@echo "  test-ci         - Run CI test suite"
	@echo "  test-local      - Run local development tests"
	@echo "  test-stress     - Run stress tests (long running)"
	@echo ""
	@echo "Utilities:"
	@echo "  setup-test      - Setup test environment"
	@echo "  clean-test      - Clean test artifacts"
	@echo "  test-bench      - Run benchmarks"

# Environment setup
RUST_LOG ?= debug
RUST_BACKTRACE ?= 1
CERBERUS_ENVIRONMENT = test
CERBERUS_TRADING_PAPER_TRADING = true

# Test configuration
TEST_THREADS ?= 4
COVERAGE_THRESHOLD ?= 80
PERFORMANCE_TIMEOUT ?= 300

# Export environment variables
export RUST_LOG
export RUST_BACKTRACE
export CERBERUS_ENVIRONMENT
export CERBERUS_TRADING_PAPER_TRADING

# Quick tests for development
test-quick:
	@echo "🚀 Running quick unit tests..."
	@cargo test --lib --tests --bins --quiet --color=always -- --test-threads=$(TEST_THREADS)

# Unit tests
test-unit:
	@echo "🧪 Running unit tests..."
	@cargo test --lib --tests --bins --color=always -- --test-threads=$(TEST_THREADS) --nocapture

# Integration tests
test-integration:
	@echo "🔗 Running integration tests..."
	@cargo test --test integration --color=always -- --test-threads=$(TEST_THREADS) --nocapture

# Performance tests
test-performance:
	@echo "⚡ Running performance tests..."
	@cargo test --test performance --color=always -- --test-threads=1 --nocapture
	@echo "⚡ Running performance tests with ignored..."
	@cargo test --test performance --color=always -- --test-threads=1 --nocapture --ignored

# Security tests
test-security:
	@echo "🔒 Running security tests..."
	@cargo test unit_security --color=always -- --test-threads=$(TEST_THREADS) --nocapture

# All tests
test-all: test-unit test-integration test-performance test-security
	@echo "✅ All test suites completed!"

# Test coverage
test-coverage:
	@echo "📊 Generating test coverage report..."
	@command -v cargo-tarpaulin >/dev/null 2>&1 || { echo "Installing cargo-tarpaulin..."; cargo install cargo-tarpaulin; }
	@cargo tarpaulin --out Html --output-dir coverage --timeout $(PERFORMANCE_TIMEOUT) \
		--exclude-files "tests/*" "target/*" \
		--ignore-panics --ignore-tests \
		--fail-under $(COVERAGE_THRESHOLD)
	@echo "📊 Coverage report generated in coverage/tarpaulin-report.html"

# Code quality checks
test-clippy:
	@echo "📎 Running Clippy linter..."
	@cargo clippy --all-targets --all-features -- -D warnings

test-fmt:
	@echo "🎨 Checking code formatting..."
	@cargo fmt --all -- --check

test-audit:
	@echo "🔍 Running security audit..."
	@command -v cargo-audit >/dev/null 2>&1 || { echo "Installing cargo-audit..."; cargo install cargo-audit; }
	@cargo audit

test-docs:
	@echo "📚 Testing documentation..."
	@cargo doc --no-deps --document-private-items
	@cargo test --doc

# Benchmarks
test-bench:
	@echo "🏃 Running benchmarks..."
	@cargo bench --color=always

# CI pipeline tests
test-ci: setup-test test-fmt test-clippy test-audit test-unit test-integration test-coverage
	@echo "🎯 CI test pipeline completed successfully!"

# Local development tests
test-local: test-quick test-clippy
	@echo "💻 Local development tests completed!"

# Stress tests (long running)
test-stress:
	@echo "💪 Running stress tests..."
	@cargo test --test performance --color=always -- --test-threads=1 --nocapture --ignored stress_test
	@cargo test --test integration --color=always -- --test-threads=1 --nocapture --ignored stress_test

# Test environment setup
setup-test:
	@echo "🔧 Setting up test environment..."
	@mkdir -p data/test
	@mkdir -p logs/test
	@mkdir -p coverage
	@echo "CERBERUS_ENVIRONMENT=test" > .env.test
	@echo "CERBERUS_TRADING_PAPER_TRADING=true" >> .env.test
	@echo "CERBERUS_DATABASE_PATH=data/test/cerberus_test.db" >> .env.test
	@echo "RUST_LOG=debug" >> .env.test
	@echo "✅ Test environment setup complete"

# Clean test artifacts
clean-test:
	@echo "🧹 Cleaning test artifacts..."
	@rm -rf target/debug/deps/cerberus-*
	@rm -rf data/test/*
	@rm -rf logs/test/*
	@rm -rf coverage/*
	@rm -f .env.test
	@cargo clean
	@echo "✅ Test artifacts cleaned"

# Database-specific tests
test-db:
	@echo "🗄️ Running database tests..."
	@cargo test database --color=always -- --test-threads=1 --nocapture

# Cache-specific tests
test-cache:
	@echo "💾 Running cache tests..."
	@cargo test cache --color=always -- --test-threads=$(TEST_THREADS) --nocapture

# Signal processing tests
test-signals:
	@echo "📡 Running signal processing tests..."
	@cargo test signals --color=always -- --test-threads=$(TEST_THREADS) --nocapture

# Risk management tests
test-risk:
	@echo "⚖️ Running risk management tests..."
	@cargo test risk --color=always -- --test-threads=$(TEST_THREADS) --nocapture

# Trading engine tests
test-trading:
	@echo "💰 Running trading engine tests..."
	@cargo test trading --color=always -- --test-threads=$(TEST_THREADS) --nocapture

# Configuration tests
test-config:
	@echo "⚙️ Running configuration tests..."
	@cargo test config --color=always -- --test-threads=$(TEST_THREADS) --nocapture

# Error handling tests
test-errors:
	@echo "❌ Running error handling tests..."
	@cargo test errors --color=always -- --test-threads=$(TEST_THREADS) --nocapture

# Component-specific test suite
test-components: test-config test-cache test-signals test-risk test-trading test-db test-errors
	@echo "🔧 All component tests completed!"

# Memory leak detection (requires valgrind)
test-memory:
	@echo "🧠 Running memory leak detection..."
	@command -v valgrind >/dev/null 2>&1 || { echo "❌ Valgrind not installed"; exit 1; }
	@cargo build --tests
	@valgrind --tool=memcheck --leak-check=full --show-leak-kinds=all \
		target/debug/deps/cerberus-* --test-threads=1

# Test with different Rust versions
test-msrv:
	@echo "🦀 Testing with Minimum Supported Rust Version..."
	@rustup toolchain install 1.70.0
	@rustup run 1.70.0 cargo test --color=always

# Continuous testing (watch mode)
test-watch:
	@echo "👀 Starting continuous testing..."
	@command -v cargo-watch >/dev/null 2>&1 || { echo "Installing cargo-watch..."; cargo install cargo-watch; }
	@cargo watch -x "test --lib --color=always"

# Test specific module
test-module:
	@echo "🎯 Testing specific module: $(MODULE)"
	@cargo test $(MODULE) --color=always -- --nocapture

# Generate test report
test-report: test-all test-coverage
	@echo "📋 Generating comprehensive test report..."
	@echo "# Cerberus v5.0 Test Report" > test-report.md
	@echo "Generated on: $$(date)" >> test-report.md
	@echo "" >> test-report.md
	@echo "## Test Results" >> test-report.md
	@cargo test --color=never 2>&1 | grep -E "(test result:|running)" >> test-report.md || true
	@echo "" >> test-report.md
	@echo "## Coverage" >> test-report.md
	@echo "Coverage report available in coverage/tarpaulin-report.html" >> test-report.md
	@echo "📋 Test report generated: test-report.md"

# Docker test environment
test-docker:
	@echo "🐳 Running tests in Docker..."
	@docker build -t cerberus-test -f Dockerfile.test .
	@docker run --rm -v $$(pwd):/app cerberus-test make test-ci

# Test data validation
test-validate:
	@echo "✅ Validating test data and configuration..."
	@cargo run --bin validate-test-data
	@echo "✅ Test data validation complete"

# Performance regression tests
test-regression:
	@echo "📈 Running performance regression tests..."
	@cargo test --test performance --color=always -- --test-threads=1 regression
	@echo "📈 Performance regression tests complete"

# Test with sanitizers
test-sanitize:
	@echo "🧼 Running tests with sanitizers..."
	@RUSTFLAGS="-Z sanitizer=address" cargo +nightly test --target x86_64-unknown-linux-gnu
	@RUSTFLAGS="-Z sanitizer=thread" cargo +nightly test --target x86_64-unknown-linux-gnu

# Help for specific test categories
help-testing:
	@echo "Testing Best Practices for Cerberus v5.0"
	@echo "========================================"
	@echo ""
	@echo "1. Always run 'make test-local' before committing"
	@echo "2. Use 'make test-quick' during development"
	@echo "3. Run 'make test-all' before creating PRs"
	@echo "4. Check coverage with 'make test-coverage'"
	@echo "5. Use 'make test-watch' for continuous testing"
	@echo ""
	@echo "Test Environment Variables:"
	@echo "  TEST_THREADS=$(TEST_THREADS)"
	@echo "  COVERAGE_THRESHOLD=$(COVERAGE_THRESHOLD)%"
	@echo "  PERFORMANCE_TIMEOUT=$(PERFORMANCE_TIMEOUT)s"
	@echo ""
	@echo "For component-specific tests:"
	@echo "  make test-module MODULE=signals"
	@echo "  make test-module MODULE=risk"
	@echo "  make test-module MODULE=trading"

# Default test target
test: test-local
