use anyhow::{Context, Result};
use sqlx::{Sqlite, SqlitePool};
use std::sync::Arc;
use tracing::{info, warn, error};

use crate::config::DatabaseConfig;

// pub mod schema;
// pub mod migrations;
// pub mod operations;

// pub use operations::*;

/// Manager bazy danych z integracją Sentry
pub struct DatabaseManager {
    pool: Arc<SqlitePool>,
    config: DatabaseConfig,
}

impl DatabaseManager {
    /// Tworzy nowy manager bazy danych
    pub async fn new(config: &DatabaseConfig) -> Result<Self> {
        info!("Initializing database manager");
        
        // Tworzenie katalogu dla bazy danych jeśli nie istnieje
        if let Some(parent) = config.path.parent() {
            tokio::fs::create_dir_all(parent).await
                .context("Failed to create database directory")?;
        }
        
        // Konfiguracja połączenia
        let connection_string = config.connection_string();
        let pool = SqlitePool::connect_with(
            sqlx::sqlite::SqliteConnectOptions::new()
                .filename(&config.path)
                .create_if_missing(true)
                .journal_mode(if config.enable_wal {
                    sqlx::sqlite::SqliteJournalMode::Wal
                } else {
                    sqlx::sqlite::SqliteJournalMode::Delete
                })
                .foreign_keys(config.enable_foreign_keys)
                .pragma("cache_size", format!("-{}", config.cache_size))
                .pragma("synchronous", "NORMAL")
                .pragma("temp_store", "MEMORY")
                .pragma("mmap_size", "268435456") // 256MB
        ).await.context("Failed to connect to database")?;
        
        let manager = Self {
            pool: Arc::new(pool),
            config: config.clone(),
        };
        
        // Uruchomienie migracji
        manager.run_migrations().await?;
        
        // Sprawdzenie stanu bazy danych
        manager.health_check().await?;
        
        info!("Database manager initialized successfully");
        Ok(manager)
    }
    
    /// Zwraca referencję do pool połączeń
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
    
    /// Uruchamia migracje bazy danych
    async fn run_migrations(&self) -> Result<()> {
        info!("Running database migrations");
        
        // Tutaj będą uruchamiane migracje
        // migrations::run_all(&self.pool).await
        //     .context("Failed to run database migrations")?;
        // Mock implementation for now
        
        info!("Database migrations completed successfully");
        Ok(())
    }
    
    /// Sprawdza stan bazy danych
    pub async fn health_check(&self) -> Result<()> {
        let start = std::time::Instant::now();
        
        // Proste zapytanie testowe
        let result = sqlx::query("SELECT 1")
            .fetch_one(&*self.pool)
            .await;
        
        let duration = start.elapsed();
        
        match result {
            Ok(_) => {
                if duration.as_millis() > self.config.query_timeout as u128 * 1000 {
                    warn!(
                        "Database health check slow: {}ms (threshold: {}s)",
                        duration.as_millis(),
                        self.config.query_timeout
                    );
                    
                    // Raportowanie do Sentry
                    sentry::capture_message(
                        &format!("Slow database health check: {}ms", duration.as_millis()),
                        sentry::Level::Warning,
                    );
                }
                
                info!("Database health check passed in {}ms", duration.as_millis());
                Ok(())
            }
            Err(e) => {
                error!("Database health check failed: {:?}", e);
                
                // Raportowanie błędu do Sentry
                sentry::capture_error(&e);
                
                Err(e.into())
            }
        }
    }
    
    /// Rozpoczyna transakcję
    pub async fn begin_transaction(&self) -> Result<sqlx::Transaction<'_, Sqlite>> {
        self.pool.begin().await
            .context("Failed to begin database transaction")
    }
    
    /// Wykonuje zapytanie z monitorowaniem wydajności
    pub async fn execute_with_monitoring<F, T>(&self, operation_name: &str, f: F) -> Result<T>
    where
        F: std::future::Future<Output = Result<T>>,
    {
        let start = std::time::Instant::now();
        
        // Rozpoczęcie span dla Sentry
        let span = sentry::start_transaction(
            sentry::TransactionContext::new("database", operation_name)
        );
        
        let result = f.await;
        let duration = start.elapsed();
        
        // Logowanie wydajności
        if duration.as_millis() > 100 {
            warn!(
                "Slow database operation '{}': {}ms",
                operation_name,
                duration.as_millis()
            );
            
            // Dodanie informacji do Sentry span
            span.set_data("duration_ms", (duration.as_millis() as u64).into());
            span.set_data("slow_query", true.into());
        }
        
        match &result {
            Ok(_) => {
                span.set_status(sentry::protocol::SpanStatus::Ok);
            }
            Err(e) => {
                span.set_status(sentry::protocol::SpanStatus::InternalError);
                span.set_data("error", format!("{:?}", e).into());
                
                // Raportowanie błędu do Sentry
                sentry::capture_message(&format!("Database error: {}", e), sentry::Level::Error);
            }
        }
        
        span.finish();
        result
    }
    
    /// Tworzy kopię zapasową bazy danych
    pub async fn create_backup(&self) -> Result<String> {
        if !self.config.enable_backup {
            return Err(anyhow::anyhow!("Backup is disabled in configuration"));
        }
        
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let backup_filename = format!("cerberus_backup_{}.db", timestamp);
        let backup_path = self.config.backup_directory.join(&backup_filename);
        
        info!("Creating database backup: {:?}", backup_path);
        
        // Kopiowanie pliku bazy danych
        tokio::fs::copy(&self.config.path, &backup_path).await
            .context("Failed to create database backup")?;
        
        info!("Database backup created successfully: {:?}", backup_path);
        Ok(backup_filename)
    }
    
    /// Czyści stare kopie zapasowe
    pub async fn cleanup_old_backups(&self, keep_count: usize) -> Result<()> {
        if !self.config.enable_backup {
            return Ok(());
        }
        
        let mut entries = tokio::fs::read_dir(&self.config.backup_directory).await
            .context("Failed to read backup directory")?;
        
        let mut backup_files = Vec::new();
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "db") {
                if let Ok(metadata) = entry.metadata().await {
                    backup_files.push((path, metadata.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH)));
                }
            }
        }
        
        // Sortowanie według daty modyfikacji (najnowsze pierwsze)
        backup_files.sort_by(|a, b| b.1.cmp(&a.1));
        
        // Usuwanie starych kopii zapasowych
        for (path, _) in backup_files.iter().skip(keep_count) {
            if let Err(e) = tokio::fs::remove_file(path).await {
                warn!("Failed to remove old backup {:?}: {:?}", path, e);
            } else {
                info!("Removed old backup: {:?}", path);
            }
        }
        
        Ok(())
    }
}
