use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Główne błędy aplikacji Cerberus z integracją Sentry
#[derive(Error, Debug, Serialize, Deserialize)]
pub enum CerberusError {
    /// Błędy konfiguracji
    #[error("Configuration error: {message}")]
    Configuration { message: String },

    /// Błędy bazy danych
    #[error("Database error: {message}")]
    Database { message: String },

    /// Błędy tradingu
    #[error("Trading error: {message}")]
    Trading { message: String },

    /// Błędy zarządzania ryzykiem
    #[error("Risk management error: {message}")]
    RiskManagement { message: String },

    /// Błędy sygnałów
    #[error("Signal processing error: {message}")]
    SignalProcessing { message: String },

    /// Błędy sieci
    #[error("Network error: {message}")]
    Network { message: String },

    /// Błędy autoryzacji
    #[error("Authentication error: {message}")]
    Authentication { message: String },

    /// Błędy walidacji
    #[error("Validation error: {message}")]
    Validation { message: String },

    /// Błędy cache
    #[error("Cache error: {message}")]
    Cache { message: String },

    /// Błędy bezpieczeństwa
    #[error("Security error: {message}")]
    Security { message: String },

    /// Błędy monitorowania
    #[error("Monitoring error: {message}")]
    Monitoring { message: String },

    /// Błędy alertów
    #[error("Alert error: {message}")]
    Alert { message: String },

    /// Błędy zewnętrzne (API, itp.)
    #[error("External service error: {service} - {message}")]
    ExternalService { service: String, message: String },

    /// Błędy wewnętrzne
    #[error("Internal error: {message}")]
    Internal { message: String },

    /// Błędy timeout
    #[error("Timeout error: {operation} timed out after {seconds}s")]
    Timeout { operation: String, seconds: u64 },

    /// Błędy rate limiting
    #[error("Rate limit exceeded: {message}")]
    RateLimit { message: String },

    /// Błędy parsowania
    #[error("Parse error: {message}")]
    Parse { message: String },

    /// Błędy IO
    #[error("IO error: {message}")]
    Io { message: String },
}

/// Poziom błędu dla Sentry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorLevel {
    Info,
    Warning,
    Error,
    Fatal,
}

/// Kontekst błędu
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorContext {
    /// Komponent, w którym wystąpił błąd
    pub component: String,

    /// Operacja, podczas której wystąpił błąd
    pub operation: String,

    /// Dodatkowe metadane
    pub metadata: serde_json::Value,

    /// Czas wystąpienia
    pub timestamp: i64,

    /// Stack trace (jeśli dostępny)
    pub stack_trace: Option<String>,
}

impl CerberusError {
    /// Zwraca poziom błędu dla Sentry
    pub fn sentry_level(&self) -> sentry::Level {
        match self {
            CerberusError::Configuration { .. } => sentry::Level::Error,
            CerberusError::Database { .. } => sentry::Level::Error,
            CerberusError::Trading { .. } => sentry::Level::Error,
            CerberusError::RiskManagement { .. } => sentry::Level::Error,
            CerberusError::SignalProcessing { .. } => sentry::Level::Warning,
            CerberusError::Network { .. } => sentry::Level::Warning,
            CerberusError::Authentication { .. } => sentry::Level::Error,
            CerberusError::Validation { .. } => sentry::Level::Warning,
            CerberusError::Cache { .. } => sentry::Level::Warning,
            CerberusError::Security { .. } => sentry::Level::Fatal,
            CerberusError::Monitoring { .. } => sentry::Level::Warning,
            CerberusError::Alert { .. } => sentry::Level::Warning,
            CerberusError::ExternalService { .. } => sentry::Level::Warning,
            CerberusError::Internal { .. } => sentry::Level::Fatal,
            CerberusError::Timeout { .. } => sentry::Level::Warning,
            CerberusError::RateLimit { .. } => sentry::Level::Info,
            CerberusError::Parse { .. } => sentry::Level::Warning,
            CerberusError::Io { .. } => sentry::Level::Error,
        }
    }

