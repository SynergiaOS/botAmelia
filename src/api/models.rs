use serde::{Deserialize, Serialize};

/// Request model for signal creation
#[derive(Debug, Deserialize)]
pub struct CreateSignalRequest {
    pub token: String,
    pub source: String,
    pub confidence: String,
    pub price: f64,
    pub volume: f64,
    pub metadata: Option<serde_json::Value>,
}

/// Response model for signal creation
#[derive(Debug, Serialize)]
pub struct CreateSignalResponse {
    pub signal_id: String,
    pub status: String,
    pub timestamp: i64,
}

/// Request model for risk assessment
#[derive(Debug, Deserialize)]
pub struct RiskAssessmentRequest {
    pub signals: Vec<serde_json::Value>,
    pub risk_threshold: Option<f64>,
    pub portfolio_balance: Option<f64>,
}

/// Response model for risk assessment
#[derive(Debug, Serialize)]
pub struct RiskAssessmentResponse {
    pub approved: bool,
    pub risk_level: String,
    pub max_leverage: u8,
    pub position_size: f64,
    pub reasoning: String,
    pub confidence_score: f64,
}

/// Request model for trade execution
#[derive(Debug, Deserialize)]
pub struct ExecuteTradeRequest {
    pub token: String,
    pub side: String, // "buy" or "sell"
    pub size: f64,
    pub leverage: u8,
    pub order_type: String, // "market" or "limit"
    pub price: Option<f64>, // For limit orders
}

/// Response model for trade execution
#[derive(Debug, Serialize)]
pub struct ExecuteTradeResponse {
    pub trade_id: String,
    pub status: String,
    pub execution_price: Option<f64>,
    pub timestamp: i64,
}

/// Model for position information
#[derive(Debug, Serialize)]
pub struct PositionInfo {
    pub id: String,
    pub token: String,
    pub side: String,
    pub size: f64,
    pub leverage: u8,
    pub entry_price: f64,
    pub current_price: f64,
    pub pnl: f64,
    pub pnl_percentage: f64,
    pub liquidation_price: f64,
    pub status: String,
    pub opened_at: i64,
    pub updated_at: i64,
}

/// Model for trade information
#[derive(Debug, Serialize)]
pub struct TradeInfo {
    pub id: String,
    pub token: String,
    pub side: String,
    pub size: f64,
    pub leverage: u8,
    pub entry_price: f64,
    pub exit_price: Option<f64>,
    pub pnl: Option<f64>,
    pub status: String,
    pub opened_at: i64,
    pub closed_at: Option<i64>,
}

/// Model for system metrics summary
#[derive(Debug, Serialize)]
pub struct MetricsSummary {
    pub trading: TradingMetrics,
    pub system: SystemResourceMetrics,
    pub database: DatabaseMetrics,
    pub performance: PerformanceMetrics,
}

#[derive(Debug, Serialize)]
pub struct TradingMetrics {
    pub total_signals: u64,
    pub successful_trades: u64,
    pub failed_trades: u64,
    pub success_rate: f64,
    pub current_balance: f64,
    pub daily_pnl: f64,
    pub total_pnl: f64,
    pub active_positions: u32,
}

#[derive(Debug, Serialize)]
pub struct SystemResourceMetrics {
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
    pub active_connections: u32,
    pub uptime_seconds: u64,
}

#[derive(Debug, Serialize)]
pub struct DatabaseMetrics {
    pub active_connections: u32,
    pub avg_query_time_ms: f64,
    pub queries_per_second: f64,
    pub error_count: u64,
    pub database_size_mb: f64,
}

#[derive(Debug, Serialize)]
pub struct PerformanceMetrics {
    pub avg_decision_time_ms: f64,
    pub min_decision_time_ms: f64,
    pub max_decision_time_ms: f64,
    pub recent_decision_times: Vec<f64>,
}

/// Request model for configuration validation
#[derive(Debug, Deserialize)]
pub struct ValidateConfigRequest {
    pub config: serde_json::Value,
}

/// Response model for configuration validation
#[derive(Debug, Serialize)]
pub struct ValidateConfigResponse {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// Model for system status
#[derive(Debug, Serialize)]
pub struct SystemStatus {
    pub status: String,
    pub version: String,
    pub environment: String,
    pub uptime_seconds: u64,
    pub trading_enabled: bool,
    pub circuit_breaker_active: bool,
    pub last_health_check: i64,
}

/// Model for emergency stop response
#[derive(Debug, Serialize)]
pub struct EmergencyStopResponse {
    pub status: String,
    pub timestamp: i64,
    pub message: String,
    pub positions_closed: u32,
    pub orders_cancelled: u32,
}

/// Model for signal validation
#[derive(Debug, Serialize)]
pub struct SignalValidationResult {
    pub valid_signals: Vec<serde_json::Value>,
    pub invalid_signals: Vec<InvalidSignal>,
    pub validation_summary: ValidationSummary,
}

#[derive(Debug, Serialize)]
pub struct InvalidSignal {
    pub signal: serde_json::Value,
    pub errors: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ValidationSummary {
    pub total: u32,
    pub valid: u32,
    pub invalid: u32,
    pub validation_time_ms: f64,
}

/// Model for paginated responses
#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub pagination: PaginationInfo,
}

#[derive(Debug, Serialize)]
pub struct PaginationInfo {
    pub page: u32,
    pub limit: u32,
    pub total: u64,
    pub total_pages: u32,
    pub has_next: bool,
    pub has_prev: bool,
}

/// Model for time range filters
#[derive(Debug, Deserialize)]
pub struct TimeRangeFilter {
    pub from: Option<i64>,
    pub to: Option<i64>,
}

/// Model for alert information
#[derive(Debug, Serialize)]
pub struct AlertInfo {
    pub id: String,
    pub level: String,
    pub title: String,
    pub message: String,
    pub timestamp: i64,
    pub acknowledged: bool,
    pub source: String,
}

/// Request model for manual signal processing
#[derive(Debug, Deserialize)]
pub struct ProcessSignalRequest {
    pub signal_id: String,
    pub force_processing: Option<bool>,
    pub override_risk_check: Option<bool>,
}

/// Response model for signal processing
#[derive(Debug, Serialize)]
pub struct ProcessSignalResponse {
    pub signal_id: String,
    pub processing_status: String,
    pub decision: Option<String>,
    pub trade_executed: bool,
    pub processing_time_ms: f64,
    pub timestamp: i64,
}
