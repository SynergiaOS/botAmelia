use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Moduł monitorowania systemu z integracją Sentry
// pub mod metrics;
// pub mod health;
// pub mod performance;

// pub use metrics::MetricsCollector;
// pub use health::HealthMonitor;
// pub use performance::PerformanceMonitor;

/// Metryki systemowe
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    /// Całkowita liczba sygnałów
    pub total_signals: u64,

    /// Liczba udanych transakcji
    pub successful_trades: u64,

    /// Liczba nieudanych transakcji
    pub failed_trades: u64,

    /// Aktualny balans
    pub current_balance: f64,

    /// Dzienny P&L
    pub daily_pnl: f64,

    /// Czasy podejmowania decyzji (w ms)
    pub decision_times: Vec<f64>,

    /// Współczynnik sukcesu
    pub success_rate: f64,

    /// Użycie pamięci (w MB)
    pub memory_usage: f64,

    /// Użycie CPU (w procentach)
    pub cpu_usage: f64,

    /// Liczba aktywnych połączeń
    pub active_connections: u32,

    /// Metryki bazy danych
    pub database_metrics: DatabaseMetrics,

    /// Ostatnia aktualizacja
    pub last_updated: i64,
}

/// Metryki bazy danych
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseMetrics {
    /// Liczba aktywnych połączeń
    pub active_connections: u32,

    /// Średni czas zapytania (w ms)
    pub avg_query_time: f64,

    /// Liczba zapytań na sekundę
    pub queries_per_second: f64,

    /// Liczba błędów bazy danych
    pub error_count: u64,

    /// Rozmiar bazy danych (w MB)
    pub database_size: f64,
}

/// Stan zdrowia systemu
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum HealthStatus {
    Healthy,
    Warning,
    Critical,
    Down,
}

/// Raport stanu zdrowia
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    /// Ogólny stan systemu
    pub overall_status: HealthStatus,

    /// Stan poszczególnych komponentów
    pub components: HashMap<String, ComponentHealth>,

    /// Czas sprawdzenia
    pub checked_at: i64,

    /// Czas działania systemu (w sekundach)
    pub uptime: u64,
}

/// Stan zdrowia komponentu
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    /// Status komponentu
    pub status: HealthStatus,

    /// Komunikat
    pub message: String,

    /// Ostatnie sprawdzenie
    pub last_check: i64,

    /// Metryki komponentu
    pub metrics: HashMap<String, f64>,
}

/// Alert systemowy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemAlert {
    /// Identyfikator alertu
    pub id: String,

    /// Poziom alertu
    pub level: AlertLevel,

    /// Tytuł alertu
    pub title: String,

    /// Opis alertu
    pub description: String,

    /// Komponent, którego dotyczy alert
    pub component: String,

    /// Metryki związane z alertem
    pub metrics: HashMap<String, f64>,

    /// Czas utworzenia
    pub created_at: i64,

    /// Czy alert jest aktywny
    pub is_active: bool,
}

/// Poziom alertu
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum AlertLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl SystemMetrics {
    /// Tworzy nowe metryki systemowe
    pub fn new() -> Self {
        Self {
            total_signals: 0,
            successful_trades: 0,
            failed_trades: 0,
            current_balance: 0.0,
            daily_pnl: 0.0,
            decision_times: Vec::new(),
            success_rate: 0.0,
            memory_usage: 0.0,
            cpu_usage: 0.0,
            active_connections: 0,
            database_metrics: DatabaseMetrics::default(),
            last_updated: chrono::Utc::now().timestamp(),
        }
    }

    /// Aktualizuje metryki
    pub fn update(&mut self) {
        // Obliczenie współczynnika sukcesu
        let total_trades = self.successful_trades + self.failed_trades;
        if total_trades > 0 {
            self.success_rate = self.successful_trades as f64 / total_trades as f64;
        }

        // Aktualizacja czasu
        self.last_updated = chrono::Utc::now().timestamp();

        // Ograniczenie rozmiaru wektora czasów decyzji
        if self.decision_times.len() > 1000 {
            self.decision_times.drain(0..500); // Usuń starsze połowę
        }
    }

    /// Dodaje czas decyzji
    pub fn add_decision_time(&mut self, time_ms: f64) {
        self.decision_times.push(time_ms);
        self.update();
    }

    /// Zwraca średni czas decyzji
    pub fn avg_decision_time(&self) -> f64 {
        if self.decision_times.is_empty() {
            return 0.0;
        }
        self.decision_times.iter().sum::<f64>() / self.decision_times.len() as f64
    }

    /// Zwraca percentyl czasu decyzji
    pub fn decision_time_percentile(&self, percentile: f64) -> f64 {
        if self.decision_times.is_empty() {
            return 0.0;
        }

        let mut sorted_times = self.decision_times.clone();
        sorted_times.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let index = ((percentile / 100.0) * (sorted_times.len() - 1) as f64) as usize;
        sorted_times[index]
    }
}

