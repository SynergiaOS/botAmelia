use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::risk::TradeSide;

/// Moduł wykonywania transakcji z integracją Sentry
// pub mod executor;
// pub mod orders;
// pub mod manager;

// pub use executor::TradeExecutor;
// pub use orders::*;
// pub use manager::PositionManager;

/// Typ zlecenia
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OrderType {
    Market,
    Limit,
    StopLoss,
    TakeProfit,
}

/// Zlecenie handlowe
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeOrder {
    /// Identyfikator zlecenia
    pub id: String,

    /// Token/symbol
    pub token: String,

    /// Strona transakcji
    pub side: TradeSide,

    /// Rozmiar pozycji
    pub size: f64,

    /// Dźwignia
    pub leverage: u8,

    /// Typ zlecenia
    pub order_type: OrderType,

    /// Cena (dla zleceń limit)
    pub price: Option<f64>,

    /// Stop price (dla zleceń stop)
    pub stop_price: Option<f64>,

    /// Czas utworzenia
    pub created_at: i64,

    /// Timeout zlecenia (w sekundach)
    pub timeout: u64,

    /// Dodatkowe parametry
    pub metadata: serde_json::Value,
}

/// Wynik wykonania transakcji
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Czy transakcja została wykonana pomyślnie
    pub success: bool,

    /// Identyfikator transakcji
    pub transaction_id: Option<String>,

    /// Wykonana cena
    pub executed_price: Option<f64>,

    /// Wykonany rozmiar
    pub executed_size: Option<f64>,

    /// Opłaty
    pub fees: Option<f64>,

    /// Czas wykonania
    pub executed_at: Option<i64>,

    /// Komunikat błędu (jeśli wystąpił)
    pub error_message: Option<String>,

    /// Dodatkowe informacje
    pub metadata: serde_json::Value,
}

/// Trait dla executora transakcji
#[async_trait]
pub trait TradeExecutorTrait: Send + Sync {
    /// Wykonuje zlecenie handlowe
    async fn execute_trade(&self, order: &TradeOrder) -> Result<ExecutionResult>;

    /// Zamyka pozycję
    async fn close_position(&self, position_id: &str) -> Result<()>;

    /// Pobiera aktualną cenę
    async fn get_current_price(&self, token: &str) -> Result<f64>;

    /// Sprawdza status zlecenia
    async fn get_order_status(&self, order_id: &str) -> Result<OrderStatus>;

    /// Anuluje zlecenie
    async fn cancel_order(&self, order_id: &str) -> Result<()>;
}

/// Status zlecenia
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OrderStatus {
    Pending,
    PartiallyFilled,
    Filled,
    Cancelled,
    Rejected,
    Expired,
}

impl TradeOrder {
    /// Tworzy nowe zlecenie rynkowe
    pub fn market_order(token: String, side: TradeSide, size: f64, leverage: u8) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            token,
            side,
            size,
            leverage,
            order_type: OrderType::Market,
            price: None,
            stop_price: None,
            created_at: chrono::Utc::now().timestamp(),
            timeout: 30, // 30 sekund
            metadata: serde_json::Value::Null,
        }
    }

    /// Tworzy nowe zlecenie limitowe
    pub fn limit_order(
        token: String,
        side: TradeSide,
        size: f64,
        leverage: u8,
        price: f64,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            token,
            side,
            size,
            leverage,
            order_type: OrderType::Limit,
            price: Some(price),
            stop_price: None,
            created_at: chrono::Utc::now().timestamp(),
            timeout: 300, // 5 minut
            metadata: serde_json::Value::Null,
        }
    }

    /// Tworzy zlecenie stop-loss
    pub fn stop_loss_order(
        token: String,
        side: TradeSide,
        size: f64,
        leverage: u8,
        stop_price: f64,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            token,
            side,
            size,
            leverage,
            order_type: OrderType::StopLoss,
            price: None,
            stop_price: Some(stop_price),
            created_at: chrono::Utc::now().timestamp(),
            timeout: 3600, // 1 godzina
            metadata: serde_json::Value::Null,
        }
    }

    /// Sprawdza czy zlecenie wygasło
    pub fn is_expired(&self) -> bool {
        let now = chrono::Utc::now().timestamp();
        now - self.created_at > self.timeout as i64
    }

    /// Waliduje zlecenie
    pub fn validate(&self) -> Result<()> {
        if self.token.is_empty() {
            return Err(anyhow::anyhow!("Token cannot be empty"));
        }

        if self.size <= 0.0 {
            return Err(anyhow::anyhow!("Size must be greater than 0"));
        }

        if self.leverage == 0 {
            return Err(anyhow::anyhow!("Leverage must be greater than 0"));
        }

        if self.leverage > 100 {
            return Err(anyhow::anyhow!("Leverage cannot exceed 100"));
        }

        match self.order_type {
            OrderType::Limit => {
                if self.price.is_none() {
                    return Err(anyhow::anyhow!("Limit orders require a price"));
                }
                if self.price.unwrap() <= 0.0 {
                    return Err(anyhow::anyhow!("Price must be greater than 0"));
                }
            }
            OrderType::StopLoss | OrderType::TakeProfit => {
                if self.stop_price.is_none() {
                    return Err(anyhow::anyhow!("Stop orders require a stop price"));
                }
                if self.stop_price.unwrap() <= 0.0 {
                    return Err(anyhow::anyhow!("Stop price must be greater than 0"));
                }
            }
            OrderType::Market => {
                // Zlecenia rynkowe nie wymagają dodatkowej walidacji
            }
        }

        Ok(())
    }
}

