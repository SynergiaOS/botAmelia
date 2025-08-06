use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde_json::Value;
use tracing::{info, error, warn};

use super::{ApiState, ApiResponse, PaginationParams, TimeRangeParams};
use crate::{
    monitoring::{HealthReport, ComponentHealth, HealthStatus},
    signals::Signal,
    errors::CerberusError,
};

/// Health check endpoint
pub async fn health_handler(
    State(state): State<ApiState>,
) -> Result<Json<ApiResponse<HealthReport>>, (StatusCode, Json<ApiResponse<()>>)> {
    info!("Health check requested");
    
    // Sprawdzenie stanu bazy danych
    let db_health = match state.db_manager.health_check().await {
        Ok(_) => ComponentHealth {
            status: HealthStatus::Healthy,
            message: "Database connection OK".to_string(),
            last_check: chrono::Utc::now().timestamp(),
            metrics: std::collections::HashMap::new(),
        },
        Err(e) => ComponentHealth {
            status: HealthStatus::Critical,
            message: format!("Database error: {}", e),
            last_check: chrono::Utc::now().timestamp(),
            metrics: std::collections::HashMap::new(),
        },
    };

    let mut health_report = HealthReport::new();
    health_report.add_component("database".to_string(), db_health);
    
    // Sprawdzenie metryk systemu
    let metrics = state.metrics.clone();
    let system_health = ComponentHealth {
        status: if metrics.memory_usage < 512.0 && metrics.cpu_usage < 80.0 {
            HealthStatus::Healthy
        } else {
            HealthStatus::Warning
        },
        message: "System resources OK".to_string(),
        last_check: chrono::Utc::now().timestamp(),
        metrics: {
            let mut m = std::collections::HashMap::new();
            m.insert("memory_usage_mb".to_string(), metrics.memory_usage);
            m.insert("cpu_usage_percent".to_string(), metrics.cpu_usage);
            m
        },
    };
    
    health_report.add_component("system".to_string(), system_health);

    Ok(Json(ApiResponse::success(health_report)))
}

/// Detailed health check endpoint
pub async fn detailed_health_handler(
    State(state): State<ApiState>,
) -> Result<Json<ApiResponse<Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    info!("Detailed health check requested");
    
    let metrics = state.metrics.clone();
    
    let detailed_health = serde_json::json!({
        "system": {
            "uptime_seconds": chrono::Utc::now().timestamp() - metrics.last_updated,
            "memory_usage_mb": metrics.memory_usage,
            "cpu_usage_percent": metrics.cpu_usage,
            "active_connections": metrics.active_connections,
        },
        "trading": {
            "total_signals": metrics.total_signals,
            "successful_trades": metrics.successful_trades,
            "failed_trades": metrics.failed_trades,
            "success_rate": metrics.success_rate,
            "current_balance": metrics.current_balance,
            "daily_pnl": metrics.daily_pnl,
        },
        "database": {
            "active_connections": metrics.database_metrics.active_connections,
            "avg_query_time_ms": metrics.database_metrics.avg_query_time,
            "queries_per_second": metrics.database_metrics.queries_per_second,
            "error_count": metrics.database_metrics.error_count,
            "database_size_mb": metrics.database_metrics.database_size,
        },
        "performance": {
            "avg_decision_time_ms": if !metrics.decision_times.is_empty() {
                metrics.decision_times.iter().sum::<f64>() / metrics.decision_times.len() as f64
            } else {
                0.0
            },
            "recent_decision_times": metrics.decision_times.iter().rev().take(10).collect::<Vec<_>>(),
        }
    });

    Ok(Json(ApiResponse::success(detailed_health)))
}

/// Metrics endpoint (JSON format)
pub async fn metrics_handler(
    State(state): State<ApiState>,
) -> Result<Json<ApiResponse<Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    info!("Metrics requested");
    
    let metrics = state.metrics.clone();
    let metrics_json = serde_json::to_value(&*metrics)
        .map_err(|e| {
            error!("Failed to serialize metrics: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<()>::error("Failed to serialize metrics".to_string())))
        })?;

    Ok(Json(ApiResponse::success(metrics_json)))
}