impl Default for DatabaseMetrics {
    fn default() -> Self {
        Self {
            active_connections: 0,
            avg_query_time: 0.0,
            queries_per_second: 0.0,
            error_count: 0,
            database_size: 0.0,
        }
    }
}

impl HealthReport {
    /// Tworzy nowy raport zdrowia
    pub fn new() -> Self {
        Self {
            overall_status: HealthStatus::Healthy,
            components: HashMap::new(),
            checked_at: chrono::Utc::now().timestamp(),
            uptime: 0,
        }
    }

    /// Dodaje komponent do raportu
    pub fn add_component(&mut self, name: String, health: ComponentHealth) {
        // Aktualizacja ogólnego statusu na podstawie najgorszego komponentu
        if health.status > self.overall_status {
            self.overall_status = health.status.clone();
        }

        self.components.insert(name, health);
        self.checked_at = chrono::Utc::now().timestamp();
    }

    /// Sprawdza czy system jest zdrowy
    pub fn is_healthy(&self) -> bool {
        matches!(self.overall_status, HealthStatus::Healthy)
    }

    /// Zwraca listę komponentów w złym stanie
    pub fn unhealthy_components(&self) -> Vec<&String> {
        self.components
            .iter()
            .filter(|(_, health)| !matches!(health.status, HealthStatus::Healthy))
            .map(|(name, _)| name)
            .collect()
    }
}

impl ComponentHealth {
    /// Tworzy zdrowy komponent
    pub fn healthy(message: String) -> Self {
        Self {
            status: HealthStatus::Healthy,
            message,
            last_check: chrono::Utc::now().timestamp(),
            metrics: HashMap::new(),
        }
    }

    /// Tworzy komponent z ostrzeżeniem
    pub fn warning(message: String) -> Self {
        Self {
            status: HealthStatus::Warning,
            message,
            last_check: chrono::Utc::now().timestamp(),
            metrics: HashMap::new(),
        }
    }

    /// Tworzy komponent w stanie krytycznym
    pub fn critical(message: String) -> Self {
        Self {
            status: HealthStatus::Critical,
            message,
            last_check: chrono::Utc::now().timestamp(),
            metrics: HashMap::new(),
        }
    }

    /// Dodaje metrykę do komponentu
    pub fn with_metric(mut self, key: String, value: f64) -> Self {
        self.metrics.insert(key, value);
        self
    }
}

impl SystemAlert {
    /// Tworzy nowy alert
    pub fn new(level: AlertLevel, title: String, description: String, component: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            level,
            title,
            description,
            component,
            metrics: HashMap::new(),
            created_at: chrono::Utc::now().timestamp(),
            is_active: true,
        }
    }

    /// Dodaje metrykę do alertu
    pub fn with_metric(mut self, key: String, value: f64) -> Self {
        self.metrics.insert(key, value);
        self
    }

    /// Dezaktywuje alert
    pub fn deactivate(&mut self) {
        self.is_active = false;
    }

    /// Sprawdza czy alert powinien być wysłany do Sentry
    pub fn should_send_to_sentry(&self) -> bool {
        matches!(self.level, AlertLevel::High | AlertLevel::Critical)
    }

    /// Konwertuje na poziom Sentry
    pub fn to_sentry_level(&self) -> sentry::Level {
        match self.level {
            AlertLevel::Low => sentry::Level::Info,
            AlertLevel::Medium => sentry::Level::Warning,
            AlertLevel::High => sentry::Level::Error,
            AlertLevel::Critical => sentry::Level::Fatal,
        }
    }
}
