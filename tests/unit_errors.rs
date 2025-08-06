use anyhow::Result;
use cerberus::errors::*;
use serde_json::json;

/// Unit tests for error handling system

#[test]
fn test_cerberus_error_creation() {
    let error = CerberusError::Trading {
        message: "Insufficient funds".to_string(),
    };

    assert_eq!(error.category(), "trading");
    assert!(!error.is_critical());
    assert!(error.is_retryable());
    assert_eq!(error.sentry_level(), sentry::Level::Error);
}

#[test]
fn test_cerberus_error_categories() {
    let config_error = CerberusError::Configuration {
        message: "Invalid config".to_string(),
    };
    assert_eq!(config_error.category(), "configuration");

    let db_error = CerberusError::Database {
        message: "Connection failed".to_string(),
    };
    assert_eq!(db_error.category(), "database");

    let security_error = CerberusError::Security {
        message: "Unauthorized access".to_string(),
    };
    assert_eq!(security_error.category(), "security");
    assert!(security_error.is_critical());
}

#[test]
fn test_cerberus_error_sentry_levels() {
    let security_error = CerberusError::Security {
        message: "Security breach".to_string(),
    };
    assert_eq!(security_error.sentry_level(), sentry::Level::Fatal);

    let internal_error = CerberusError::Internal {
        message: "Internal error".to_string(),
    };
    assert_eq!(internal_error.sentry_level(), sentry::Level::Fatal);

    let network_error = CerberusError::Network {
        message: "Network timeout".to_string(),
    };
    assert_eq!(network_error.sentry_level(), sentry::Level::Warning);

    let rate_limit_error = CerberusError::RateLimit {
        message: "Rate limit exceeded".to_string(),
    };
    assert_eq!(rate_limit_error.sentry_level(), sentry::Level::Info);
}

#[test]
fn test_cerberus_error_retryable() {
    // Retryable errors
    let network_error = CerberusError::Network {
        message: "Connection timeout".to_string(),
    };
    assert!(network_error.is_retryable());

    let external_error = CerberusError::ExternalService {
        service: "API".to_string(),
        message: "Service unavailable".to_string(),
    };
    assert!(external_error.is_retryable());

    let timeout_error = CerberusError::Timeout {
        operation: "trade_execution".to_string(),
        seconds: 30,
    };
    assert!(timeout_error.is_retryable());

    let db_error = CerberusError::Database {
        message: "Temporary connection issue".to_string(),
    };
    assert!(db_error.is_retryable());

    // Non-retryable errors
    let validation_error = CerberusError::Validation {
        message: "Invalid input".to_string(),
    };
    assert!(!validation_error.is_retryable());

    let security_error = CerberusError::Security {
        message: "Unauthorized".to_string(),
    };
    assert!(!security_error.is_retryable());
}

#[test]
fn test_cerberus_error_critical() {
    // Critical errors
    let security_error = CerberusError::Security {
        message: "Security breach".to_string(),
    };
    assert!(security_error.is_critical());

    let internal_error = CerberusError::Internal {
        message: "System failure".to_string(),
    };
    assert!(internal_error.is_critical());

    // Non-critical errors
    let trading_error = CerberusError::Trading {
        message: "Order failed".to_string(),
    };
    assert!(!trading_error.is_critical());

    let network_error = CerberusError::Network {
        message: "Connection lost".to_string(),
    };
    assert!(!network_error.is_critical());
}

#[test]
fn test_error_context_creation() {
    let context = ErrorContext::new("trading_engine".to_string(), "execute_order".to_string());

    assert_eq!(context.component, "trading_engine");
    assert_eq!(context.operation, "execute_order");
    assert_eq!(context.metadata, serde_json::Value::Null);
    assert!(context.stack_trace.is_none());
    assert!(context.timestamp > 0);
}

#[test]
fn test_error_context_with_metadata() {
    let metadata = json!({
        "order_id": "12345",
        "token": "BONK",
        "size": 100.0
    });

    let context = ErrorContext::new("trading_engine".to_string(), "execute_order".to_string())
        .with_metadata(metadata.clone());

    assert_eq!(context.metadata, metadata);
}

#[test]
fn test_error_context_with_stack_trace() {
    let stack_trace = "at trading_engine::execute_order (line 123)\nat main (line 456)".to_string();

    let context = ErrorContext::new("trading_engine".to_string(), "execute_order".to_string())
        .with_stack_trace(stack_trace.clone());

    assert_eq!(context.stack_trace, Some(stack_trace));
}

#[test]
fn test_cerberus_error_macro() {
    let error = cerberus_error!(Trading, "Order execution failed");

    match error {
        CerberusError::Trading { message } => {
            assert_eq!(message, "Order execution failed");
        }
        _ => panic!("Wrong error type"),
    }
}

#[test]
fn test_cerberus_error_macro_with_fields() {
    let error = cerberus_error!(
        ExternalService,
        "API call failed",
        service: "Binance".to_string()
    );

    match error {
        CerberusError::ExternalService { message, service } => {
            assert_eq!(message, "API call failed");
            assert_eq!(service, "Binance");
        }
        _ => panic!("Wrong error type"),
    }
}

#[test]
fn test_error_conversions() {
    // Test conversion from sqlx::Error
    let sqlx_error = sqlx::Error::RowNotFound;
    let cerberus_error: CerberusError = sqlx_error.into();

    match cerberus_error {
        CerberusError::Database { message } => {
            assert!(!message.is_empty());
        }
        _ => panic!("Wrong error type"),
    }

    // Test conversion from serde_json::Error
    let json_error = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
    let cerberus_error: CerberusError = json_error.into();

    match cerberus_error {
        CerberusError::Parse { message } => {
            assert!(!message.is_empty());
        }
        _ => panic!("Wrong error type"),
    }
}

