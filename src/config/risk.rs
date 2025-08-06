use anyhow::Result;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Konfiguracja zarządzania ryzykiem
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskConfig {
    /// Maksymalne dzienne straty (w USD)
    pub max_daily_loss: Decimal,

    /// Maksymalne straty na pozycję (w procentach)
    pub max_position_loss_percent: Decimal,

    /// Próg aktywacji circuit breaker (w USD)
    pub circuit_breaker_threshold: Decimal,

    /// Liczba kolejnych niepowodzeń aktywujących circuit breaker
    pub max_consecutive_failures: u32,

    /// Maksymalna zmienność portfela (w procentach)
    pub max_portfolio_volatility: Decimal,

    /// Bufor przed likwidacją (w procentach)
    pub liquidation_buffer: Decimal,

    /// Minimalna odległość od ceny likwidacji (w procentach)
    pub min_liquidation_distance: Decimal,

    /// Maksymalny czas trwania pozycji (w godzinach)
    pub max_position_duration: u64,

    /// Konfiguracja stop-loss
    pub stop_loss: StopLossConfig,

    /// Konfiguracja take-profit
    pub take_profit: TakeProfitConfig,

    /// Czy włączyć automatyczne zarządzanie ryzykiem
    pub enable_auto_risk_management: bool,
}

/// Konfiguracja stop-loss
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopLossConfig {
    /// Czy włączyć stop-loss
    pub enabled: bool,

    /// Domyślny poziom stop-loss (w procentach)
    pub default_percent: Decimal,

    /// Czy używać trailing stop-loss
    pub use_trailing: bool,

    /// Odległość trailing stop (w procentach)
    pub trailing_distance: Decimal,
}

/// Konfiguracja take-profit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TakeProfitConfig {
    /// Czy włączyć take-profit
    pub enabled: bool,

    /// Domyślny poziom take-profit (w procentach)
    pub default_percent: Decimal,

    /// Czy używać częściowe realizowanie zysków
    pub use_partial: bool,

    /// Procent pozycji do zamknięcia przy pierwszym take-profit
    pub partial_percent: Decimal,

    /// Poziom pierwszego częściowego take-profit (w procentach)
    pub first_partial_level: Decimal,
}

impl Default for RiskConfig {
    fn default() -> Self {
        Self {
            max_daily_loss: Decimal::new(15, 0),            // $15
            max_position_loss_percent: Decimal::new(10, 2), // 10%
            circuit_breaker_threshold: Decimal::new(15, 0), // $15
            max_consecutive_failures: 5,
            max_portfolio_volatility: Decimal::new(15, 2), // 15%
            liquidation_buffer: Decimal::new(2, 2),        // 2%
            min_liquidation_distance: Decimal::new(5, 2),  // 5%
            max_position_duration: 24,                     // 24 godziny
            stop_loss: StopLossConfig::default(),
            take_profit: TakeProfitConfig::default(),
            enable_auto_risk_management: true,
        }
    }
}

impl Default for StopLossConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_percent: Decimal::new(10, 2), // 10%
            use_trailing: false,
            trailing_distance: Decimal::new(5, 2), // 5%
        }
    }
}

impl Default for TakeProfitConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_percent: Decimal::new(20, 2), // 20%
            use_partial: true,
            partial_percent: Decimal::new(50, 2),     // 50%
            first_partial_level: Decimal::new(15, 2), // 15%
        }
    }
}

impl RiskConfig {
    /// Waliduje konfigurację zarządzania ryzykiem
    pub fn validate(&self) -> Result<()> {
        // Walidacja strat
        if self.max_daily_loss <= Decimal::ZERO {
            anyhow::bail!("max_daily_loss must be greater than 0");
        }

        if self.max_position_loss_percent <= Decimal::ZERO {
            anyhow::bail!("max_position_loss_percent must be greater than 0");
        }

        if self.max_position_loss_percent >= Decimal::ONE {
            anyhow::bail!("max_position_loss_percent must be less than 100%");
        }

        // Walidacja circuit breaker
        if self.circuit_breaker_threshold <= Decimal::ZERO {
            anyhow::bail!("circuit_breaker_threshold must be greater than 0");
        }

        if self.max_consecutive_failures == 0 {
            anyhow::bail!("max_consecutive_failures must be greater than 0");
        }

        // Walidacja zmienności
        if self.max_portfolio_volatility <= Decimal::ZERO {
            anyhow::bail!("max_portfolio_volatility must be greater than 0");
        }

        if self.max_portfolio_volatility >= Decimal::ONE {
            anyhow::bail!("max_portfolio_volatility must be less than 100%");
        }

        // Walidacja buforów
        if self.liquidation_buffer <= Decimal::ZERO {
            anyhow::bail!("liquidation_buffer must be greater than 0");
        }

        if self.liquidation_buffer >= Decimal::new(10, 2) {
            anyhow::bail!("liquidation_buffer should not exceed 10%");
        }

        if self.min_liquidation_distance <= Decimal::ZERO {
            anyhow::bail!("min_liquidation_distance must be greater than 0");
        }

        // Walidacja czasu
        if self.max_position_duration == 0 {
            anyhow::bail!("max_position_duration must be greater than 0");
        }

        // Walidacja stop-loss
        self.stop_loss.validate()?;

        // Walidacja take-profit
        self.take_profit.validate()?;

        Ok(())
    }
}

impl StopLossConfig {
    /// Waliduje konfigurację stop-loss
    pub fn validate(&self) -> Result<()> {
        if self.enabled {
            if self.default_percent <= Decimal::ZERO {
                anyhow::bail!("stop_loss default_percent must be greater than 0");
            }

            if self.default_percent >= Decimal::ONE {
                anyhow::bail!("stop_loss default_percent must be less than 100%");
            }

            if self.use_trailing {
                if self.trailing_distance <= Decimal::ZERO {
                    anyhow::bail!("stop_loss trailing_distance must be greater than 0");
                }

                if self.trailing_distance >= self.default_percent {
                    anyhow::bail!(
                        "stop_loss trailing_distance should be less than default_percent"
                    );
                }
            }
        }

        Ok(())
    }
}

impl TakeProfitConfig {
    /// Waliduje konfigurację take-profit
    pub fn validate(&self) -> Result<()> {
        if self.enabled {
            if self.default_percent <= Decimal::ZERO {
                anyhow::bail!("take_profit default_percent must be greater than 0");
            }

            if self.use_partial {
                if self.partial_percent <= Decimal::ZERO {
                    anyhow::bail!("take_profit partial_percent must be greater than 0");
                }

                if self.partial_percent >= Decimal::ONE {
                    anyhow::bail!("take_profit partial_percent must be less than 100%");
                }

                if self.first_partial_level <= Decimal::ZERO {
                    anyhow::bail!("take_profit first_partial_level must be greater than 0");
                }

                if self.first_partial_level >= self.default_percent {
                    anyhow::bail!(
                        "take_profit first_partial_level should be less than default_percent"
                    );
                }
            }
        }

        Ok(())
    }
}
