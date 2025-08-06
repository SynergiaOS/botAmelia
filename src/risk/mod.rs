use crate::signals::Signal;
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Moduł zarządzania ryzykiem z integracją Sentry
// pub mod manager;
// pub mod circuit_breaker;
// pub mod calculator;

// pub use manager::RiskManager;
// pub use circuit_breaker::CircuitBreaker;
// pub use calculator::LeverageCalculator;

/// Struktura portfela
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Portfolio {
    /// Aktualny balans (w USD)
    pub balance: f64,

    /// Equity (balans + unrealized PnL)
    pub equity: f64,

    /// Używana marża
    pub margin_used: f64,

    /// Dostępna marża
    pub margin_available: f64,

    /// Dzienny P&L
    pub daily_pnl: f64,

    /// Otwarte pozycje
    pub open_positions: Vec<Position>,

    /// Ostatnia aktualizacja
    pub last_updated: i64,
}

/// Struktura pozycji
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    /// Identyfikator pozycji
    pub id: String,

    /// Token/symbol
    pub token: String,

    /// Strona transakcji (Long/Short)
    pub side: TradeSide,

    /// Rozmiar pozycji
    pub size: f64,

    /// Dźwignia
    pub leverage: u8,

    /// Cena wejścia
    pub entry_price: f64,

    /// Aktualna cena
    pub current_price: f64,

    /// P&L (realized + unrealized)
    pub pnl: f64,

    /// Cena likwidacji
    pub liquidation_price: f64,

    /// Status pozycji
    pub status: PositionStatus,

    /// Czas otwarcia
    pub opened_at: i64,

    /// Czas zamknięcia (jeśli zamknięta)
    pub closed_at: Option<i64>,
}

/// Strona transakcji
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TradeSide {
    Long,
    Short,
}

/// Status pozycji
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PositionStatus {
    Open,
    Closing,
    Closed,
    Liquidated,
}

/// Ocena ryzyka
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    /// Czy transakcja jest zatwierdzona
    pub approved: bool,

    /// Maksymalna dźwignia
    pub max_leverage: u8,

    /// Zalecany rozmiar pozycji
    pub position_size: f64,

    /// Uzasadnienie decyzji
    pub reasoning: String,

    /// Poziom ryzyka (0-100)
    pub risk_score: u8,

    /// Ostrzeżenia
    pub warnings: Vec<String>,

    /// Czas oceny
    pub assessed_at: i64,
}

/// Trait dla managera ryzyka
#[async_trait]
pub trait RiskManagerTrait: Send + Sync {
    /// Ocenia ryzyko dla sygnału
    async fn evaluate_risk(&self, signal: &Signal, portfolio: &Portfolio) -> RiskAssessment;

    /// Sprawdza limity pozycji
    async fn check_position_limits(&self, new_position: &Position) -> bool;

    /// Oblicza rozmiar pozycji
    async fn calculate_position_size(&self, signal: &Signal, leverage: u8) -> f64;

    /// Sprawdza czy można otworzyć nową pozycję
    async fn can_open_position(&self, portfolio: &Portfolio) -> bool;

    /// Sprawdza czy pozycja wymaga zamknięcia
    async fn should_close_position(&self, position: &Position) -> bool;
}

impl Portfolio {
    /// Tworzy nowy portfel
    pub fn new(initial_balance: f64) -> Self {
        Self {
            balance: initial_balance,
            equity: initial_balance,
            margin_used: 0.0,
            margin_available: initial_balance,
            daily_pnl: 0.0,
            open_positions: Vec::new(),
            last_updated: chrono::Utc::now().timestamp(),
        }
    }

    /// Aktualizuje portfel
    pub fn update(&mut self) {
        // Obliczenie unrealized PnL
        let unrealized_pnl: f64 = self.open_positions.iter().map(|pos| pos.pnl).sum();

        // Aktualizacja equity
        self.equity = self.balance + unrealized_pnl;

        // Obliczenie używanej marży
        self.margin_used = self
            .open_positions
            .iter()
            .map(|pos| pos.size / pos.leverage as f64)
            .sum();

        // Aktualizacja dostępnej marży
        self.margin_available = self.equity - self.margin_used;

        self.last_updated = chrono::Utc::now().timestamp();
    }

    /// Dodaje pozycję do portfela
    pub fn add_position(&mut self, position: Position) {
        self.open_positions.push(position);
        self.update();
    }

    /// Usuwa pozycję z portfela
    pub fn remove_position(&mut self, position_id: &str) -> Option<Position> {
        if let Some(index) = self.open_positions.iter().position(|p| p.id == position_id) {
            let position = self.open_positions.remove(index);
            self.update();
            Some(position)
        } else {
            None
        }
    }

    /// Zwraca pozycję według ID
    pub fn get_position(&self, position_id: &str) -> Option<&Position> {
        self.open_positions.iter().find(|p| p.id == position_id)
    }

    /// Zwraca pozycję według ID (mutable)
    pub fn get_position_mut(&mut self, position_id: &str) -> Option<&mut Position> {
        self.open_positions.iter_mut().find(|p| p.id == position_id)
    }

