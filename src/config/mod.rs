use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

pub mod database;
pub mod trading;
pub mod risk;
pub mod monitoring;
pub mod alerts;

pub use database::DatabaseConfig;
pub use trading::TradingConfig;
pub use risk::RiskConfig;
pub use monitoring::MonitoringConfig;
pub use alerts::AlertsConfig;

/// Główna konfiguracja aplikacji Cerberus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Konfiguracja bazy danych
    pub database: DatabaseConfig,
    
    /// Konfiguracja tradingu
    pub trading: TradingConfig,
    
    /// Konfiguracja zarządzania ryzykiem
    pub risk: RiskConfig,
    
    /// Konfiguracja monitorowania
    pub monitoring: MonitoringConfig,
    
    /// Konfiguracja alertów
    pub alerts: AlertsConfig,
    
    /// Konfiguracja Sentry
    pub sentry: SentryConfig,
    
    /// Środowisko (development, staging, production)
    pub environment: String,
    
    /// Port serwera HTTP dla API
    pub http_port: u16,
    
    /// Port dla metryk Prometheus
    pub metrics_port: u16,
}

/// Konfiguracja integracji Sentry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentryConfig {
    /// DSN dla Sentry
    pub dsn: Option<String>,
    
    /// Środowisko dla Sentry
    pub environment: String,
    
    /// Release version
    pub release: Option<String>,
    
    /// Sample rate dla traces (0.0 - 1.0)
    pub traces_sample_rate: f32,
    
    /// Czy wysyłać PII (Personally Identifiable Information)
    pub send_default_pii: bool,
    
    /// Debug mode
    pub debug: bool,
    
    /// Custom tags
    pub tags: std::collections::HashMap<String, String>,
}

impl Default for SentryConfig {
    fn default() -> Self {
        Self {
            dsn: None,
            environment: "development".to_string(),
            release: None,
            traces_sample_rate: 1.0,
            send_default_pii: false,
            debug: cfg!(debug_assertions),
            tags: std::collections::HashMap::new(),
        }
    }
}

impl Config {
    /// Ładuje konfigurację z pliku TOML i zmiennych środowiskowych
    pub fn load() -> Result<Self> {
        let mut settings = config::Config::builder();
        
        // Ładowanie z pliku konfiguracyjnego
        if Path::new("config/config.toml").exists() {
            settings = settings.add_source(
                config::File::with_name("config/config")
                    .format(config::FileFormat::Toml)
            );
        }
        
        // Ładowanie z zmiennych środowiskowych z prefiksem CERBERUS_
        settings = settings.add_source(
            config::Environment::with_prefix("CERBERUS")
                .separator("_")
        );
        
        let config = settings
            .build()
            .context("Failed to build configuration")?
            .try_deserialize::<Self>()
            .context("Failed to deserialize configuration")?;
        
        tracing::info!("Configuration loaded successfully");
        Ok(config)
    }
    
    /// Waliduje konfigurację
    pub fn validate(&self) -> Result<()> {
        // Walidacja konfiguracji bazy danych
        self.database.validate()
            .context("Database configuration validation failed")?;
        
        // Walidacja konfiguracji tradingu
        self.trading.validate()
            .context("Trading configuration validation failed")?;
        
        // Walidacja konfiguracji zarządzania ryzykiem
        self.risk.validate()
            .context("Risk configuration validation failed")?;
        
        // Walidacja portów
        if self.http_port == self.metrics_port {
            anyhow::bail!("HTTP port and metrics port cannot be the same");
        }
        
        // Walidacja Sentry
        if let Some(ref dsn) = self.sentry.dsn {
            if dsn.is_empty() {
                anyhow::bail!("Sentry DSN cannot be empty if provided");
            }
        }
        
        if !(0.0..=1.0).contains(&self.sentry.traces_sample_rate) {
            anyhow::bail!("Sentry traces_sample_rate must be between 0.0 and 1.0");
        }
        
        tracing::info!("Configuration validation passed");
        Ok(())
    }
    
    /// Zwraca czy aplikacja działa w trybie produkcyjnym
    pub fn is_production(&self) -> bool {
        self.environment == "production"
    }
    
    /// Zwraca czy aplikacja działa w trybie deweloperskim
    pub fn is_development(&self) -> bool {
        self.environment == "development"
    }
    
    /// Inicjalizuje Sentry z bieżącą konfiguracją
    pub fn init_sentry(&self) -> sentry::ClientInitGuard {
        let dsn = self.sentry.dsn.clone().unwrap_or_default();
        
        sentry::init((
            dsn,
            sentry::ClientOptions {
                release: self.sentry.release.clone()
                    .map(|s| s.into())
                    .or_else(|| sentry::release_name!().map(|s| s.to_string().into())),
                environment: Some(self.sentry.environment.clone().into()),
                traces_sample_rate: self.sentry.traces_sample_rate,
                send_default_pii: self.sentry.send_default_pii,
                debug: self.sentry.debug,
                ..Default::default()
            },
        ))
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            database: DatabaseConfig::default(),
            trading: TradingConfig::default(),
            risk: RiskConfig::default(),
            monitoring: MonitoringConfig::default(),
            alerts: AlertsConfig::default(),
            sentry: SentryConfig::default(),
            environment: "development".to_string(),
            http_port: 8080,
            metrics_port: 9090,
        }
    }
}
