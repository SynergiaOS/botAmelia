# 📚 Kestra + Cerberus - Przykłady Użycia

Praktyczne przykłady użycia integracji Kestra z Cerberus Trading Bot.

## 🎯 Spis Treści

- [Podstawowe Operacje](#podstawowe-operacje)
- [Zarządzanie Workflow'ami](#zarządzanie-workflowami)
- [Monitoring i Alerty](#monitoring-i-alerty)
- [Zarządzanie Pozycjami](#zarządzanie-pozycjami)
- [Analityka i Raporty](#analityka-i-raporty)
- [Operacje Systemowe](#operacje-systemowe)
- [Integracje Zewnętrzne](#integracje-zewnętrzne)

## 🚀 Podstawowe Operacje

### Sprawdzenie Stanu Systemu

```bash
# Przez API Cerberus
curl http://localhost:8080/health | jq

# Przez Kestra workflow
curl -X POST http://localhost:8081/api/v1/executions/cerberus.system/cerberus-system-management \
  -H "Content-Type: application/json" \
  -d '{"inputs": {"operation_type": "health_check"}}'
```

### Uruchomienie Trading Pipeline

```bash
# Manualnie z domyślnymi parametrami
curl -X POST http://localhost:8081/api/v1/executions/cerberus.trading/cerberus-trading-pipeline

# Z custom parametrami
curl -X POST http://localhost:8081/api/v1/executions/cerberus.trading/cerberus-trading-pipeline \
  -H "Content-Type: application/json" \
  -d '{
    "inputs": {
      "risk_threshold": 10.0,
      "max_leverage": 20,
      "force_execution": false
    }
  }'
```

### Sprawdzenie Metryk

```bash
# Metryki JSON
curl http://localhost:8080/metrics | jq

# Metryki Prometheus
curl http://localhost:8080/metrics/prometheus

# Szczegółowe metryki systemu
curl http://localhost:8080/health/detailed | jq
```

## 🔄 Zarządzanie Workflow'ami

### Lista Wszystkich Workflow'ów

```bash
# Wszystkie workflow'y
curl http://localhost:8081/api/v1/flows | jq '.[] | {namespace, id, description}'

# Workflow'y Cerberus
curl http://localhost:8081/api/v1/flows | jq '.[] | select(.namespace | startswith("cerberus"))'
```

### Sprawdzenie Statusu Execution

```bash
# Lista ostatnich executions
curl http://localhost:8081/api/v1/executions | jq '.results[0:5]'

# Szczegóły konkretnego execution
EXECUTION_ID="your-execution-id"
curl http://localhost:8081/api/v1/executions/$EXECUTION_ID | jq
```

### Włączanie/Wyłączanie Triggerów

```bash
# Wyłączenie automatycznego tradingu
curl -X PUT http://localhost:8081/api/v1/flows/cerberus.trading/cerberus-trading-pipeline \
  -H "Content-Type: application/json" \
  -d '{
    "triggers": [
      {
        "id": "trading-hours-trigger",
        "disabled": true
      }
    ]
  }'

# Włączenie z powrotem
curl -X PUT http://localhost:8081/api/v1/flows/cerberus.trading/cerberus-trading-pipeline \
  -H "Content-Type: application/json" \
  -d '{
    "triggers": [
      {
        "id": "trading-hours-trigger",
        "disabled": false
      }
    ]
  }'
```

## 🚨 Monitoring i Alerty

### Test Alertów

```bash
# Test alertu ostrzegawczego
curl -X POST http://localhost:8081/api/v1/executions/cerberus.alerting/cerberus-advanced-alerting \
  -H "Content-Type: application/json" \
  -d '{
    "inputs": {
      "alert_severity": "warning",
      "check_anomalies": true,
      "send_notifications": true
    }
  }'

# Test alertu krytycznego
curl -X POST http://localhost:8081/api/v1/executions/cerberus.alerting/cerberus-advanced-alerting \
  -H "Content-Type: application/json" \
  -d '{
    "inputs": {
      "alert_severity": "critical",
      "send_notifications": true
    }
  }'
```

### Monitoring Ciągły

```bash
# Uruchomienie monitoringu z custom parametrami
curl -X POST http://localhost:8081/api/v1/executions/cerberus.monitoring/cerberus-monitoring \
  -H "Content-Type: application/json" \
  -d '{
    "inputs": {
      "alert_threshold_cpu": 85.0,
      "alert_threshold_memory": 600.0,
      "alert_threshold_success_rate": 0.7
    }
  }'
```

### Sprawdzenie Alertów w Systemie

```bash
# Ostatnie alerty (jeśli endpoint istnieje)
curl http://localhost:8080/api/alerts/recent | jq

# Status alertów
curl http://localhost:8080/api/risk/status | jq
```

## 💼 Zarządzanie Pozycjami

### Monitoring Pozycji

```bash
# Sprawdzenie aktualnych pozycji
curl http://localhost:8080/api/positions | jq

# Monitoring przez Kestra
curl -X POST http://localhost:8081/api/v1/executions/cerberus.positions/cerberus-position-management \
  -H "Content-Type: application/json" \
  -d '{
    "inputs": {
      "action": "monitor",
      "max_loss_threshold": -5.0
    }
  }'
```

### Zamykanie Pozycji

```bash
# Zamknięcie stratnych pozycji
curl -X POST http://localhost:8081/api/v1/executions/cerberus.positions/cerberus-position-management \
  -H "Content-Type: application/json" \
  -d '{
    "inputs": {
      "action": "close_losing",
      "force_close": false,
      "max_loss_threshold": -8.0
    }
  }'

# Emergency close wszystkich pozycji
curl -X POST http://localhost:8081/api/v1/executions/cerberus.positions/cerberus-position-management \
  -H "Content-Type: application/json" \
  -d '{
    "inputs": {
      "action": "emergency_close",
      "force_close": true
    }
  }'
```

### Zamknięcie Konkretnej Pozycji

```bash
# Przez API Cerberus
POSITION_ID="position-123"
curl -X POST http://localhost:8080/api/positions/$POSITION_ID/close \
  -H "Content-Type: application/json" \
  -d '{"reason": "Manual close via API"}'
```

## 📈 Analityka i Raporty

### Generowanie Raportów

```bash
# Raport dzienny
curl -X POST http://localhost:8081/api/v1/executions/cerberus.analytics/cerberus-analytics \
  -H "Content-Type: application/json" \
  -d '{
    "inputs": {
      "analysis_period_days": 1,
      "generate_charts": true,
      "send_report": true
    }
  }'

# Raport tygodniowy
curl -X POST http://localhost:8081/api/v1/executions/cerberus.analytics/cerberus-analytics \
  -H "Content-Type: application/json" \
  -d '{
    "inputs": {
      "analysis_period_days": 7,
      "generate_charts": true,
      "send_report": false
    }
  }'

# Raport miesięczny z wysłaniem
curl -X POST http://localhost:8081/api/v1/executions/cerberus.analytics/cerberus-analytics \
  -H "Content-Type: application/json" \
  -d '{
    "inputs": {
      "analysis_period_days": 30,
      "generate_charts": true,
      "send_report": true
    }
  }'
```

### Pobieranie Danych Historycznych

```bash
# Transakcje z ostatnich 7 dni
curl "http://localhost:8080/api/trades?from=$(date -d '7 days ago' +%s)&to=$(date +%s)" | jq

# Sygnały z ostatnich 24 godzin
curl "http://localhost:8080/api/signals?from=$(date -d '1 day ago' +%s)&to=$(date +%s)" | jq
```

## ⚙️ Operacje Systemowe

### Backup

```bash
# Backup manualny
curl -X POST http://localhost:8081/api/v1/executions/cerberus.system/cerberus-system-management \
  -H "Content-Type: application/json" \
  -d '{
    "inputs": {
      "operation_type": "backup",
      "force_operation": false
    }
  }'

# Backup przez API Cerberus
curl -X POST http://localhost:8080/api/system/backup
```

### Maintenance Mode

```bash
# Włączenie maintenance na 30 minut
curl -X POST http://localhost:8081/api/v1/executions/cerberus.system/cerberus-system-management \
  -H "Content-Type: application/json" \
  -d '{
    "inputs": {
      "operation_type": "maintenance",
      "force_operation": false,
      "maintenance_duration_minutes": 30
    }
  }'

# Wymuszenie maintenance pomimo aktywnych pozycji
curl -X POST http://localhost:8081/api/v1/executions/cerberus.system/cerberus-system-management \
  -H "Content-Type: application/json" \
  -d '{
    "inputs": {
      "operation_type": "maintenance",
      "force_operation": true,
      "maintenance_duration_minutes": 15
    }
  }'
```

### Emergency Stop

```bash
# Emergency stop przez Kestra
curl -X POST http://localhost:8081/api/v1/executions/cerberus.system/cerberus-system-management \
  -H "Content-Type: application/json" \
  -d '{
    "inputs": {
      "operation_type": "emergency_stop",
      "force_operation": true
    }
  }'

# Emergency stop przez API Cerberus
curl -X POST http://localhost:8080/api/system/emergency-stop
```

### Restart Systemu

```bash
# Restart z backup
curl -X POST http://localhost:8081/api/v1/executions/cerberus.system/cerberus-system-management \
  -H "Content-Type: application/json" \
  -d '{
    "inputs": {
      "operation_type": "restart",
      "force_operation": false
    }
  }'
```

## 🔗 Integracje Zewnętrzne

### Webhook dla Sygnałów Zewnętrznych

```bash
# Wysłanie sygnału przez webhook
curl -X POST http://localhost:8081/api/v1/webhooks/cerberus-trading-signal \
  -H "Content-Type: application/json" \
  -d '{
    "token": "DOGE",
    "source": "external_provider",
    "confidence": "High",
    "price": 0.08,
    "volume": 50000000,
    "metadata": {
      "signal_strength": 0.85,
      "market_sentiment": "bullish"
    }
  }'
```

### Webhook dla Alertów Zewnętrznych

```bash
# Wysłanie alertu zewnętrznego
curl -X POST http://localhost:8081/api/v1/webhooks/cerberus-external-alert \
  -H "Content-Type: application/json" \
  -d '{
    "severity": "warning",
    "message": "External system detected anomaly",
    "source": "external_monitor",
    "timestamp": "'$(date -u +%Y-%m-%dT%H:%M:%SZ)'"
  }'
```

### Integracja z Telegram

```bash
# Test bezpośredni Telegram API
TELEGRAM_BOT_TOKEN="your-bot-token"
TELEGRAM_CHAT_ID="your-chat-id"

curl -X POST "https://api.telegram.org/bot$TELEGRAM_BOT_TOKEN/sendMessage" \
  -H "Content-Type: application/json" \
  -d "{
    \"chat_id\": \"$TELEGRAM_CHAT_ID\",
    \"text\": \"🤖 Test message from Cerberus\",
    \"parse_mode\": \"Markdown\"
  }"
```

## 📊 Przykłady Skryptów Automatyzacji

### Skrypt Monitoringu

```bash
#!/bin/bash
# monitor.sh - Prosty skrypt monitoringu

# Sprawdzenie stanu systemu
HEALTH=$(curl -s http://localhost:8080/health | jq -r '.success')

if [ "$HEALTH" != "true" ]; then
    echo "❌ System unhealthy - triggering alert workflow"
    curl -X POST http://localhost:8081/api/v1/executions/cerberus.alerting/cerberus-advanced-alerting \
      -H "Content-Type: application/json" \
      -d '{"inputs": {"alert_severity": "critical", "send_notifications": true}}'
else
    echo "✅ System healthy"
fi
```

### Skrypt Backup

```bash
#!/bin/bash
# backup.sh - Automatyczny backup

echo "🔄 Starting backup process..."

# Trigger backup workflow
EXECUTION_ID=$(curl -s -X POST http://localhost:8081/api/v1/executions/cerberus.system/cerberus-system-management \
  -H "Content-Type: application/json" \
  -d '{"inputs": {"operation_type": "backup"}}' | jq -r '.id')

echo "📦 Backup started with execution ID: $EXECUTION_ID"

# Wait for completion (simplified)
sleep 60

# Check result
STATUS=$(curl -s http://localhost:8081/api/v1/executions/$EXECUTION_ID | jq -r '.state.current')
echo "📊 Backup status: $STATUS"
```

### Skrypt Analizy Wydajności

```bash
#!/bin/bash
# performance.sh - Analiza wydajności

echo "📈 Generating performance report..."

# Trigger analytics workflow
curl -X POST http://localhost:8081/api/v1/executions/cerberus.analytics/cerberus-analytics \
  -H "Content-Type: application/json" \
  -d '{
    "inputs": {
      "analysis_period_days": 7,
      "generate_charts": true,
      "send_report": true
    }
  }'

echo "✅ Performance analysis started"
```

## 🔧 Debugging i Troubleshooting

### Sprawdzenie Logów Execution

```bash
# Ostatnie executions z błędami
curl http://localhost:8081/api/v1/executions?state=FAILED | jq '.results[0:5]'

# Logi konkretnego execution
EXECUTION_ID="your-execution-id"
curl http://localhost:8081/api/v1/executions/$EXECUTION_ID/logs | jq
```

### Test Połączenia

```bash
# Test wszystkich endpointów
echo "Testing Cerberus..."
curl -f http://localhost:8080/health || echo "❌ Cerberus failed"

echo "Testing Kestra..."
curl -f http://localhost:8081/api/v1/flows || echo "❌ Kestra failed"

echo "Testing Prometheus..."
curl -f http://localhost:9091/metrics || echo "❌ Prometheus failed"

echo "Testing Grafana..."
curl -f http://localhost:3000/api/health || echo "❌ Grafana failed"
```

### Restart Workflow'ów

```bash
# Restart konkretnego workflow'u (przez ponowne załadowanie)
curl -X PUT http://localhost:8081/api/v1/flows/cerberus.trading/cerberus-trading-pipeline \
  -H "Content-Type: application/json" \
  -d @kestra/flows/cerberus-trading-pipeline.yaml
```

---

**💡 Tip**: Wszystkie powyższe przykłady można zautomatyzować i włączyć do własnych skryptów monitoringu i zarządzania systemem.