impl ExecutionResult {
    /// Tworzy wynik sukcesu
    pub fn success(
        transaction_id: String,
        executed_price: f64,
        executed_size: f64,
        fees: f64,
    ) -> Self {
        Self {
            success: true,
            transaction_id: Some(transaction_id),
            executed_price: Some(executed_price),
            executed_size: Some(executed_size),
            fees: Some(fees),
            executed_at: Some(chrono::Utc::now().timestamp()),
            error_message: None,
            metadata: serde_json::Value::Null,
        }
    }

    /// Tworzy wynik błędu
    pub fn error(error_message: String) -> Self {
        Self {
            success: false,
            transaction_id: None,
            executed_price: None,
            executed_size: None,
            fees: None,
            executed_at: None,
            error_message: Some(error_message),
            metadata: serde_json::Value::Null,
        }
    }

    /// Sprawdza czy wykonanie było częściowe
    pub fn is_partial(&self, original_size: f64) -> bool {
        if let Some(executed_size) = self.executed_size {
            executed_size < original_size && executed_size > 0.0
        } else {
            false
        }
    }
}

/// Statystyki tradingu
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingStats {
    /// Całkowita liczba transakcji
    pub total_trades: u64,

    /// Liczba udanych transakcji
    pub successful_trades: u64,

    /// Liczba nieudanych transakcji
    pub failed_trades: u64,

    /// Całkowity P&L
    pub total_pnl: f64,

    /// Najlepszy trade
    pub best_trade: f64,

    /// Najgorszy trade
    pub worst_trade: f64,

    /// Średni czas wykonania (w ms)
    pub avg_execution_time: f64,

    /// Całkowite opłaty
    pub total_fees: f64,

    /// Współczynnik sukcesu
    pub success_rate: f64,

    /// Ostatnia aktualizacja
    pub last_updated: i64,
}

impl Default for TradingStats {
    fn default() -> Self {
        Self {
            total_trades: 0,
            successful_trades: 0,
            failed_trades: 0,
            total_pnl: 0.0,
            best_trade: f64::NEG_INFINITY,
            worst_trade: f64::INFINITY,
            avg_execution_time: 0.0,
            total_fees: 0.0,
            success_rate: 0.0,
            last_updated: chrono::Utc::now().timestamp(),
        }
    }
}

