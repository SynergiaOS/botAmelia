use anyhow::Result;
use sentry::integrations::tracing::EventFilter;
use std::sync::Arc;
use tokio::signal;
use tracing::{error, info, warn};
use tracing_subscriber::prelude::*;

mod config;
mod database;
mod signals;
mod risk;
mod trading;
mod monitoring;
mod alerts;
mod cache;
mod security;
mod errors;
mod api;

use config::Config;
use database::DatabaseManager;
use monitoring::SystemMetrics;
// use api::ApiServer;

/// Główna struktura aplikacji Cerberus
pub struct CerberusApp {
    config: Arc<Config>,
    db_manager: Arc<DatabaseManager>,
    metrics: Arc<SystemMetrics>,
}

impl CerberusApp {
    /// Inicjalizuje nową instancję aplikacji Cerberus
    pub async fn new() -> Result<Self> {
        // Ładowanie konfiguracji
        let config = Arc::new(Config::load()?);
        
        // Inicjalizacja bazy danych
        let db_manager = Arc::new(DatabaseManager::new(&config.database).await?);
        
        // Inicjalizacja systemu metryk
        let metrics = Arc::new(SystemMetrics::new());

        info!("Cerberus application initialized successfully");

        Ok(Self {
            config,
            db_manager,
            metrics,
        })
    }
    
    /// Uruchamia główną pętlę aplikacji
    pub async fn run(&self) -> Result<()> {
        info!("Starting Cerberus v4.0 - Leverage Trading Framework");
        
        // Sprawdzenie stanu systemu
        self.health_check().await?;
        
        // Uruchomienie głównej pętli
        self.main_loop().await?;
        
        Ok(())
    }
    
    /// Sprawdza stan systemu przed uruchomieniem
    async fn health_check(&self) -> Result<()> {
        info!("Performing system health check...");
        
        // Sprawdzenie połączenia z bazą danych
        self.db_manager.health_check().await?;
        
        // Sprawdzenie konfiguracji
        self.config.validate()?;
        
        info!("System health check passed");
        Ok(())
    }
    
    /// Główna pętla aplikacji
    async fn main_loop(&self) -> Result<()> {
        info!("Starting main application loop");

        // Uruchomienie serwera API w tle
        let api_state = api::ApiState {
            config: self.config.clone(),
            db_manager: self.db_manager.clone(),
            metrics: self.metrics.clone(),
        };

        let api_future = tokio::spawn(async move {
            let mut api_server = match api::ApiServer::new(
                api_state.config.clone(),
                api_state.db_manager.clone(),
                api_state.metrics.clone(),
            ).await {
                Ok(server) => server,
                Err(e) => {
                    error!("Failed to create API server: {}", e);
                    return;
                }
            };

            if let Err(e) = api_server.serve().await {
                error!("API server error: {}", e);
            }
        });

        // Główna pętla z graceful shutdown
        tokio::select! {
            _ = signal::ctrl_c() => {
                info!("Received shutdown signal");
            }
            _ = self.run_trading_loop() => {
                warn!("Trading loop exited unexpectedly");
            }
            _ = api_future => {
                warn!("API server exited unexpectedly");
            }
        }

        self.shutdown().await?;
        Ok(())
    }
    
    /// Główna pętla tradingu (placeholder)
    async fn run_trading_loop(&self) -> Result<()> {
        loop {
            // Placeholder dla głównej logiki tradingu
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    }
    
    /// Graceful shutdown
    async fn shutdown(&self) -> Result<()> {
        info!("Shutting down Cerberus application...");
        
        // Zamknięcie wszystkich pozycji (jeśli są otwarte)
        // Zapisanie stanu do bazy danych
        // Zamknięcie połączeń
        
        info!("Cerberus application shut down successfully");
        Ok(())
    }
}

/// Inicjalizuje system logowania i monitorowania z integracją Sentry
fn init_observability() -> Result<sentry::ClientInitGuard> {
    // Ładowanie konfiguracji Sentry z zmiennych środowiskowych
    let sentry_dsn = std::env::var("SENTRY_DSN")
        .unwrap_or_else(|_| "".to_string());
    
    // Inicjalizacja Sentry
    let _guard = sentry::init((
        sentry_dsn,
        sentry::ClientOptions {
            release: sentry::release_name!(),
            environment: Some(
                std::env::var("ENVIRONMENT")
                    .unwrap_or_else(|_| "development".to_string())
                    .into()
            ),
            // Włączenie logów strukturalnych
            // enable_logs: true, // Field removed in newer sentry version
            // Sampling rate dla performance monitoring
            traces_sample_rate: 1.0,
            // Capture user IPs and headers for debugging
            send_default_pii: true,
            // Debug mode dla development
            debug: cfg!(debug_assertions),
            ..Default::default()
        },
    ));
    
    // Konfiguracja tracing subscriber z integracją Sentry
    let sentry_layer = sentry::integrations::tracing::layer()
        .event_filter(|md| match *md.level() {
            // Błędy krytyczne jako eventy Sentry (grupowane w issues)
            tracing::Level::ERROR => EventFilter::Event,
            // Ostrzeżenia jako logi i eventy
            tracing::Level::WARN => EventFilter::Event,
            // Trace level ignorowany (zbyt verbose)
            tracing::Level::TRACE => EventFilter::Ignore,
            // Wszystko inne jako logi strukturalne
            _ => EventFilter::Event,
        });
    
    // Inicjalizacja tracing subscriber
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true)
        )
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "cerberus=debug,info".into())
        )
        .with(sentry_layer)
        .init();
    
    info!("Observability system initialized with Sentry integration");
    Ok(_guard)
}

#[tokio::main]
async fn main() -> Result<()> {
    // Ładowanie zmiennych środowiskowych
    dotenvy::dotenv().ok();
    
    // Inicjalizacja systemu obserwacji (logging + Sentry)
    let _guard = init_observability()?;
    
    // Capture panic jako Sentry events
    std::panic::set_hook(Box::new(|panic_info| {
        sentry::capture_message(
            &format!("Panic occurred: {}", panic_info),
            sentry::Level::Fatal,
        );
    }));
    
    info!("Starting Cerberus v4.0 - High-Performance Leverage Trading Framework");
    
    // Inicjalizacja i uruchomienie aplikacji
    match CerberusApp::new().await {
        Ok(app) => {
            if let Err(e) = app.run().await {
                error!("Application error: {:?}", e);
                sentry::capture_error(e.as_ref() as &dyn std::error::Error);
                std::process::exit(1);
            }
        }
        Err(e) => {
            error!("Failed to initialize application: {:?}", e);
            sentry::capture_error(e.as_ref() as &dyn std::error::Error);
            std::process::exit(1);
        }
    }
    
    Ok(())
}
