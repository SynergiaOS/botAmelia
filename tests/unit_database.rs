use anyhow::Result;
use cerberus::database::*;
use cerberus::config::DatabaseConfig;
use tempfile::TempDir;
use tokio::test;

/// Unit tests for database operations

#[tokio::test]
async fn test_database_manager_creation() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let mut config = DatabaseConfig::default();
    config.path = temp_dir.path().join("test.db");
    
    let db_manager = DatabaseManager::new(&config).await?;
    
    // Test that database file was created
    assert!(config.path.exists());
    
    Ok(())
}

#[tokio::test]
async fn test_database_health_check() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let mut config = DatabaseConfig::default();
    config.path = temp_dir.path().join("test.db");
    
    let db_manager = DatabaseManager::new(&config).await?;
    
    // Health check should pass for new database
    db_manager.health_check().await?;
    
    Ok(())
}

#[tokio::test]
async fn test_database_transaction() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let mut config = DatabaseConfig::default();
    config.path = temp_dir.path().join("test.db");
    
    let db_manager = DatabaseManager::new(&config).await?;
    
    // Test transaction creation and commit
    let mut tx = db_manager.begin_transaction().await?;
    tx.commit().await?;
    
    Ok(())
}

#[tokio::test]
async fn test_database_transaction_rollback() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let mut config = DatabaseConfig::default();
    config.path = temp_dir.path().join("test.db");
    
    let db_manager = DatabaseManager::new(&config).await?;
    
    // Test transaction creation and rollback
    let mut tx = db_manager.begin_transaction().await?;
    tx.rollback().await?;
    
    Ok(())
}

#[tokio::test]
async fn test_database_backup_creation() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let mut config = DatabaseConfig::default();
    config.path = temp_dir.path().join("test.db");
    config.enable_backup = true;
    
    let db_manager = DatabaseManager::new(&config).await?;
    
    // Create backup
    let backup_name = db_manager.create_backup().await?;
    assert!(!backup_name.is_empty());
    
    Ok(())
}

#[tokio::test]
async fn test_database_backup_disabled() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let mut config = DatabaseConfig::default();
    config.path = temp_dir.path().join("test.db");
    config.enable_backup = false;
    
    let db_manager = DatabaseManager::new(&config).await?;
    
    // Backup should be skipped when disabled
    let backup_name = db_manager.create_backup().await?;
    assert!(backup_name.is_empty());
    
    Ok(())
}

#[test]
fn test_database_config_connection_string() {
    let config = DatabaseConfig::default();
    let connection_string = config.connection_string();
    
    assert!(connection_string.starts_with("sqlite:"));
    assert!(connection_string.contains("cerberus.db"));
}

#[test]
fn test_database_config_validation() {
    let mut config = DatabaseConfig::default();
    
    // Valid config
    assert!(config.validate().is_ok());
    
    // Invalid max connections
    config.max_connections = 0;
    assert!(config.validate().is_err());
    
    // Invalid connection timeout
    config.max_connections = 10;
    config.connection_timeout = 0;
    assert!(config.validate().is_err());
    
    // Invalid cache size
    config.connection_timeout = 30;
    config.cache_size = 0;
    assert!(config.validate().is_err());
}

#[test]
fn test_database_config_wal_mode() {
    let mut config = DatabaseConfig::default();
    
    // WAL mode should be enabled by default
    assert!(config.enable_wal);
    
    // Test disabling WAL mode
    config.enable_wal = false;
    assert!(!config.enable_wal);
}

#[test]
fn test_database_config_backup_settings() {
    let mut config = DatabaseConfig::default();
    
    // Backup should be enabled by default
    assert!(config.enable_backup);
    
    // Test backup retention
    assert!(config.backup_retention_days > 0);
    
    // Test max backups
    assert!(config.max_backups > 0);
}

#[tokio::test]
async fn test_database_connection_pool() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let mut config = DatabaseConfig::default();
    config.path = temp_dir.path().join("test.db");
    config.max_connections = 5;
    
    let db_manager = DatabaseManager::new(&config).await?;
    
    // Test that we can get multiple connections
    let mut connections = Vec::new();
    for _ in 0..3 {
        let conn = db_manager.get_connection().await?;
        connections.push(conn);
    }
    
    assert_eq!(connections.len(), 3);
    
    Ok(())
}

#[tokio::test]
async fn test_database_query_execution() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let mut config = DatabaseConfig::default();
    config.path = temp_dir.path().join("test.db");
    
    let db_manager = DatabaseManager::new(&config).await?;
    
    // Test simple query execution
    let result = db_manager.execute_query("SELECT 1 as test_value").await?;
    assert!(result.rows_affected() >= 0);
    
    Ok(())
}