#[test]
fn test_error_serialization() -> Result<()> {
    let error = CerberusError::Trading {
        message: "Test error".to_string(),
    };

    // Test JSON serialization
    let json = serde_json::to_string(&error)?;
    assert!(!json.is_empty());

    // Test deserialization
    let deserialized: CerberusError = serde_json::from_str(&json)?;

    match (error, deserialized) {
        (CerberusError::Trading { message: m1 }, CerberusError::Trading { message: m2 }) => {
            assert_eq!(m1, m2);
        }
        _ => panic!("Serialization/deserialization failed"),
    }

    Ok(())
}

#[test]
fn test_error_context_serialization() -> Result<()> {
    let context = ErrorContext::new("test_component".to_string(), "test_operation".to_string())
        .with_metadata(json!({"test": true}));

    // Test JSON serialization
    let json = serde_json::to_string(&context)?;
    assert!(!json.is_empty());

    // Test deserialization
    let deserialized: ErrorContext = serde_json::from_str(&json)?;
    assert_eq!(context.component, deserialized.component);
    assert_eq!(context.operation, deserialized.operation);
    assert_eq!(context.metadata, deserialized.metadata);

    Ok(())
}

#[test]
fn test_into_sentry_error_trait() {
    // Test successful result
    let success_result: Result<i32, std::io::Error> = Ok(42);
    let mapped_result = success_result.report_error("test_component", "test_operation");
    assert!(mapped_result.is_ok());
    assert_eq!(mapped_result.unwrap(), 42);

    // Test error result
    let error_result: Result<i32, std::io::Error> = Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "File not found",
    ));
    let mapped_result = error_result.report_error("test_component", "test_operation");
    assert!(mapped_result.is_err());

    match mapped_result.unwrap_err() {
        CerberusError::Internal { message } => {
            assert!(message.contains("File not found"));
        }
        _ => panic!("Wrong error type"),
    }
}

#[test]
fn test_cerberus_result_type() {
    // Test successful result
    let success: CerberusResult<i32> = Ok(42);
    assert!(success.is_ok());
    assert_eq!(success.unwrap(), 42);

    // Test error result
    let error: CerberusResult<i32> = Err(CerberusError::Validation {
        message: "Invalid input".to_string(),
    });
    assert!(error.is_err());
}

#[test]
fn test_error_display() {
    let error = CerberusError::Trading {
        message: "Order execution failed".to_string(),
    };

    let display_string = format!("{}", error);
    assert!(display_string.contains("Trading error"));
    assert!(display_string.contains("Order execution failed"));
}

#[test]
fn test_error_debug() {
    let error = CerberusError::Security {
        message: "Unauthorized access".to_string(),
    };

    let debug_string = format!("{:?}", error);
    assert!(debug_string.contains("Security"));
    assert!(debug_string.contains("Unauthorized access"));
}

#[test]
fn test_error_chain() {
    use std::error::Error;

    let io_error = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "Access denied");
    let cerberus_error: CerberusError = io_error.into();

    // Test error chain
    let source = cerberus_error.source();
    assert!(source.is_none()); // CerberusError doesn't chain sources by default
}

#[test]
fn test_error_utils_log_error() {
    let error = CerberusError::Network {
        message: "Connection timeout".to_string(),
    };

    let context = ErrorContext::new("network_client".to_string(), "send_request".to_string());

    // Test that logging doesn't panic
    utils::log_error(&error, Some(&context));
    utils::log_error(&error, None);
}

#[tokio::test]
async fn test_error_utils_retry_operation() {
    let mut attempt_count = 0;

    // Test successful operation after retries
    let operation = || {
        attempt_count += 1;
        Box::pin(async move {
            if attempt_count < 3 {
                Err(std::io::Error::new(std::io::ErrorKind::TimedOut, "Timeout"))
            } else {
                Ok(42)
            }
        })
    };

    let result = utils::retry_operation(operation, 5, 10).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 42);
    assert_eq!(attempt_count, 3);
}

#[tokio::test]
async fn test_error_utils_retry_operation_max_retries() {
    let mut attempt_count = 0;

    // Test operation that always fails
    let operation = || {
        attempt_count += 1;
        Box::pin(async move {
            Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "Always fails",
            ))
        })
    };

    let result = utils::retry_operation(operation, 2, 10).await;
    assert!(result.is_err());
    assert_eq!(attempt_count, 3); // Initial attempt + 2 retries
}

#[test]
fn test_error_edge_cases() {
    // Test empty error messages
    let empty_error = CerberusError::Internal {
        message: "".to_string(),
    };
    assert_eq!(empty_error.category(), "internal");

    // Test very long error messages
    let long_message = "x".repeat(10000);
    let long_error = CerberusError::Validation {
        message: long_message.clone(),
    };

    match long_error {
        CerberusError::Validation { message } => {
            assert_eq!(message.len(), 10000);
        }
        _ => panic!("Wrong error type"),
    }
}

#[test]
fn test_error_equality() {
    let error1 = CerberusError::Trading {
        message: "Same message".to_string(),
    };

    let error2 = CerberusError::Trading {
        message: "Same message".to_string(),
    };

    let error3 = CerberusError::Trading {
        message: "Different message".to_string(),
    };

    // Note: CerberusError doesn't implement PartialEq by default
    // This test verifies the structure is consistent
    match (&error1, &error2, &error3) {
        (
            CerberusError::Trading { message: m1 },
            CerberusError::Trading { message: m2 },
            CerberusError::Trading { message: m3 },
        ) => {
            assert_eq!(m1, m2);
            assert_ne!(m1, m3);
        }
        _ => panic!("Error structure mismatch"),
    }
}