impl TradingStats {
    /// Aktualizuje statystyki po wykonaniu transakcji
    pub fn update_for_trade(&mut self, result: &ExecutionResult, execution_time_ms: f64, pnl: f64) {
        self.total_trades += 1;

        // Aktualizacja P&L niezależnie od sukcesu
        self.total_pnl += pnl;

        // Aktualizacja najlepszej i najgorszej transakcji
        if self.total_trades == 1 {
            // Pierwszy trade - ustaw jako najlepszy i najgorszy
            self.best_trade = pnl;
            self.worst_trade = pnl;
        } else {
            if pnl > self.best_trade {
                self.best_trade = pnl;
            }
            if pnl < self.worst_trade {
                self.worst_trade = pnl;
            }
        }

        if result.success {
            self.successful_trades += 1;

            if let Some(fees) = result.fees {
                self.total_fees += fees;
            }
        } else {
            self.failed_trades += 1;
        }

        // Aktualizacja średniego czasu wykonania
        self.avg_execution_time = (self.avg_execution_time * (self.total_trades - 1) as f64
            + execution_time_ms)
            / self.total_trades as f64;

        // Aktualizacja współczynnika sukcesu
        self.success_rate = self.successful_trades as f64 / self.total_trades as f64;

        self.last_updated = chrono::Utc::now().timestamp();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::risk::TradeSide;

    #[test]
    fn test_market_order_creation() {
        let order = TradeOrder::market_order("BONK".to_string(), TradeSide::Long, 100.0, 5);

        assert_eq!(order.token, "BONK");
        assert_eq!(order.side, TradeSide::Long);
        assert_eq!(order.size, 100.0);
        assert_eq!(order.leverage, 5);
        assert_eq!(order.order_type, OrderType::Market);
        assert!(order.price.is_none());
        assert!(order.stop_price.is_none());
        assert_eq!(order.timeout, 30);
        assert!(!order.id.is_empty());
    }

    #[test]
    fn test_limit_order_creation() {
        let order = TradeOrder::limit_order("BONK".to_string(), TradeSide::Short, 50.0, 10, 0.001);

        assert_eq!(order.token, "BONK");
        assert_eq!(order.side, TradeSide::Short);
        assert_eq!(order.size, 50.0);
        assert_eq!(order.leverage, 10);
        assert_eq!(order.order_type, OrderType::Limit);
        assert_eq!(order.price, Some(0.001));
        assert!(order.stop_price.is_none());
        assert_eq!(order.timeout, 300);
    }

    #[test]
    fn test_stop_loss_order_creation() {
        let order =
            TradeOrder::stop_loss_order("TEST".to_string(), TradeSide::Long, 75.0, 3, 0.0008);

        assert_eq!(order.order_type, OrderType::StopLoss);
        assert!(order.price.is_none());
        assert_eq!(order.stop_price, Some(0.0008));
        assert_eq!(order.timeout, 3600);
    }

    #[test]
    fn test_order_validation_valid_market() {
        let order = TradeOrder::market_order("TEST".to_string(), TradeSide::Long, 100.0, 5);

        assert!(order.validate().is_ok());
    }

    #[test]
    fn test_order_validation_empty_token() {
        let order = TradeOrder::market_order("".to_string(), TradeSide::Long, 100.0, 5);

        assert!(order.validate().is_err());
    }

    #[test]
    fn test_order_validation_zero_size() {
        let order = TradeOrder::market_order("TEST".to_string(), TradeSide::Long, 0.0, 5);

        assert!(order.validate().is_err());
    }

    #[test]
    fn test_order_validation_negative_size() {
        let order = TradeOrder::market_order("TEST".to_string(), TradeSide::Long, -100.0, 5);

        assert!(order.validate().is_err());
    }

    #[test]
    fn test_order_validation_zero_leverage() {
        let order = TradeOrder::market_order("TEST".to_string(), TradeSide::Long, 100.0, 0);

        assert!(order.validate().is_err());
    }

    #[test]
    fn test_order_validation_excessive_leverage() {
        let order = TradeOrder::market_order("TEST".to_string(), TradeSide::Long, 100.0, 150);

        assert!(order.validate().is_err());
    }

    #[test]
    fn test_limit_order_validation_no_price() {
        let mut order =
            TradeOrder::limit_order("TEST".to_string(), TradeSide::Long, 100.0, 5, 0.001);
        order.price = None;

        assert!(order.validate().is_err());
    }

    #[test]
    fn test_limit_order_validation_negative_price() {
        let order = TradeOrder::limit_order("TEST".to_string(), TradeSide::Long, 100.0, 5, -0.001);

        assert!(order.validate().is_err());
    }

    #[test]
    fn test_stop_order_validation_no_stop_price() {
        let mut order =
            TradeOrder::stop_loss_order("TEST".to_string(), TradeSide::Long, 100.0, 5, 0.0008);
        order.stop_price = None;

        assert!(order.validate().is_err());
    }

    #[test]
    fn test_order_expiration() {
        let mut order = TradeOrder::market_order("TEST".to_string(), TradeSide::Long, 100.0, 5);

        // Fresh order should not be expired
        assert!(!order.is_expired());

        // Simulate old order
        order.created_at = chrono::Utc::now().timestamp() - 3600; // 1 hour ago
        order.timeout = 30; // 30 seconds timeout

        assert!(order.is_expired());
    }

    #[test]
    fn test_execution_result_success() {
        let result = ExecutionResult::success("tx123".to_string(), 0.001, 100.0, 0.5);

        assert!(result.success);
        assert_eq!(result.transaction_id, Some("tx123".to_string()));
        assert_eq!(result.executed_price, Some(0.001));
        assert_eq!(result.executed_size, Some(100.0));
        assert_eq!(result.fees, Some(0.5));
        assert!(result.executed_at.is_some());
        assert!(result.error_message.is_none());
    }

    #[test]
    fn test_execution_result_error() {
        let result = ExecutionResult::error("Insufficient funds".to_string());

        assert!(!result.success);
        assert!(result.transaction_id.is_none());
        assert!(result.executed_price.is_none());
        assert!(result.executed_size.is_none());
        assert!(result.fees.is_none());
        assert!(result.executed_at.is_none());
        assert_eq!(result.error_message, Some("Insufficient funds".to_string()));
    }

    #[test]
    fn test_execution_result_partial() {
        let result = ExecutionResult::success(
            "tx123".to_string(),
            0.001,
            75.0, // Partial fill
            0.5,
        );

        assert!(result.is_partial(100.0)); // Original size was 100
        assert!(!result.is_partial(75.0)); // Not partial if same size
        assert!(!result.is_partial(50.0)); // Not partial if executed more than requested
    }

    #[test]
    fn test_trading_stats_default() {
        let stats = TradingStats::default();

        assert_eq!(stats.total_trades, 0);
        assert_eq!(stats.successful_trades, 0);
        assert_eq!(stats.failed_trades, 0);
        assert_eq!(stats.total_pnl, 0.0);
        assert_eq!(stats.best_trade, f64::NEG_INFINITY);
        assert_eq!(stats.worst_trade, f64::INFINITY);
        assert_eq!(stats.avg_execution_time, 0.0);
        assert_eq!(stats.total_fees, 0.0);
        assert_eq!(stats.success_rate, 0.0);
    }

    #[test]
    fn test_trading_stats_update_successful_trade() {
        let mut stats = TradingStats::default();
        let result = ExecutionResult::success("tx123".to_string(), 0.001, 100.0, 0.5);

        stats.update_for_trade(&result, 50.0, 25.0); // 50ms execution, $25 profit

        assert_eq!(stats.total_trades, 1);
        assert_eq!(stats.successful_trades, 1);
        assert_eq!(stats.failed_trades, 0);
        assert_eq!(stats.total_pnl, 25.0);
        assert_eq!(stats.best_trade, 25.0);
        assert_eq!(stats.worst_trade, 25.0);
        assert_eq!(stats.avg_execution_time, 50.0);
        assert_eq!(stats.total_fees, 0.5);
        assert_eq!(stats.success_rate, 1.0);
    }

    #[test]
    fn test_trading_stats_update_failed_trade() {
        let mut stats = TradingStats::default();
        let result = ExecutionResult::error("Network error".to_string());

        stats.update_for_trade(&result, 100.0, 0.0);

        assert_eq!(stats.total_trades, 1);
        assert_eq!(stats.successful_trades, 0);
        assert_eq!(stats.failed_trades, 1);
        assert_eq!(stats.total_pnl, 0.0);
        assert_eq!(stats.success_rate, 0.0);
    }

    #[test]
    fn test_trading_stats_multiple_trades() {
        let mut stats = TradingStats::default();

        // Successful trade
        let success_result = ExecutionResult::success("tx1".to_string(), 0.001, 100.0, 0.5);
        stats.update_for_trade(&success_result, 50.0, 20.0);

        // Failed trade
        let fail_result = ExecutionResult::error("Error".to_string());
        stats.update_for_trade(&fail_result, 75.0, 0.0);

        // Another successful trade with loss
        let loss_result = ExecutionResult::success("tx2".to_string(), 0.001, 100.0, 0.3);
        stats.update_for_trade(&loss_result, 25.0, -10.0);

        assert_eq!(stats.total_trades, 3);
        assert_eq!(stats.successful_trades, 2);
        assert_eq!(stats.failed_trades, 1);
        assert_eq!(stats.total_pnl, 10.0); // 20 - 10
        assert_eq!(stats.best_trade, 20.0);
        assert_eq!(stats.worst_trade, -10.0);
        assert_eq!(stats.avg_execution_time, 50.0); // (50 + 75 + 25) / 3
        assert_eq!(stats.total_fees, 0.8); // 0.5 + 0.3
        assert!((stats.success_rate - 0.6666666666666666).abs() < 0.0001); // 2/3
    }

    #[test]
    fn test_order_type_equality() {
        assert_eq!(OrderType::Market, OrderType::Market);
        assert_eq!(OrderType::Limit, OrderType::Limit);
        assert_ne!(OrderType::Market, OrderType::Limit);
    }

    #[test]
    fn test_order_status_equality() {
        assert_eq!(OrderStatus::Pending, OrderStatus::Pending);
        assert_eq!(OrderStatus::Filled, OrderStatus::Filled);
        assert_ne!(OrderStatus::Pending, OrderStatus::Filled);
    }
}
