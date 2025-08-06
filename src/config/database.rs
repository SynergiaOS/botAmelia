use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Konfiguracja bazy danych SQLite
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Ścieżka do pliku bazy danych
    pub path: PathBuf,
    
    /// Maksymalna liczba połączeń w pool
    pub max_connections: u32,
    
    /// Timeout dla połączeń (w sekundach)
    pub connection_timeout: u64,
    
    /// Czy włączyć WAL mode dla lepszej wydajności
    pub enable_wal: bool,
    
    /// Rozmiar cache dla SQLite (w KB)
    pub cache_size: i32,
    
    /// Czy włączyć foreign keys
    pub enable_foreign_keys: bool,
    
    /// Timeout dla zapytań (w sekundach)
    pub query_timeout: u64,
    
    /// Czy włączyć automatyczne tworzenie kopii zapasowych
    pub enable_backup: bool,
    
    /// Interwał tworzenia kopii zapasowych (w minutach)
    pub backup_interval: u64,
    
    /// Katalog dla kopii zapasowych
    pub backup_directory: PathBuf,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            path: PathBuf::from("data/cerberus.db"),
            max_connections: 10,
            connection_timeout: 30,
            enable_wal: true,
            cache_size: 64000, // 64MB
            enable_foreign_keys: true,
            query_timeout: 30,
            enable_backup: true,
            backup_interval: 60, // co godzinę
            backup_directory: PathBuf::from("data/backups"),
        }
    }
}

impl DatabaseConfig {
    /// Waliduje konfigurację bazy danych
    pub fn validate(&self) -> Result<()> {
        // Sprawdzenie czy katalog dla bazy danych istnieje
        if let Some(parent) = self.path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
                tracing::info!("Created database directory: {:?}", parent);
            }
        }
        
        // Sprawdzenie czy katalog dla kopii zapasowych istnieje
        if self.enable_backup && !self.backup_directory.exists() {
            std::fs::create_dir_all(&self.backup_directory)?;
            tracing::info!("Created backup directory: {:?}", self.backup_directory);
        }
        
        // Walidacja wartości
        if self.max_connections == 0 {
            anyhow::bail!("max_connections must be greater than 0");
        }
        
        if self.connection_timeout == 0 {
            anyhow::bail!("connection_timeout must be greater than 0");
        }
        
        if self.query_timeout == 0 {
            anyhow::bail!("query_timeout must be greater than 0");
        }
        
        if self.cache_size < 1000 {
            anyhow::bail!("cache_size must be at least 1000 KB");
        }
        
        if self.enable_backup && self.backup_interval == 0 {
            anyhow::bail!("backup_interval must be greater than 0 when backup is enabled");
        }
        
        Ok(())
    }
    
    /// Zwraca connection string dla SQLite
    pub fn connection_string(&self) -> String {
        format!("sqlite:{}", self.path.display())
    }
    
    /// Zwraca PRAGMA statements dla optymalizacji SQLite
    pub fn pragma_statements(&self) -> Vec<String> {
        let mut pragmas = Vec::new();
        
        if self.enable_wal {
            pragmas.push("PRAGMA journal_mode = WAL".to_string());
        }
        
        if self.enable_foreign_keys {
            pragmas.push("PRAGMA foreign_keys = ON".to_string());
        }
        
        pragmas.push(format!("PRAGMA cache_size = -{}", self.cache_size));
        pragmas.push("PRAGMA synchronous = NORMAL".to_string());
        pragmas.push("PRAGMA temp_store = MEMORY".to_string());
        pragmas.push("PRAGMA mmap_size = 268435456".to_string()); // 256MB
        
        pragmas
    }
}
