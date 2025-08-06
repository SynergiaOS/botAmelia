use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Konfiguracja systemu monitorowania
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Czy włączyć zbieranie metryk
    pub enable_metrics: bool,

    /// Interwał zbierania metryk (w sekundach)
    pub metrics_interval: u64,

    /// Maksymalne użycie pamięci (w MB)
    pub max_memory_usage: u64,

    /// Próg ostrzeżenia o użyciu pamięci (w MB)
    pub memory_warning_threshold: u64,

    /// Maksymalny czas odpowiedzi dla zapytań (w ms)
    pub max_query_time: u64,

    /// Próg ostrzeżenia o czasie odpowiedzi (w ms)
    pub query_time_warning: u64,

    /// Interwał sprawdzania stanu systemu (w sekundach)
    pub health_check_interval: u64,

    /// Czy włączyć monitoring wydajności
    pub enable_performance_monitoring: bool,

    /// Czy włączyć monitoring zasobów systemowych
    pub enable_system_monitoring: bool,

    /// Konfiguracja retencji metryk
    pub retention: RetentionConfig,

    /// Konfiguracja eksportu metryk
    pub export: ExportConfig,
}

/// Konfiguracja retencji danych monitorowania
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionConfig {
    /// Czas przechowywania metryk w wysokiej rozdzielczości (w dniach)
    pub high_resolution_days: u32,

    /// Czas przechowywania metryk w średniej rozdzielczości (w dniach)
    pub medium_resolution_days: u32,

    /// Czas przechowywania metryk w niskiej rozdzielczości (w dniach)
    pub low_resolution_days: u32,

    /// Czy włączyć automatyczne czyszczenie starych danych
    pub enable_auto_cleanup: bool,

    /// Interwał czyszczenia (w godzinach)
    pub cleanup_interval: u64,
}

/// Konfiguracja eksportu metryk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportConfig {
    /// Czy włączyć eksport do Prometheus
    pub enable_prometheus: bool,

    /// Port dla endpointu Prometheus
    pub prometheus_port: u16,

    /// Ścieżka dla endpointu Prometheus
    pub prometheus_path: String,

    /// Czy włączyć eksport do InfluxDB
    pub enable_influxdb: bool,

    /// URL InfluxDB
    pub influxdb_url: Option<String>,

    /// Token InfluxDB
    pub influxdb_token: Option<String>,

    /// Organizacja InfluxDB
    pub influxdb_org: Option<String>,

    /// Bucket InfluxDB
    pub influxdb_bucket: Option<String>,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            enable_metrics: true,
            metrics_interval: 10,          // 10 sekund
            max_memory_usage: 256,         // 256 MB
            memory_warning_threshold: 200, // 200 MB
            max_query_time: 100,           // 100 ms
            query_time_warning: 50,        // 50 ms
            health_check_interval: 30,     // 30 sekund
            enable_performance_monitoring: true,
            enable_system_monitoring: true,
            retention: RetentionConfig::default(),
            export: ExportConfig::default(),
        }
    }
}

impl Default for RetentionConfig {
    fn default() -> Self {
        Self {
            high_resolution_days: 7,    // 1 tydzień
            medium_resolution_days: 30, // 1 miesiąc
            low_resolution_days: 365,   // 1 rok
            enable_auto_cleanup: true,
            cleanup_interval: 24, // co 24 godziny
        }
    }
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            enable_prometheus: true,
            prometheus_port: 9090,
            prometheus_path: "/metrics".to_string(),
            enable_influxdb: false,
            influxdb_url: None,
            influxdb_token: None,
            influxdb_org: None,
            influxdb_bucket: None,
        }
    }
}

impl MonitoringConfig {
    /// Waliduje konfigurację monitorowania
    pub fn validate(&self) -> Result<()> {
        // Walidacja interwałów
        if self.metrics_interval == 0 {
            anyhow::bail!("metrics_interval must be greater than 0");
        }

        if self.health_check_interval == 0 {
            anyhow::bail!("health_check_interval must be greater than 0");
        }

        // Walidacja pamięci
        if self.max_memory_usage == 0 {
            anyhow::bail!("max_memory_usage must be greater than 0");
        }

        if self.memory_warning_threshold >= self.max_memory_usage {
            anyhow::bail!("memory_warning_threshold must be less than max_memory_usage");
        }

        // Walidacja czasów odpowiedzi
        if self.max_query_time == 0 {
            anyhow::bail!("max_query_time must be greater than 0");
        }

        if self.query_time_warning >= self.max_query_time {
            anyhow::bail!("query_time_warning must be less than max_query_time");
        }

        // Walidacja retencji
        self.retention.validate()?;

        // Walidacja eksportu
        self.export.validate()?;

        Ok(())
    }
}

impl RetentionConfig {
    /// Waliduje konfigurację retencji
    pub fn validate(&self) -> Result<()> {
        if self.high_resolution_days == 0 {
            anyhow::bail!("high_resolution_days must be greater than 0");
        }

        if self.medium_resolution_days < self.high_resolution_days {
            anyhow::bail!(
                "medium_resolution_days must be greater than or equal to high_resolution_days"
            );
        }

        if self.low_resolution_days < self.medium_resolution_days {
            anyhow::bail!(
                "low_resolution_days must be greater than or equal to medium_resolution_days"
            );
        }

        if self.enable_auto_cleanup && self.cleanup_interval == 0 {
            anyhow::bail!("cleanup_interval must be greater than 0 when auto_cleanup is enabled");
        }

        Ok(())
    }
}

impl ExportConfig {
    /// Waliduje konfigurację eksportu
    pub fn validate(&self) -> Result<()> {
        if self.enable_prometheus {
            if self.prometheus_port == 0 {
                anyhow::bail!("prometheus_port must be greater than 0");
            }

            if self.prometheus_path.is_empty() {
                anyhow::bail!("prometheus_path cannot be empty");
            }

            if !self.prometheus_path.starts_with('/') {
                anyhow::bail!("prometheus_path must start with '/'");
            }
        }

        if self.enable_influxdb {
            if self.influxdb_url.is_none() {
                anyhow::bail!("influxdb_url is required when InfluxDB export is enabled");
            }

            if self.influxdb_token.is_none() {
                anyhow::bail!("influxdb_token is required when InfluxDB export is enabled");
            }

            if self.influxdb_org.is_none() {
                anyhow::bail!("influxdb_org is required when InfluxDB export is enabled");
            }

            if self.influxdb_bucket.is_none() {
                anyhow::bail!("influxdb_bucket is required when InfluxDB export is enabled");
            }
        }

        Ok(())
    }
}
