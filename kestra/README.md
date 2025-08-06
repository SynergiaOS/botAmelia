# 🔄 Kestra Integration for Cerberus Trading Bot

Integracja platformy orkiestracji Kestra z botem tradingowym Cerberus, umożliwiająca zaawansowane zarządzanie workflow'ami, monitoringiem i automatyzacją procesów tradingowych.

## 📋 Spis Treści

- [Przegląd](#przegląd)
- [Architektura](#architektura)
- [Instalacja](#instalacja)
- [Konfiguracja](#konfiguracja)
- [Workflow'y](#workflows)
- [Monitoring](#monitoring)
- [API](#api)
- [Testowanie](#testowanie)
- [Troubleshooting](#troubleshooting)

## 🎯 Przegląd

### Funkcjonalności

- **🤖 Orkiestracja Tradingu**: Automatyczne zarządzanie procesem od sygnału do wykonania transakcji
- **📊 Monitoring Real-time**: Ciągłe monitorowanie stanu systemu i wydajności
- **🚨 Inteligentne Alerty**: Wykrywanie anomalii i eskalacja powiadomień
- **📈 Analityka**: Automatyczne generowanie raportów i analiz wydajności
- **⚙️ Zarządzanie Systemem**: Backup, maintenance i operacje systemowe
- **💼 Zarządzanie Pozycjami**: Automatyczne monitorowanie i zamykanie pozycji

### Korzyści

- **Automatyzacja**: Pełna automatyzacja procesów tradingowych
- **Niezawodność**: Retry logic, error handling, graceful degradation
- **Skalowalność**: Możliwość dodawania nowych workflow'ów i integracji
- **Observability**: Kompleksowy monitoring i logging
- **Bezpieczeństwo**: Rate limiting, authentication, secure secrets

## 🏗️ Architektura

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Kestra UI     │    │  Kestra Server  │    │ Cerberus Bot    │
│   (Port 8081)   │◄──►│                 │◄──►│   (Port 8080)   │
└─────────────────┘    └─────────────────┘    └─────────────────┘
                                │                        │
                                ▼                        ▼
                       ┌─────────────────┐    ┌─────────────────┐
                       │   PostgreSQL    │    │   SQLite DB     │
                       │   (Kestra)      │    │   (Cerberus)    │
                       └─────────────────┘    └─────────────────┘
                                │
                                ▼
                       ┌─────────────────┐
                       │   Monitoring    │
                       │ Prometheus +    │
                       │    Grafana      │
                       └─────────────────┘
```

## 🚀 Instalacja

### Wymagania

- Docker & Docker Compose
- Cerberus Trading Bot (skonfigurowany)
- Telegram Bot Token (dla powiadomień)
- 4GB RAM minimum
- 10GB miejsca na dysku

### Szybki Start

1. **Klonowanie i przygotowanie**:
```bash
cd /path/to/cerberus
git pull  # Upewnij się, że masz najnowszą wersję
```

2. **Konfiguracja sekretów**:
```bash
cp kestra/secrets.env.example kestra/secrets.env
# Edytuj kestra/secrets.env i uzupełnij prawdziwe wartości
```

3. **Uruchomienie stacku**:
```bash
# Uruchomienie z Kestra
docker-compose --profile orchestration up -d

# Lub pełny stack z monitoringiem
docker-compose --profile orchestration --profile monitoring up -d
```

4. **Weryfikacja**:
```bash
# Sprawdzenie statusu
docker-compose ps

# Sprawdzenie logów
docker-compose logs -f kestra
docker-compose logs -f cerberus
```

### Dostęp do Interfejsów

- **Kestra UI**: http://localhost:8081
- **Cerberus API**: http://localhost:8080
- **Grafana**: http://localhost:3000 (admin/admin)
- **Prometheus**: http://localhost:9091

## ⚙️ Konfiguracja

### Sekrety

Skopiuj `kestra/secrets.env.example` jako `kestra/secrets.env` i skonfiguruj:

```env
# Telegram (wymagane dla alertów)
TELEGRAM_BOT_TOKEN=your_bot_token
TELEGRAM_CHAT_ID=your_chat_id

# Discord (opcjonalne)
DISCORD_WEBHOOK_URL=your_webhook_url

# Slack (opcjonalne)
SLACK_WEBHOOK_URL=your_webhook_url
```

### Telegram Bot Setup

1. Utwórz bota przez @BotFather
2. Otrzymaj token
3. Dodaj bota do grupy/kanału
4. Otrzymaj chat_id przez @userinfobot

### Konfiguracja Workflow'ów

Workflow'y są automatycznie ładowane z katalogu `kestra/flows/`. Możesz:

- Edytować istniejące workflow'y
- Dodawać nowe pliki `.yaml`
- Konfigurować triggery i parametry

## 🔄 Workflow'y

### 1. Trading Pipeline (`cerberus-trading-pipeline.yaml`)

**Cel**: Główny proces tradingowy od sygnału do wykonania

**Triggery**:
- Manual (dla testów)
- Co 5 minut w godzinach tradingu (9:00-17:00)
- Webhook dla zewnętrznych sygnałów

**Parametry**:
- `risk_threshold`: Próg ryzyka (default: 15.0 USD)
- `max_leverage`: Maksymalna dźwignia (default: 30x)
- `force_execution`: Wymusza wykonanie (default: false)

### 2. Monitoring (`cerberus-monitoring.yaml`)

**Cel**: Ciągłe monitorowanie stanu systemu

**Triggery**:
- Co 2 minuty (continuous monitoring)
- Codziennie o 18:00 (daily report)

**Funkcje**:
- Health checks
- Analiza metryk
- Wykrywanie problemów
- Automatyczne alerty

### 3. Analytics (`cerberus-analytics.yaml`)

**Cel**: Analiza wydajności i generowanie raportów

**Triggery**:
- Codziennie o 19:00 (1 dzień)
- W niedzielę o 20:00 (7 dni)
- 1. dnia miesiąca o 21:00 (30 dni)

**Funkcje**:
- Analiza historyczna
- Generowanie wykresów
- Rekomendacje
- Raporty przez Telegram

### 4. System Management (`cerberus-system-management.yaml`)

**Cel**: Operacje systemowe i maintenance

**Operacje**:
- `health_check`: Sprawdzenie stanu
- `backup`: Kopia zapasowa
- `maintenance`: Tryb maintenance
- `emergency_stop`: Zatrzymanie awaryjne
- `restart`: Restart systemu

### 5. Advanced Alerting (`cerberus-advanced-alerting.yaml`)

**Cel**: Inteligentne wykrywanie anomalii

**Funkcje**:
- Wykrywanie anomalii statystycznych
- Rate limiting alertów
- Multi-channel notifications
- Eskalacja alertów

### 6. Position Management (`cerberus-position-management.yaml`)

**Cel**: Zarządzanie otwartymi pozycjami

**Akcje**:
- `monitor`: Monitorowanie pozycji
- `close_all`: Zamknięcie wszystkich
- `close_losing`: Zamknięcie stratnych
- `emergency_close`: Zamknięcie awaryjne

## 📊 Monitoring

### Metryki Kluczowe

- **System**: CPU, RAM, uptime
- **Trading**: Success rate, P&L, pozycje
- **Performance**: Czas decyzji, throughput
- **Database**: Połączenia, query time, błędy

### Alerty

#### Poziomy Alertów:
- **INFO**: Informacyjne
- **WARNING**: Ostrzeżenia
- **CRITICAL**: Krytyczne
- **EMERGENCY**: Awaryjne

#### Kanały Powiadomień:
- Telegram (główny)
- Discord (opcjonalny)
- Slack (opcjonalny)
- Email (opcjonalny)

### Dashboardy

- **Kestra UI**: Status workflow'ów, logi, metryki
- **Grafana**: Metryki systemowe i tradingowe
- **Cerberus API**: Real-time data przez REST

## 🔌 API

### Cerberus API Endpoints

```
GET  /health                    # Basic health check
GET  /health/detailed          # Detailed system metrics
GET  /metrics                  # JSON metrics
GET  /metrics/prometheus       # Prometheus format

GET  /api/signals              # List signals
POST /api/signals              # Create signal
POST /api/signals/validate     # Validate signals

POST /api/risk/assess          # Risk assessment
GET  /api/risk/status          # Risk status

GET  /api/trades               # List trades
POST /api/trades               # Execute trade
GET  /api/trades/:id           # Get trade

GET  /api/positions            # List positions
GET  /api/positions/:id        # Get position
POST /api/positions/:id/close  # Close position

POST /api/system/emergency-stop # Emergency stop
POST /api/system/restart       # System restart
```

### Webhook Endpoints

```
POST /webhook/cerberus-trading-signal    # External trading signals
POST /webhook/cerberus-external-alert    # External alerts
```

## 🧪 Testowanie

### Test Manualny

1. **Sprawdzenie połączenia**:
```bash
curl http://localhost:8080/health
curl http://localhost:8081/api/v1/flows
```

2. **Test workflow'u**:
- Otwórz Kestra UI (http://localhost:8081)
- Przejdź do namespace `cerberus.trading`
- Uruchom `cerberus-trading-pipeline` manualnie
- Sprawdź logi i wyniki

3. **Test alertów**:
```bash
# Symulacja wysokiego CPU
curl -X POST http://localhost:8081/api/v1/executions/cerberus.alerting/cerberus-advanced-alerting
```

### Test Automatyczny

```bash
# Uruchomienie testów
./scripts/test-kestra-integration.sh
```

### Monitoring Testów

- Sprawdź logi w Kestra UI
- Monitoruj powiadomienia Telegram
- Weryfikuj metryki w Grafana

## 🔧 Troubleshooting

### Częste Problemy

#### 1. Kestra nie startuje
```bash
# Sprawdź logi
docker-compose logs kestra

# Sprawdź PostgreSQL
docker-compose logs postgres

# Restart
docker-compose restart kestra
```

#### 2. Brak połączenia z Cerberus
```bash
# Sprawdź czy Cerberus działa
curl http://localhost:8080/health

# Sprawdź network
docker network ls
docker network inspect botamelia_cerberus-net
```

#### 3. Workflow'y nie działają
- Sprawdź namespace w Kestra UI
- Weryfikuj składnię YAML
- Sprawdź logi execution

#### 4. Brak powiadomień
- Sprawdź konfigurację sekretów
- Zweryfikuj Telegram bot token
- Sprawdź chat_id

### Logi

```bash
# Wszystkie logi
docker-compose logs -f

# Tylko Kestra
docker-compose logs -f kestra

# Tylko Cerberus
docker-compose logs -f cerberus

# Z timestampami
docker-compose logs -f -t
```

### Performance

```bash
# Sprawdzenie zasobów
docker stats

# Sprawdzenie miejsca na dysku
df -h

# Sprawdzenie procesów
docker-compose top
```

## 📞 Wsparcie

- **Dokumentacja Kestra**: https://kestra.io/docs
- **Issues**: Utwórz issue w repozytorium
- **Logi**: Zawsze dołącz logi przy zgłaszaniu problemów

## 🔄 Aktualizacje

```bash
# Aktualizacja obrazów
docker-compose pull

# Restart z nowymi obrazami
docker-compose up -d

# Sprawdzenie wersji
docker-compose exec kestra kestra --version
```