    /// Sprawdza czy portfel jest w dobrej kondycji
    pub fn is_healthy(&self) -> bool {
        self.equity > 0.0 && self.margin_available >= 0.0
    }

    /// Zwraca współczynnik wykorzystania marży
    pub fn margin_utilization(&self) -> f64 {
        if self.equity <= 0.0 {
            return 1.0;
        }
        self.margin_used / self.equity
    }
}

impl Position {
    /// Tworzy nową pozycję
    pub fn new(token: String, side: TradeSide, size: f64, leverage: u8, entry_price: f64) -> Self {
        let liquidation_price = Self::calculate_liquidation_price(&side, entry_price, leverage);

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            token,
            side,
            size,
            leverage,
            entry_price,
            current_price: entry_price,
            pnl: 0.0,
            liquidation_price,
            status: PositionStatus::Open,
            opened_at: chrono::Utc::now().timestamp(),
            closed_at: None,
        }
    }

    /// Aktualizuje pozycję z nową ceną
    pub fn update_price(&mut self, new_price: f64) {
        self.current_price = new_price;
        self.pnl = self.calculate_pnl();
    }

    /// Oblicza P&L pozycji
    pub fn calculate_pnl(&self) -> f64 {
        let price_change = match self.side {
            TradeSide::Long => self.current_price - self.entry_price,
            TradeSide::Short => self.entry_price - self.current_price,
        };

        (price_change / self.entry_price) * self.size * self.leverage as f64
    }

    /// Oblicza cenę likwidacji
    pub fn calculate_liquidation_price(side: &TradeSide, entry_price: f64, leverage: u8) -> f64 {
        let liquidation_threshold = 0.95; // 95% marży

        match side {
            TradeSide::Long => entry_price * (1.0 - liquidation_threshold / leverage as f64),
            TradeSide::Short => entry_price * (1.0 + liquidation_threshold / leverage as f64),
        }
    }

    /// Sprawdza czy pozycja jest bliska likwidacji
    pub fn is_near_liquidation(&self, buffer_percent: f64) -> bool {
        let buffer = self.entry_price * buffer_percent;

        match self.side {
            TradeSide::Long => self.current_price <= self.liquidation_price + buffer,
            TradeSide::Short => self.current_price >= self.liquidation_price - buffer,
        }
    }

    /// Zamyka pozycję
    pub fn close(&mut self) {
        self.status = PositionStatus::Closed;
        self.closed_at = Some(chrono::Utc::now().timestamp());
    }

    /// Zwraca czas trwania pozycji w sekundach
    pub fn duration_seconds(&self) -> i64 {
        let end_time = self
            .closed_at
            .unwrap_or_else(|| chrono::Utc::now().timestamp());
        end_time - self.opened_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_portfolio_creation() {
        let portfolio = Portfolio::new(1000.0);

        assert_eq!(portfolio.balance, 1000.0);
        assert_eq!(portfolio.equity, 1000.0);
        assert_eq!(portfolio.margin_used, 0.0);
        assert_eq!(portfolio.margin_available, 1000.0);
        assert_eq!(portfolio.daily_pnl, 0.0);
        assert_eq!(portfolio.open_positions.len(), 0);
        assert!(portfolio.is_healthy());
    }

    #[test]
    fn test_portfolio_add_position() {
        let mut portfolio = Portfolio::new(1000.0);
        let position = Position::new("TEST".to_string(), TradeSide::Long, 100.0, 5, 0.001);

        portfolio.add_position(position);

        assert_eq!(portfolio.open_positions.len(), 1);
        assert_eq!(portfolio.margin_used, 20.0); // 100 / 5 = 20
        assert_eq!(portfolio.margin_available, 980.0); // 1000 - 20
    }

    #[test]
    fn test_portfolio_remove_position() {
        let mut portfolio = Portfolio::new(1000.0);
        let position = Position::new("TEST".to_string(), TradeSide::Long, 100.0, 5, 0.001);
        let position_id = position.id.clone();

        portfolio.add_position(position);
        assert_eq!(portfolio.open_positions.len(), 1);

        let removed = portfolio.remove_position(&position_id);
        assert!(removed.is_some());
        assert_eq!(portfolio.open_positions.len(), 0);
        assert_eq!(portfolio.margin_used, 0.0);
    }

    #[test]
    fn test_portfolio_margin_utilization() {
        let mut portfolio = Portfolio::new(1000.0);
        let position = Position::new(
            "TEST".to_string(),
            TradeSide::Long,
            200.0, // 20% of portfolio
            10,    // 10x leverage
            0.001,
        );

        portfolio.add_position(position);

        // Margin used = 200 / 10 = 20
        // Margin utilization = 20 / 1000 = 0.02 (2%)
        assert!((portfolio.margin_utilization() - 0.02).abs() < 0.001);
    }

    #[test]
    fn test_position_creation() {
        let position = Position::new("BONK".to_string(), TradeSide::Long, 100.0, 5, 0.001);

        assert_eq!(position.token, "BONK");
        assert_eq!(position.side, TradeSide::Long);
        assert_eq!(position.size, 100.0);
        assert_eq!(position.leverage, 5);
        assert_eq!(position.entry_price, 0.001);
        assert_eq!(position.current_price, 0.001);
        assert_eq!(position.pnl, 0.0);
        assert_eq!(position.status, PositionStatus::Open);
        assert!(!position.id.is_empty());
    }

    #[test]
    fn test_position_pnl_calculation_long() {
        let mut position = Position::new("TEST".to_string(), TradeSide::Long, 100.0, 5, 0.001);

        // Price increases by 20%
        position.update_price(0.0012);

        // Expected PnL = (0.0012 - 0.001) / 0.001 * 100 * 5 = 0.2 * 100 * 5 = 100
        assert!((position.pnl - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_position_pnl_calculation_short() {
        let mut position = Position::new("TEST".to_string(), TradeSide::Short, 100.0, 5, 0.001);

        // Price increases by 20% (bad for short)
        position.update_price(0.0012);

        // Expected PnL = (0.001 - 0.0012) / 0.001 * 100 * 5 = -0.2 * 100 * 5 = -100
        assert!((position.pnl - (-100.0)).abs() < 0.001);
    }

    #[test]
    fn test_liquidation_price_calculation_long() {
        let liquidation_price = Position::calculate_liquidation_price(&TradeSide::Long, 0.001, 5);

        // For long position with 5x leverage, liquidation at ~81% of entry price
        // 0.001 * (1 - 0.95/5) = 0.001 * 0.81 = 0.00081
        assert!((liquidation_price - 0.00081).abs() < 0.00001);
    }

    #[test]
    fn test_liquidation_price_calculation_short() {
        let liquidation_price = Position::calculate_liquidation_price(&TradeSide::Short, 0.001, 5);

        // For short position with 5x leverage, liquidation at ~119% of entry price
        // 0.001 * (1 + 0.95/5) = 0.001 * 1.19 = 0.00119
        assert!((liquidation_price - 0.00119).abs() < 0.00001);
    }

    #[test]
    fn test_position_near_liquidation() {
        let mut position = Position::new("TEST".to_string(), TradeSide::Long, 100.0, 5, 0.001);

        // Set price close to liquidation
        position.update_price(0.00082); // Just above liquidation price

        assert!(position.is_near_liquidation(0.01)); // 1% buffer
        assert!(!position.is_near_liquidation(0.001)); // 0.1% buffer
    }

    #[test]
    fn test_position_close() {
        let mut position = Position::new("TEST".to_string(), TradeSide::Long, 100.0, 5, 0.001);

        assert_eq!(position.status, PositionStatus::Open);
        assert!(position.closed_at.is_none());

        position.close();

        assert_eq!(position.status, PositionStatus::Closed);
        assert!(position.closed_at.is_some());
    }

    #[test]
    fn test_position_duration() {
        let position = Position::new("TEST".to_string(), TradeSide::Long, 100.0, 5, 0.001);

        let duration = position.duration_seconds();
        assert!(duration >= 0);
        assert!(duration < 5); // Should be very recent
    }

    #[test]
    fn test_portfolio_health_with_losses() {
        let mut portfolio = Portfolio::new(1000.0);
        let mut position = Position::new(
            "TEST".to_string(),
            TradeSide::Long,
            500.0, // 50% of portfolio
            10,    // 10x leverage
            0.001,
        );

        // Simulate 50% price drop (would cause 500% loss with 10x leverage)
        position.update_price(0.0005);
        portfolio.add_position(position);

        // Portfolio should be unhealthy due to massive losses
        assert!(!portfolio.is_healthy());
    }

    #[test]
    fn test_portfolio_update_with_multiple_positions() {
        let mut portfolio = Portfolio::new(1000.0);

        // Add profitable position
        let mut profitable_position =
            Position::new("PROFIT".to_string(), TradeSide::Long, 100.0, 5, 0.001);
        profitable_position.update_price(0.0012); // +20% price

        // Add losing position
        let mut losing_position =
            Position::new("LOSS".to_string(), TradeSide::Long, 100.0, 5, 0.001);
        losing_position.update_price(0.0008); // -20% price

        portfolio.add_position(profitable_position);
        portfolio.add_position(losing_position);

        // Net PnL should be 0 (100 profit - 100 loss)
        assert!((portfolio.equity - 1000.0).abs() < 0.1);
        assert_eq!(portfolio.open_positions.len(), 2);
    }

    #[test]
    fn test_trade_side_equality() {
        assert_eq!(TradeSide::Long, TradeSide::Long);
        assert_eq!(TradeSide::Short, TradeSide::Short);
        assert_ne!(TradeSide::Long, TradeSide::Short);
    }

    #[test]
    fn test_position_status_equality() {
        assert_eq!(PositionStatus::Open, PositionStatus::Open);
        assert_eq!(PositionStatus::Closed, PositionStatus::Closed);
        assert_ne!(PositionStatus::Open, PositionStatus::Closed);
    }
}