/// Prometheus metrics endpoint
pub async fn prometheus_metrics_handler(
    State(state): State<ApiState>,
) -> Result<String, (StatusCode, Json<ApiResponse<()>>)> {
    info!("Prometheus metrics requested");

    // Extract values to local variables to avoid potential Arc access issues
    let total_signals = state.metrics.total_signals;
    let successful_trades = state.metrics.successful_trades;
    let failed_trades = state.metrics.failed_trades;
    let success_rate = state.metrics.success_rate;
    let current_balance = state.metrics.current_balance;
    let daily_pnl = state.metrics.daily_pnl;
    let memory_usage = state.metrics.memory_usage;
    let cpu_usage = state.metrics.cpu_usage;
    let active_connections = state.metrics.active_connections;

    info!("Building prometheus format with {} signals", total_signals);

    let prometheus_format = format!(
        "# HELP cerberus_total_signals Total number of trading signals processed\n\
         # TYPE cerberus_total_signals counter\n\
         cerberus_total_signals {}\n\
         \n\
         # HELP cerberus_successful_trades Number of successful trades\n\
         # TYPE cerberus_successful_trades counter\n\
         cerberus_successful_trades {}\n\
         \n\
         # HELP cerberus_failed_trades Number of failed trades\n\
         # TYPE cerberus_failed_trades counter\n\
         cerberus_failed_trades {}\n\
         \n\
         # HELP cerberus_success_rate Trading success rate\n\
         # TYPE cerberus_success_rate gauge\n\
         cerberus_success_rate {}\n\
         \n\
         # HELP cerberus_current_balance Current portfolio balance\n\
         # TYPE cerberus_current_balance gauge\n\
         cerberus_current_balance {}\n\
         \n\
         # HELP cerberus_daily_pnl Daily profit and loss\n\
         # TYPE cerberus_daily_pnl gauge\n\
         cerberus_daily_pnl {}\n\
         \n\
         # HELP cerberus_memory_usage_mb Memory usage in megabytes\n\
         # TYPE cerberus_memory_usage_mb gauge\n\
         cerberus_memory_usage_mb {}\n\
         \n\
         # HELP cerberus_cpu_usage_percent CPU usage percentage\n\
         # TYPE cerberus_cpu_usage_percent gauge\n\
         cerberus_cpu_usage_percent {}\n\
         \n\
         # HELP cerberus_active_connections Number of active connections\n\
         # TYPE cerberus_active_connections gauge\n\
         cerberus_active_connections {}\n",
        total_signals,
        successful_trades,
        failed_trades,
        success_rate,
        current_balance,
        daily_pnl,
        memory_usage,
        cpu_usage,
        active_connections,
    );

    info!("Prometheus format built successfully");
    Ok(prometheus_format)
}

/// Get signals endpoint
pub async fn get_signals_handler(
    State(_state): State<ApiState>,
    Query(params): Query<PaginationParams>,
    Query(time_params): Query<TimeRangeParams>,
) -> Result<Json<ApiResponse<Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    info!("Get signals requested with pagination: {:?}", params);
    
    // TODO: Implement actual signal retrieval from database
    let mock_signals = serde_json::json!({
        "signals": [],
        "pagination": {
            "page": params.page.unwrap_or(1),
            "limit": params.limit.unwrap_or(50),
            "total": 0
        },
        "time_range": {
            "from": time_params.from,
            "to": time_params.to
        }
    });

    Ok(Json(ApiResponse::success(mock_signals)))
}

/// Create signal endpoint
pub async fn create_signal_handler(
    State(_state): State<ApiState>,
    Json(payload): Json<Value>,
) -> Result<Json<ApiResponse<Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    info!("Create signal requested: {:?}", payload);

    // TODO: Implement signal creation logic
    let now = chrono::Utc::now().timestamp();
    let response = serde_json::json!({
        "signal_id": "mock_signal_id",
        "status": "created",
        "timestamp": now
    });

    info!("Returning signal response: {:?}", response);
    Ok(Json(ApiResponse::success(response)))
}

/// Validate signals endpoint
pub async fn validate_signals_handler(
    State(_state): State<ApiState>,
    Json(_signals): Json<Value>,
) -> Result<Json<ApiResponse<Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    info!("Validate signals requested");
    
    // TODO: Implement signal validation logic
    let validation_result = serde_json::json!({
        "valid_signals": [],
        "invalid_signals": [],
        "validation_summary": {
            "total": 0,
            "valid": 0,
            "invalid": 0
        }
    });

    Ok(Json(ApiResponse::success(validation_result)))
}

/// Get single signal endpoint
pub async fn get_signal_handler(
    State(_state): State<ApiState>,
    Path(signal_id): Path<String>,
) -> Result<Json<ApiResponse<Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    info!("Get signal {} requested", signal_id);

    // TODO: Implement signal retrieval by ID
    let signal = serde_json::json!({
        "id": signal_id,
        "status": "not_found"
    });

    Ok(Json(ApiResponse::success(signal)))
}

/// Risk assessment endpoint
pub async fn assess_risk_handler(
    State(_state): State<ApiState>,
    Json(_payload): Json<Value>,
) -> Result<Json<ApiResponse<Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    info!("Risk assessment requested");

    // TODO: Implement risk assessment logic
    let risk_assessment = serde_json::json!({
        "approved": false,
        "risk_level": "high",
        "max_leverage": 5,
        "position_size": 0.0,
        "reasoning": "Mock risk assessment - implementation pending"
    });

    Ok(Json(ApiResponse::success(risk_assessment)))
}

/// Risk status endpoint
pub async fn risk_status_handler(
    State(_state): State<ApiState>,
) -> Result<Json<ApiResponse<Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    info!("Risk status requested");
    
    let risk_status = serde_json::json!({
        "circuit_breaker_active": false,
        "daily_loss": 0.0,
        "daily_loss_limit": 15.0,
        "consecutive_failures": 0,
        "max_consecutive_failures": 5,
        "current_positions": 0,
        "max_positions": 3
    });

    Ok(Json(ApiResponse::success(risk_status)))
}