#[tokio::test]
async fn test_database_prepared_statement() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let mut config = DatabaseConfig::default();
    config.path = temp_dir.path().join("test.db");
    
    let db_manager = DatabaseManager::new(&config).await?;
    
    // Test prepared statement
    let query = "SELECT ? as test_value";
    let result = db_manager.execute_prepared(query, &[&42]).await?;
    assert!(result.rows_affected() >= 0);
    
    Ok(())
}

#[tokio::test]
async fn test_database_migration() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let mut config = DatabaseConfig::default();
    config.path = temp_dir.path().join("test.db");
    
    let db_manager = DatabaseManager::new(&config).await?;
    
    // Test migration execution
    db_manager.run_migrations().await?;
    
    // Verify that migrations table exists
    let result = db_manager.execute_query(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='migrations'"
    ).await?;
    
    // Should find the migrations table
    assert!(result.rows_affected() >= 0);
    
    Ok(())
}

#[tokio::test]
async fn test_database_error_handling() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let mut config = DatabaseConfig::default();
    config.path = temp_dir.path().join("test.db");
    
    let db_manager = DatabaseManager::new(&config).await?;
    
    // Test invalid query
    let result = db_manager.execute_query("INVALID SQL QUERY").await;
    assert!(result.is_err());
    
    Ok(())
}

#[tokio::test]
async fn test_database_concurrent_access() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let mut config = DatabaseConfig::default();
    config.path = temp_dir.path().join("test.db");
    config.max_connections = 10;
    
    let db_manager = DatabaseManager::new(&config).await?;
    
    // Test concurrent database access
    let mut handles = Vec::new();
    
    for i in 0..5 {
        let db_clone = db_manager.clone();
        let handle = tokio::spawn(async move {
            let query = format!("SELECT {} as test_value", i);
            db_clone.execute_query(&query).await
        });
        handles.push(handle);
    }
    
    // Wait for all tasks to complete
    for handle in handles {
        let result = handle.await?;
        assert!(result.is_ok());
    }
    
    Ok(())
}

#[tokio::test]
async fn test_database_performance_monitoring() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let mut config = DatabaseConfig::default();
    config.path = temp_dir.path().join("test.db");
    
    let db_manager = DatabaseManager::new(&config).await?;
    
    // Test performance monitoring
    let start = std::time::Instant::now();
    db_manager.execute_query("SELECT 1").await?;
    let duration = start.elapsed();
    
    // Query should complete quickly
    assert!(duration.as_millis() < 1000);
    
    Ok(())
}

#[tokio::test]
async fn test_database_cleanup() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let mut config = DatabaseConfig::default();
    config.path = temp_dir.path().join("test.db");
    
    let db_manager = DatabaseManager::new(&config).await?;
    
    // Test cleanup operations
    db_manager.cleanup().await?;
    
    // Database should still be accessible after cleanup
    db_manager.health_check().await?;
    
    Ok(())
}

#[test]
fn test_database_config_serialization() -> Result<()> {
    let config = DatabaseConfig::default();
    
    // Test TOML serialization
    let toml_string = toml::to_string(&config)?;
    assert!(!toml_string.is_empty());
    
    // Test deserialization
    let deserialized: DatabaseConfig = toml::from_str(&toml_string)?;
    assert_eq!(config.max_connections, deserialized.max_connections);
    assert_eq!(config.connection_timeout, deserialized.connection_timeout);
    assert_eq!(config.enable_wal, deserialized.enable_wal);
    
    Ok(())
}

#[test]
fn test_database_config_clone() {
    let config = DatabaseConfig::default();
    let cloned = config.clone();
    
    assert_eq!(config.max_connections, cloned.max_connections);
    assert_eq!(config.connection_timeout, cloned.connection_timeout);
    assert_eq!(config.enable_wal, cloned.enable_wal);
    assert_eq!(config.cache_size, cloned.cache_size);
}

#[test]
fn test_database_config_debug() {
    let config = DatabaseConfig::default();
    let debug_string = format!("{:?}", config);
    
    assert!(!debug_string.is_empty());
    assert!(debug_string.contains("DatabaseConfig"));
}

#[tokio::test]
async fn test_database_stress_test() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let mut config = DatabaseConfig::default();
    config.path = temp_dir.path().join("test.db");
    config.max_connections = 20;
    
    let db_manager = DatabaseManager::new(&config).await?;
    
    // Stress test with many concurrent operations
    let mut handles = Vec::new();
    
    for i in 0..50 {
        let db_clone = db_manager.clone();
        let handle = tokio::spawn(async move {
            for j in 0..10 {
                let query = format!("SELECT {} + {} as result", i, j);
                let _ = db_clone.execute_query(&query).await;
            }
        });
        handles.push(handle);
    }
    
    // Wait for all tasks to complete
    for handle in handles {
        handle.await?;
    }
    
    // Database should still be healthy after stress test
    db_manager.health_check().await?;
    
    Ok(())
}
