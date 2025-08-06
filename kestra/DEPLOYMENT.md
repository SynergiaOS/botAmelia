# 🚀 Deployment Guide - Kestra + Cerberus Integration

Przewodnik wdrożenia integracji Kestra z Cerberus Trading Bot w środowisku produkcyjnym.

## 📋 Spis Treści

- [Wymagania](#wymagania)
- [Przygotowanie Środowiska](#przygotowanie-środowiska)
- [Konfiguracja Sekretów](#konfiguracja-sekretów)
- [Deployment](#deployment)
- [Weryfikacja](#weryfikacja)
- [Monitoring](#monitoring)
- [Backup i Recovery](#backup-i-recovery)
- [Troubleshooting](#troubleshooting)

## 🔧 Wymagania

### Minimalne Wymagania Systemowe

- **CPU**: 2 cores (4 cores zalecane)
- **RAM**: 4GB (8GB zalecane)
- **Dysk**: 20GB wolnego miejsca (SSD zalecane)
- **Network**: Stabilne połączenie internetowe
- **OS**: Linux (Ubuntu 20.04+ zalecane)

### Wymagane Oprogramowanie

```bash
# Docker & Docker Compose
sudo apt update
sudo apt install docker.io docker-compose
sudo usermod -aG docker $USER

# Narzędzia pomocnicze
sudo apt install curl jq bc

# Restart sesji po dodaniu do grupy docker
newgrp docker
```

### Porty

Upewnij się, że następujące porty są dostępne:

- **8080**: Cerberus API
- **8081**: Kestra UI
- **9090**: Cerberus Metrics
- **9091**: Prometheus
- **3000**: Grafana
- **5432**: PostgreSQL (internal)

## 🏗️ Przygotowanie Środowiska

### 1. Klonowanie Repozytorium

```bash
git clone <repository-url>
cd BotAmelia
```

### 2. Sprawdzenie Konfiguracji

```bash
# Sprawdź czy Cerberus jest skonfigurowany
ls -la config/
cat config/config.toml

# Sprawdź strukturę Kestra
ls -la kestra/
```

### 3. Przygotowanie Katalogów

```bash
# Utwórz katalogi dla danych
mkdir -p data/kestra
mkdir -p data/postgres
mkdir -p logs

# Ustaw uprawnienia
chmod 755 data/kestra data/postgres logs
```

## 🔐 Konfiguracja Sekretów

### 1. Telegram Bot

**Krok 1**: Utwórz bota Telegram
```bash
# 1. Napisz do @BotFather na Telegram
# 2. Użyj komendy /newbot
# 3. Podaj nazwę i username bota
# 4. Zapisz otrzymany token
```

**Krok 2**: Otrzymaj Chat ID
```bash
# 1. Dodaj bota do grupy/kanału
# 2. Napisz do @userinfobot
# 3. Przekaż wiadomość od bota
# 4. Zapisz chat_id
```

### 2. Konfiguracja Sekretów

```bash
# Skopiuj template
cp kestra/secrets.env.example kestra/secrets.env

# Edytuj plik
nano kestra/secrets.env
```

**Przykład konfiguracji**:
```env
# Telegram (WYMAGANE)
TELEGRAM_BOT_TOKEN=1234567890:ABCdefGHIjklMNOpqrsTUVwxyz
TELEGRAM_CHAT_ID=-1001234567890

# Discord (opcjonalne)
DISCORD_WEBHOOK_URL=https://discord.com/api/webhooks/...

# Slack (opcjonalne)
SLACK_WEBHOOK_URL=https://hooks.slack.com/services/...

# Email (opcjonalne)
SMTP_SERVER=smtp.gmail.com
SMTP_PORT=587
SMTP_USERNAME=your-email@gmail.com
SMTP_PASSWORD=your-app-password
EMAIL_FROM=your-email@gmail.com
EMAIL_TO=alerts@yourdomain.com
```

### 3. Zabezpieczenie Sekretów

```bash
# Ustaw odpowiednie uprawnienia
chmod 600 kestra/secrets.env

# Dodaj do .gitignore (jeśli nie ma)
echo "kestra/secrets.env" >> .gitignore
```

## 🚀 Deployment

### 1. Deployment Podstawowy (Tylko Kestra)

```bash
# Uruchomienie z Kestra
docker-compose --profile orchestration up -d

# Sprawdzenie statusu
docker-compose ps
```

### 2. Deployment Pełny (z Monitoringiem)

```bash
# Uruchomienie pełnego stacku
docker-compose --profile orchestration --profile monitoring up -d

# Sprawdzenie wszystkich serwisów
docker-compose ps
```

### 3. Deployment Produkcyjny

```bash
# Ustawienie zmiennych środowiskowych
export COMPOSE_PROJECT_NAME=cerberus-prod
export CERBERUS_ENVIRONMENT=production

# Uruchomienie z restart policy
docker-compose --profile orchestration --profile monitoring up -d --restart unless-stopped

# Sprawdzenie logów
docker-compose logs -f --tail=100
```

## ✅ Weryfikacja

### 1. Automatyczna Weryfikacja

```bash
# Uruchomienie testów integracyjnych
./scripts/test-kestra-integration.sh
```

### 2. Manualna Weryfikacja

**Sprawdzenie Serwisów**:
```bash
# Health checks
curl http://localhost:8080/health
curl http://localhost:8081/api/v1/flows

# Sprawdzenie metryk
curl http://localhost:8080/metrics
curl http://localhost:9091/metrics
```

**Sprawdzenie UI**:
- Kestra UI: http://localhost:8081
- Grafana: http://localhost:3000 (admin/admin)

**Test Workflow'u**:
1. Otwórz Kestra UI
2. Przejdź do namespace `cerberus.system`
3. Uruchom `cerberus-system-management` z parametrem `operation_type: health_check`
4. Sprawdź logi wykonania

### 3. Test Alertów

```bash
# Test powiadomienia Telegram
curl -X POST http://localhost:8081/api/v1/executions/cerberus.alerting/cerberus-advanced-alerting \
  -H "Content-Type: application/json" \
  -d '{"inputs": {"alert_severity": "warning", "send_notifications": true}}'
```

## 📊 Monitoring

### 1. Kestra Monitoring

**Metryki do Monitorowania**:
- Execution success rate
- Average execution time
- Failed executions
- Queue size

**Dostęp**:
- UI: http://localhost:8081
- API: http://localhost:8081/api/v1/stats

### 2. Cerberus Monitoring

**Metryki do Monitorowania**:
- Trading success rate
- System resources (CPU, RAM)
- API response times
- Database performance

**Dostęp**:
- API: http://localhost:8080/metrics
- Prometheus: http://localhost:9091
- Grafana: http://localhost:3000

### 3. Alerty

**Konfiguracja Alertów**:
```bash
# Sprawdź konfigurację alertów
docker-compose exec kestra cat /app/flows/cerberus-advanced-alerting.yaml

# Test alertów
curl -X POST http://localhost:8081/api/v1/executions/cerberus.alerting/cerberus-advanced-alerting
```

## 💾 Backup i Recovery

### 1. Backup Automatyczny

Kestra automatycznie wykonuje backup:
- Codziennie o 2:00 (workflow: `cerberus-system-management`)
- Przed każdym restartem systemu

### 2. Backup Manualny

```bash
# Backup bazy danych Cerberus
docker-compose exec cerberus curl -X POST http://localhost:8080/api/system/backup

# Backup konfiguracji Kestra
docker-compose exec postgres pg_dump -U kestra kestra > backup_kestra_$(date +%Y%m%d_%H%M%S).sql

# Backup workflow'ów
tar -czf workflows_backup_$(date +%Y%m%d_%H%M%S).tar.gz kestra/flows/
```

### 3. Recovery

```bash
# Restore Cerberus database
# (implementacja zależy od struktury backup w Cerberus)

# Restore Kestra database
docker-compose exec -T postgres psql -U kestra kestra < backup_kestra_YYYYMMDD_HHMMSS.sql

# Restore workflows
tar -xzf workflows_backup_YYYYMMDD_HHMMSS.tar.gz
docker-compose restart kestra
```

## 🔧 Troubleshooting

### Częste Problemy

#### 1. Kestra nie startuje

```bash
# Sprawdź logi
docker-compose logs kestra

# Sprawdź PostgreSQL
docker-compose logs postgres

# Sprawdź połączenie sieciowe
docker network inspect botamelia_cerberus-net

# Restart
docker-compose restart postgres kestra
```

#### 2. Workflow'y nie działają

```bash
# Sprawdź namespace
curl http://localhost:8081/api/v1/flows | jq '.[] | .namespace' | sort | uniq

# Sprawdź składnię workflow'u
docker-compose exec kestra kestra flow validate /app/flows/cerberus-trading-pipeline.yaml

# Sprawdź logi execution
# (w Kestra UI -> Executions)
```

#### 3. Brak powiadomień

```bash
# Test Telegram API
curl -X POST "https://api.telegram.org/bot$TELEGRAM_BOT_TOKEN/sendMessage" \
  -H "Content-Type: application/json" \
  -d "{\"chat_id\": \"$TELEGRAM_CHAT_ID\", \"text\": \"Test message\"}"

# Sprawdź sekrety w Kestra
docker-compose exec kestra env | grep TELEGRAM
```

#### 4. Wysokie użycie zasobów

```bash
# Sprawdź użycie zasobów
docker stats

# Sprawdź logi błędów
docker-compose logs | grep -i error

# Optymalizacja
# - Zwiększ limity pamięci w docker-compose.yml
# - Zmniejsz częstotliwość workflow'ów
# - Wyłącz niepotrzebne triggery
```

### Logi i Diagnostyka

```bash
# Wszystkie logi
docker-compose logs -f

# Logi z ostatniej godziny
docker-compose logs --since 1h

# Logi konkretnego serwisu
docker-compose logs -f kestra
docker-compose logs -f cerberus

# Eksport logów
docker-compose logs > logs/deployment_$(date +%Y%m%d_%H%M%S).log
```

### Performance Tuning

```bash
# Zwiększenie limitów pamięci (docker-compose.yml)
services:
  kestra:
    mem_limit: 2g
    environment:
      JAVA_OPTS: "-Xmx1g"

# Optymalizacja PostgreSQL
services:
  postgres:
    environment:
      POSTGRES_SHARED_BUFFERS: "256MB"
      POSTGRES_EFFECTIVE_CACHE_SIZE: "1GB"
```

## 📈 Scaling

### Horizontal Scaling

```bash
# Dodanie worker nodes (przyszła funkcjonalność)
docker-compose up -d --scale kestra-worker=3
```

### Vertical Scaling

```bash
# Zwiększenie zasobów w docker-compose.yml
services:
  kestra:
    deploy:
      resources:
        limits:
          cpus: '2.0'
          memory: 4G
        reservations:
          cpus: '1.0'
          memory: 2G
```

## 🔄 Maintenance

### Regularne Zadania

1. **Codziennie**:
   - Sprawdzenie logów błędów
   - Weryfikacja backup'ów
   - Monitoring alertów

2. **Tygodniowo**:
   - Aktualizacja obrazów Docker
   - Czyszczenie starych logów
   - Przegląd metryk wydajności

3. **Miesięcznie**:
   - Aktualizacja systemu operacyjnego
   - Przegląd konfiguracji
   - Test procedur recovery

### Aktualizacje

```bash
# Aktualizacja obrazów
docker-compose pull

# Restart z nowymi obrazami
docker-compose up -d

# Weryfikacja po aktualizacji
./scripts/test-kestra-integration.sh
```

## 📞 Wsparcie

W przypadku problemów:

1. Sprawdź logi: `docker-compose logs`
2. Uruchom testy: `./scripts/test-kestra-integration.sh`
3. Sprawdź dokumentację: `kestra/README.md`
4. Utwórz issue z logami i opisem problemu

---

**✅ Po zakończeniu deployment'u system jest gotowy do produkcyjnego użycia!**
