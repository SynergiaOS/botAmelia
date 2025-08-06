use anyhow::Result;
use axum::{
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{
    cors::CorsLayer,
    trace::TraceLayer,
};
use tracing::info;

use crate::errors::CerberusError;

use crate::{
    config::Config,
    database::DatabaseManager,
    monitoring::SystemMetrics,
};

pub mod handlers;
pub mod models;
pub mod middleware;

pub use handlers::*;
// pub use models::*;

/// Stan aplikacji dla API
#[derive(Clone)]
pub struct ApiState {
    pub config: Arc<Config>,
    pub db_manager: Arc<DatabaseManager>,
    pub metrics: Arc<SystemMetrics>,
}

/// Serwer HTTP API dla integracji z Kestra
pub struct ApiServer {
    state: ApiState,
    listener: Option<TcpListener>,
}

impl ApiServer {
    /// Tworzy nowy serwer API
    pub async fn new(
        config: Arc<Config>,
        db_manager: Arc<DatabaseManager>,
        metrics: Arc<SystemMetrics>,
    ) -> Result<Self> {
        let state = ApiState {
            config,
            db_manager,
            metrics,
        };

        Ok(Self {
            state,
            listener: None,
        })
    }

    /// Inicjalizuje listener
    pub async fn bind(&mut self) -> Result<()> {
        let addr = format!("0.0.0.0:{}", self.state.config.http_port);
        let listener = TcpListener::bind(&addr).await?;
        
        info!("API server bound to {}", addr);
        self.listener = Some(listener);
        Ok(())
    }

    /// Tworzy router z wszystkimi endpointami
    fn create_router(&self) -> Router {
        Router::new()
            // Health endpoints
            .route("/health", get(health_handler))
            .route("/health/detailed", get(detailed_health_handler))
            
            // Metrics endpoints
            .route("/metrics", get(metrics_handler))
            .route("/metrics/prometheus", get(prometheus_metrics_handler))
            
            // Signal endpoints
            .route("/api/signals", get(get_signals_handler))
            .route("/api/signals", post(create_signal_handler))
            .route("/api/signals/validate", post(validate_signals_handler))
            .route("/api/signals/:id", get(get_signal_handler))
            
            // Risk management endpoints
            .route("/api/risk/assess", post(assess_risk_handler))
            .route("/api/risk/status", get(risk_status_handler))
            
            // Trading endpoints
            .route("/api/trades", get(get_trades_handler))
            .route("/api/trades", post(execute_trade_handler))
            .route("/api/trades/:id", get(get_trade_handler))
            
            // Position endpoints
            .route("/api/positions", get(get_positions_handler))
            .route("/api/positions/:id", get(get_position_handler))
            .route("/api/positions/:id/close", post(close_position_handler))
            
            // System control endpoints
            .route("/api/system/status", get(system_status_handler))
            .route("/api/system/emergency-stop", post(emergency_stop_handler))
            .route("/api/system/reset", post(reset_system_handler))
            
            // Configuration endpoints
            .route("/api/config", get(get_config_handler))
            .route("/api/config/validate", post(validate_config_handler))
            
            .layer(
                ServiceBuilder::new()
                    .layer(TraceLayer::new_for_http())
                    .layer(CorsLayer::permissive())
            )
            .with_state(self.state.clone())
    }

    /// Uruchamia serwer API
    pub async fn serve(&mut self) -> Result<()> {
        if self.listener.is_none() {
            self.bind().await?;
        }

        let listener = self.listener.take().unwrap();
        let router = self.create_router();

        info!("Starting API server on port {}", self.state.config.http_port);
        
        axum::serve(listener, router)
            .await
            .map_err(|e| anyhow::anyhow!("API server error: {}", e))?;

        Ok(())
    }
}

/// Podstawowy response dla API
#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub timestamp: i64,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }

    pub fn error(message: String) -> ApiResponse<()> {
        ApiResponse {
            success: false,
            data: None,
            error: Some(message),
            timestamp: chrono::Utc::now().timestamp(),
        }
    }
}

/// Konwersja błędów na HTTP responses
impl From<CerberusError> for (StatusCode, Json<ApiResponse<()>>) {
    fn from(err: CerberusError) -> Self {
        let status = match err {
            CerberusError::Validation { .. } => StatusCode::BAD_REQUEST,
            CerberusError::Database { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            CerberusError::Configuration { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            CerberusError::Network { .. } => StatusCode::SERVICE_UNAVAILABLE,
            CerberusError::Authentication { .. } => StatusCode::UNAUTHORIZED,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let response = ApiResponse::<()>::error(err.to_string());
        (status, Json(response))
    }
}

/// Parametry paginacji
#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    pub page: Option<u32>,
    pub limit: Option<u32>,
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self {
            page: Some(1),
            limit: Some(50),
        }
    }
}

/// Parametry filtrowania czasowego
#[derive(Deserialize)]
pub struct TimeRangeParams {
    pub from: Option<i64>,
    pub to: Option<i64>,
}