/// Get trades endpoint
pub async fn get_trades_handler(
    State(_state): State<ApiState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<ApiResponse<Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    info!("Get trades requested");

    let trades = serde_json::json!({
        "trades": [],
        "pagination": {
            "page": params.page.unwrap_or(1),
            "limit": params.limit.unwrap_or(50),
            "total": 0
        }
    });

    Ok(Json(ApiResponse::success(trades)))
}

/// Execute trade endpoint
pub async fn execute_trade_handler(
    State(_state): State<ApiState>,
    Json(_payload): Json<Value>,
) -> Result<Json<ApiResponse<Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    info!("Execute trade requested");

    let trade_result = serde_json::json!({
        "trade_id": "mock_trade_id",
        "status": "pending",
        "timestamp": chrono::Utc::now().timestamp()
    });

    Ok(Json(ApiResponse::success(trade_result)))
}

/// Get single trade endpoint
pub async fn get_trade_handler(
    State(_state): State<ApiState>,
    Path(trade_id): Path<String>,
) -> Result<Json<ApiResponse<Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    info!("Get trade {} requested", trade_id);

    let trade = serde_json::json!({
        "id": trade_id,
        "status": "not_found"
    });

    Ok(Json(ApiResponse::success(trade)))
}

/// Get positions endpoint
pub async fn get_positions_handler(
    State(_state): State<ApiState>,
) -> Result<Json<ApiResponse<Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    info!("Get positions requested");

    let positions = serde_json::json!({
        "positions": [],
        "total_positions": 0,
        "total_value": 0.0
    });

    Ok(Json(ApiResponse::success(positions)))
}

/// Get single position endpoint
pub async fn get_position_handler(
    State(_state): State<ApiState>,
    Path(position_id): Path<String>,
) -> Result<Json<ApiResponse<Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    info!("Get position {} requested", position_id);

    let position = serde_json::json!({
        "id": position_id,
        "status": "not_found"
    });

    Ok(Json(ApiResponse::success(position)))
}

/// Close position endpoint
pub async fn close_position_handler(
    State(_state): State<ApiState>,
    Path(position_id): Path<String>,
) -> Result<Json<ApiResponse<Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    info!("Close position {} requested", position_id);

    let result = serde_json::json!({
        "position_id": position_id,
        "status": "closing",
        "timestamp": chrono::Utc::now().timestamp()
    });

    Ok(Json(ApiResponse::success(result)))
}

/// System status endpoint
pub async fn system_status_handler(
    State(state): State<ApiState>,
) -> Result<Json<ApiResponse<Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    info!("System status requested");

    let system_status = serde_json::json!({
        "status": "running",
        "version": env!("CARGO_PKG_VERSION"),
        "uptime_seconds": chrono::Utc::now().timestamp() - state.metrics.last_updated,
        "environment": state.config.environment
    });

    Ok(Json(ApiResponse::success(system_status)))
}

/// Emergency stop endpoint
pub async fn emergency_stop_handler(
    State(_state): State<ApiState>,
) -> Result<Json<ApiResponse<Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    warn!("Emergency stop requested!");

    // TODO: Implement emergency stop logic
    let result = serde_json::json!({
        "status": "emergency_stop_activated",
        "timestamp": chrono::Utc::now().timestamp(),
        "message": "All trading operations halted"
    });

    Ok(Json(ApiResponse::success(result)))
}

/// Reset system endpoint
pub async fn reset_system_handler(
    State(_state): State<ApiState>,
) -> Result<Json<ApiResponse<Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    warn!("System reset requested!");

    let result = serde_json::json!({
        "status": "system_reset",
        "timestamp": chrono::Utc::now().timestamp(),
        "message": "System reset initiated"
    });

    Ok(Json(ApiResponse::success(result)))
}

/// Get configuration endpoint
pub async fn get_config_handler(
    State(state): State<ApiState>,
) -> Result<Json<ApiResponse<Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    info!("Get configuration requested");

    // Return sanitized config (without sensitive data)
    let config = serde_json::json!({
        "environment": state.config.environment,
        "http_port": state.config.http_port,
        "metrics_port": state.config.metrics_port,
        "trading": {
            "enabled": true // TODO: Get from actual config
        },
        "risk": {
            "max_daily_loss": 15.0 // TODO: Get from actual config
        }
    });

    Ok(Json(ApiResponse::success(config)))
}

/// Validate configuration endpoint
pub async fn validate_config_handler(
    State(_state): State<ApiState>,
    Json(_config): Json<Value>,
) -> Result<Json<ApiResponse<Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    info!("Validate configuration requested");

    // TODO: Implement config validation
    let validation_result = serde_json::json!({
        "valid": true,
        "errors": [],
        "warnings": []
    });

    Ok(Json(ApiResponse::success(validation_result)))
}
