/// Cerberus v5.0 Trading System Library
/// 
/// This library provides all the core functionality for the Cerberus trading system,
/// including signal processing, risk management, trading execution, and monitoring.

// Core modules
pub mod config;
pub mod database;
pub mod signals;
pub mod risk;
pub mod trading;
pub mod cache;
pub mod security;
pub mod monitoring;
pub mod alerts;
pub mod errors;
pub mod api;

// Re-export commonly used types
pub use config::Config;
pub use errors::{CerberusError, CerberusResult};

// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = env!("CARGO_PKG_NAME");

/// Initialize the Cerberus library
/// 
/// This function should be called once at the start of the application
/// to set up logging, error handling, and other global state.
pub fn init() -> CerberusResult<()> {
    // Initialize logging if not already done
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    
    env_logger::try_init().ok(); // Ignore error if already initialized
    
    log::info!("Cerberus v{} initialized", VERSION);
    Ok(())
}

/// Initialize the Cerberus library with custom configuration
pub fn init_with_config(config: &Config) -> CerberusResult<()> {
    // Set log level based on environment
    let log_level = match config.environment.as_str() {
        "development" => "debug",
        "test" => "debug", 
        "staging" => "info",
        "production" => "warn",
        _ => "info",
    };
    
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", log_level);
    }
    
    env_logger::try_init().ok();
    
    log::info!("Cerberus v{} initialized with {} environment", VERSION, config.environment);
    
    // Initialize Sentry if configured
    let sentry_config = &config.sentry;
    if let Some(ref dsn) = sentry_config.dsn {
        let _guard = sentry::init((
            dsn.as_str(),
            sentry::ClientOptions {
                release: sentry::release_name!(),
                environment: Some(config.environment.clone().into()),
                traces_sample_rate: sentry_config.traces_sample_rate,
                ..Default::default()
            },
        ));

        log::info!("Sentry monitoring initialized");
    }
    
    Ok(())
}

/// Get version information
pub fn version_info() -> String {
    format!("{} v{}", NAME, VERSION)
}

/// Check if running in paper trading mode
pub fn is_paper_trading() -> bool {
    std::env::var("CERBERUS_TRADING_PAPER_TRADING")
        .unwrap_or_default()
        .parse()
        .unwrap_or(true) // Default to paper trading for safety
}

/// Check if running in test environment
pub fn is_test_environment() -> bool {
    std::env::var("CERBERUS_ENVIRONMENT")
        .unwrap_or_default()
        == "test"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_info() {
        let version = version_info();
        assert!(version.contains("cerberus"));
        assert!(version.contains("v"));
    }

    #[test]
    fn test_paper_trading_detection() {
        // Should default to true for safety
        assert!(is_paper_trading());
    }

    #[test]
    fn test_init() {
        let result = init();
        assert!(result.is_ok());
    }
}
