# 🚀 Cerberus v5.0 - Complete Setup Guide

This guide will walk you through setting up Cerberus from scratch to your first trade.

## 📋 Prerequisites

### System Requirements
- **OS**: Linux, macOS, or Windows (WSL2 recommended)
- **RAM**: Minimum 4GB, recommended 8GB+
- **Storage**: 10GB free space
- **Network**: Stable internet connection (low latency preferred)

### Software Requirements
- **Rust**: 1.70 or later
- **Git**: Latest version
- **Docker**: Optional, for containerized deployment
- **Node.js**: 18+ (for MCP Context7 support)

## 🛠️ Installation

### Step 1: Install Rust

```bash
# Install Rust via rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Verify installation
rustc --version
cargo --version
```

### Step 2: Clone Repository

```bash
git clone https://github.com/SynergiaOS/BotAmelia.git
cd BotAmelia
```

### Step 3: Environment Setup

```bash
# Copy environment template
cp .env.example .env

# Edit with your configuration
nano .env
```

### Step 4: Configure Wallet

⚠️ **SECURITY WARNING**: Use a burner wallet, not your main wallet!

```bash
# Generate a new Solana wallet (recommended)
solana-keygen new --outfile ~/.config/solana/cerberus-wallet.json

# Or use existing wallet
# Copy your private key to .env as SOLANA_PRIVATE_KEY
```

### Step 5: API Keys Setup

#### Required APIs (Free Tiers Available):

1. **Birdeye API** (Free tier: 100 requests/minute)
   - Visit: https://birdeye.so/
   - Sign up and get API key
   - Add to `.env`: `BIRDEYE_API_KEY=your_key`

2. **Gemini API** (Free tier: 1000 requests/day)
   - Visit: https://ai.google.dev/
   - Get API key for Gemini Flash
   - Add to `.env`: `GEMINI_API_KEY=your_key`

3. **Sentry** (Free tier: 5k errors/month)
   - Visit: https://sentry.io/
   - Create Rust project
   - Add to `.env`: `SENTRY_DSN=your_dsn`

#### Optional APIs:

4. **Telegram Bot** (Free)
   - Message @BotFather on Telegram
   - Create new bot: `/newbot`
   - Add to `.env`: `TELEGRAM_BOT_TOKEN=your_token`
   - Get chat ID: Message your bot, then visit: `https://api.telegram.org/bot<TOKEN>/getUpdates`

5. **DexScreener** (Free tier available)
   - Visit: https://dexscreener.com/
   - Optional for additional data

### Step 6: Configuration

Edit `config/config.toml`:

```toml
# Start with paper trading!
[trading]
initial_balance = 50.0
paper_trading = true  # KEEP THIS TRUE INITIALLY
max_leverage = 10     # Start conservative

[risk]
max_daily_loss = 5.0  # Start with $5 max loss
max_position_loss_percent = 0.20  # 20% stop loss

[alerts]
enabled = true

[alerts.telegram]
enabled = true  # If you set up Telegram
```

## 🧪 Testing Setup

### Step 1: Build and Test

```bash
# Build the project
cargo build --release

# Run tests
cargo test

# Check if everything compiles
cargo check
```

### Step 2: Paper Trading Test

```bash
# Start in paper trading mode
PAPER_TRADING=true cargo run --release
```

You should see:
```
🚀 Cerberus Starting with $50.0
📊 Paper Trading Mode: ENABLED
✅ All systems operational
```

### Step 3: Validate Configuration

```bash
# Test API connections
cargo run --bin test-apis

# Test wallet connection
cargo run --bin test-wallet

# Test database
cargo run --bin test-db
```

## 🔧 Advanced Configuration

### Performance Tuning

```toml
# config/config.toml
[monitoring]
metrics_interval = 5  # Faster metrics collection
enable_performance_monitoring = true

[database]
cache_size = 128000  # 128MB cache for better performance
enable_wal = true    # Write-Ahead Logging for speed
```

### Security Hardening

```bash
# Encrypt private key
cargo run --bin encrypt-key -- --input wallet.json --output encrypted.key

# Set restrictive permissions
chmod 600 .env
chmod 600 config/config.toml
```

### Monitoring Setup

```bash
# Start with monitoring stack
docker-compose --profile monitoring up -d

# Access Grafana
open http://localhost:3000
# Login: admin/admin
```

## 🚨 Pre-Production Checklist

Before going live with real money:

### Security Checklist
- [ ] Using burner wallet (not main wallet)
- [ ] Private keys encrypted
- [ ] Environment variables secured
- [ ] 2FA enabled on all accounts
- [ ] Backup seed phrase stored securely

### Configuration Checklist
- [ ] Paper trading tested successfully
- [ ] All APIs working
- [ ] Alerts configured and tested
- [ ] Stop losses configured
- [ ] Daily loss limits set
- [ ] Monitoring working

### Testing Checklist
- [ ] All unit tests pass
- [ ] Integration tests pass
- [ ] Paper trading for 24+ hours
- [ ] Alert system tested
- [ ] Emergency shutdown tested

## 🎯 Going Live

### Step 1: Final Configuration

```toml
# config/config.toml
[trading]
paper_trading = false  # ONLY CHANGE AFTER THOROUGH TESTING
initial_balance = 50.0  # Your actual starting amount
```

### Step 2: Start Small

```bash
# Start with minimal amount
INITIAL_BALANCE=10.0 cargo run --release
```

### Step 3: Monitor Closely

- Watch Telegram alerts
- Check Sentry dashboard
- Monitor system metrics
- Be ready to emergency stop

## 🆘 Emergency Procedures

### Emergency Stop
```bash
# Kill all trading immediately
pkill -f cerberus

# Or use emergency endpoint
curl -X POST http://localhost:8080/emergency/stop
```

### Recovery
```bash
# Check system status
cargo run --bin status

# Recover from backup
cargo run --bin recover -- --backup-file data/backups/latest.db
```

## 📞 Support

If you encounter issues:

1. **Check logs**: `tail -f logs/cerberus.log`
2. **Verify configuration**: `cargo run --bin validate-config`
3. **Test APIs**: `cargo run --bin test-apis`
4. **GitHub Issues**: [Report bugs](https://github.com/SynergiaOS/BotAmelia/issues)
5. **Telegram**: [@CerberusTrading](https://t.me/CerberusTrading)

## ⚠️ Final Warning

**Remember**: This is experimental software with extreme risk. Only use money you can afford to lose completely. Start with paper trading and very small amounts.

**Good luck! 🚀**
