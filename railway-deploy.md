# 🚀 Railway Deployment Guide - Cerberus v4.0

## Quick Deploy

### 1. Connect Repository
```bash
# Install Railway CLI
npm install -g @railway/cli

# Login to Railway
railway login

# Link project
railway link
```

### 2. Environment Variables
Set these in Railway dashboard:

```bash
# Required
DATABASE_URL=sqlite:///app/data/cerberus.db
RUST_LOG=info
PORT=8080

# Optional - Monitoring
SENTRY_DSN=your_sentry_dsn_here
PROMETHEUS_ENABLED=true

# Optional - Alerts  
TELEGRAM_BOT_TOKEN=your_telegram_token
DISCORD_WEBHOOK_URL=your_discord_webhook
```

### 3. Deploy
```bash
# Deploy current branch
railway up

# Or connect GitHub for auto-deploy
railway connect
```

## Railway Configuration

### Build Settings
- **Builder:** Nixpacks (automatic Rust detection)
- **Build Command:** `cargo build --release`
- **Start Command:** `./target/release/cerberus-bot`

### Health Check
- **Path:** `/health`
- **Timeout:** 300s
- **Expected Response:** 200 OK

### Resources
- **Memory:** 512MB (minimum)
- **CPU:** 0.5 vCPU (minimum)
- **Storage:** 1GB (for SQLite database)

## Post-Deploy Verification

### 1. Check Health
```bash
curl https://your-app.railway.app/health
```

### 2. Test API
```bash
# Create signal
curl -X POST https://your-app.railway.app/api/signals \
  -H "Content-Type: application/json" \
  -d '{"token":"PEPE","source":"test","confidence":"high","price":0.1,"volume":1000}'

# Get signals
curl https://your-app.railway.app/api/signals
```

### 3. Monitor Metrics
```bash
curl https://your-app.railway.app/metrics
```

## Scaling

### Horizontal Scaling
- Railway supports automatic scaling
- Configure in Railway dashboard
- Monitor CPU/Memory usage

### Database Considerations
- SQLite works for single instance
- For multi-instance, consider PostgreSQL addon

## Monitoring

### Railway Metrics
- Built-in CPU, Memory, Network monitoring
- Logs accessible via Railway CLI: `railway logs`

### Application Metrics
- Prometheus metrics at `/metrics`
- Health status at `/health`
- Structured JSON logs

## Troubleshooting

### Common Issues
1. **Build fails:** Check Rust version in nixpacks.toml
2. **Health check fails:** Verify PORT environment variable
3. **Database errors:** Check file permissions and storage

### Debug Commands
```bash
# View logs
railway logs

# Connect to shell
railway shell

# Check environment
railway variables
```

## Production Checklist

- [ ] Environment variables configured
- [ ] Health check responding
- [ ] Metrics endpoint working
- [ ] Database persisting data
- [ ] Logs structured and readable
- [ ] Monitoring alerts configured

## Next Steps

After successful deployment:
1. Set up monitoring dashboards
2. Configure alerting
3. Implement trading logic (Phase 2)
4. Add exchange integrations

**Cerberus v4.0 is Railway-ready! 🚀**