    /// Zwraca kategorię błędu
    pub fn category(&self) -> &'static str {
        match self {
            CerberusError::Configuration { .. } => "configuration",
            CerberusError::Database { .. } => "database",
            CerberusError::Trading { .. } => "trading",
            CerberusError::RiskManagement { .. } => "risk_management",
            CerberusError::SignalProcessing { .. } => "signal_processing",
            CerberusError::Network { .. } => "network",
            CerberusError::Authentication { .. } => "authentication",
            CerberusError::Validation { .. } => "validation",
            CerberusError::Cache { .. } => "cache",
            CerberusError::Security { .. } => "security",
            CerberusError::Monitoring { .. } => "monitoring",
            CerberusError::Alert { .. } => "alert",
            CerberusError::ExternalService { .. } => "external_service",
            CerberusError::Internal { .. } => "internal",
            CerberusError::Timeout { .. } => "timeout",
            CerberusError::RateLimit { .. } => "rate_limit",
            CerberusError::Parse { .. } => "parse",
            CerberusError::Io { .. } => "io",
        }
    }

    /// Sprawdza czy błąd jest krytyczny
    pub fn is_critical(&self) -> bool {
        matches!(
            self,
            CerberusError::Security { .. } | CerberusError::Internal { .. }
        )
    }

    /// Sprawdza czy błąd można ponowić
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            CerberusError::Network { .. }
                | CerberusError::ExternalService { .. }
                | CerberusError::Timeout { .. }
                | CerberusError::Database { .. }
        )
    }

    /// Raportuje błąd do Sentry z kontekstem
    pub fn report_to_sentry(&self, context: Option<ErrorContext>) {
        sentry::configure_scope(|scope| {
            scope.set_tag("error_category", self.category());
            scope.set_tag("error_critical", self.is_critical().to_string());
            scope.set_tag("error_retryable", self.is_retryable().to_string());

            if let Some(ctx) = &context {
                scope.set_tag("component", &ctx.component);
                scope.set_tag("operation", &ctx.operation);
                scope.set_extra("metadata", ctx.metadata.clone().into());
                scope.set_extra("timestamp", ctx.timestamp.into());

                if let Some(ref stack_trace) = ctx.stack_trace {
                    scope.set_extra("stack_trace", stack_trace.clone().into());
                }
            }
        });

        sentry::capture_error(self);

        // Logowanie błędu
        match self.sentry_level() {
            sentry::Level::Fatal => tracing::error!("FATAL ERROR: {}", self),
            sentry::Level::Error => tracing::error!("ERROR: {}", self),
            sentry::Level::Warning => tracing::warn!("WARNING: {}", self),
            sentry::Level::Info => tracing::info!("INFO: {}", self),
            _ => tracing::debug!("DEBUG: {}", self),
        }
    }
}

impl ErrorContext {
    /// Tworzy nowy kontekst błędu
    pub fn new(component: String, operation: String) -> Self {
        Self {
            component,
            operation,
            metadata: serde_json::Value::Null,
            timestamp: chrono::Utc::now().timestamp(),
            stack_trace: None,
        }
    }

    /// Dodaje metadane do kontekstu
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }

    /// Dodaje stack trace do kontekstu
    pub fn with_stack_trace(mut self, stack_trace: String) -> Self {
        self.stack_trace = Some(stack_trace);
        self
    }
}

/// Makro do tworzenia błędów z kontekstem
#[macro_export]
macro_rules! cerberus_error {
    ($variant:ident, $message:expr) => {
        CerberusError::$variant {
            message: $message.to_string(),
        }
    };
    ($variant:ident, $message:expr, $($field:ident: $value:expr),+) => {
        CerberusError::$variant {
            message: $message.to_string(),
            $($field: $value),+
        }
    };
}

/// Trait dla konwersji błędów z automatycznym raportowaniem do Sentry
pub trait IntoSentryError<T> {
    fn report_error(self, component: &str, operation: &str) -> Result<T, CerberusError>;
}

impl<T, E> IntoSentryError<T> for Result<T, E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn report_error(self, component: &str, operation: &str) -> Result<T, CerberusError> {
        self.map_err(|e| {
            let error = CerberusError::Internal {
                message: e.to_string(),
            };

            let context = ErrorContext::new(component.to_string(), operation.to_string());
            error.report_to_sentry(Some(context));

            error
        })
    }
}

/// Konwersje z popularnych błędów
impl From<sqlx::Error> for CerberusError {
    fn from(err: sqlx::Error) -> Self {
        CerberusError::Database {
            message: err.to_string(),
        }
    }
}

impl From<reqwest::Error> for CerberusError {
    fn from(err: reqwest::Error) -> Self {
        CerberusError::Network {
            message: err.to_string(),
        }
    }
}

impl From<serde_json::Error> for CerberusError {
    fn from(err: serde_json::Error) -> Self {
        CerberusError::Parse {
            message: err.to_string(),
        }
    }
}

impl From<std::io::Error> for CerberusError {
    fn from(err: std::io::Error) -> Self {
        CerberusError::Io {
            message: err.to_string(),
        }
    }
}

impl From<tokio::time::error::Elapsed> for CerberusError {
    fn from(err: tokio::time::error::Elapsed) -> Self {
        CerberusError::Timeout {
            operation: "unknown".to_string(),
            seconds: 0,
        }
    }
}

/// Result type dla aplikacji Cerberus
pub type CerberusResult<T> = Result<T, CerberusError>;

/// Utility funkcje dla obsługi błędów
pub mod utils {
    use super::*;

    /// Retry mechanizm dla operacji, które mogą się nie powieść
    pub async fn retry_operation<F, T, E>(
        operation: F,
        max_retries: u32,
        delay_ms: u64,
    ) -> Result<T, E>
    where
        F: Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, E>> + Send>>,
        E: std::error::Error,
    {
        let mut last_error = None;

        for attempt in 0..=max_retries {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    last_error = Some(e);

                    if attempt < max_retries {
                        tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                    }
                }
            }
        }

        Err(last_error.unwrap())
    }

    /// Loguje błąd z odpowiednim poziomem
    pub fn log_error(error: &CerberusError, context: Option<&ErrorContext>) {
        let level = error.sentry_level();
        let message = format!("{} (category: {})", error, error.category());

        match level {
            sentry::Level::Fatal => tracing::error!("{}", message),
            sentry::Level::Error => tracing::error!("{}", message),
            sentry::Level::Warning => tracing::warn!("{}", message),
            sentry::Level::Info => tracing::info!("{}", message),
            _ => tracing::debug!("{}", message),
        }

        if let Some(ctx) = context {
            tracing::debug!(
                "Error context: component={}, operation={}",
                ctx.component,
                ctx.operation
            );
        }
    }
}
