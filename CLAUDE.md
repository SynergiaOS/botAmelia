# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Cerberus v5.0** - High-performance Rust trading bot for aggressive memecoin leverage trading on Solana. Built with production-ready architecture emphasizing safety, observability, and modular design.

## Essential Commands

### Build & Run
```bash
cargo build --release
cargo run --release

# Paper trading mode (safe testing - default enabled)
PAPER_TRADING=true cargo run --release
```

### Testing (comprehensive Makefile available)
```bash
make test-quick          # Fast development tests
make test-all           # Full test suite  
make test-ci            # CI pipeline tests
make test-coverage      # Coverage report
make test-watch         # Watch mode
make setup-test         # Test environment setup
```

### Development
```bash
cp .env.example .env    # Setup environment
cargo check             # Quick compilation check
```

## Architecture

### Core Module Structure
- **`config/`** - TOML + environment variable configuration (hierarchical override)
- **`trading/`** - Core trading engine and execution
- **`risk/`** - Risk management, stop-loss, circuit breakers
- **`database/`** - SQLite with WAL mode, async sqlx
- **`monitoring/`** - Metrics and observability (Sentry + Prometheus)
- **`alerts/`** - Multi-channel notifications (Telegram, Discord, Email)
- **`api/`** - Axum REST API for Kestra orchestration
- **`security/`** - Cryptographic operations and key management

### Configuration Pattern
Primary config in `config/config.toml` with `CERBERUS_` prefixed environment variable overrides. All config structs have validation methods.

### Tech Stack
- **Runtime**: Tokio async throughout
- **Database**: SQLite + sqlx (async)  
- **HTTP**: Axum + Tower ecosystem
- **Observability**: Tracing + Sentry + Prometheus
- **Security**: Ring crypto, Argon2, AES-GCM encryption

## Safety Features

- **Paper trading mode** enabled by default
- **Circuit breakers** and emergency stops
- **Encrypted private key storage** 
- **Graceful shutdown handling**
- **Comprehensive error handling** with custom `CerberusError` enum

## Development Requirements

- **Rust 1.70+**
- **SQLite development libraries**
- **OpenSSL headers**
- **Burner wallet only** - never use main wallet for development

## Key Patterns

- Use `anyhow::Result<T>` for error handling
- Configuration via builder pattern with validation
- Structured logging with `tracing` crate
- Modular architecture allows independent component testing
- Environment-specific configs (dev/staging/production)