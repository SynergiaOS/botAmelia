use anyhow::Result;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Konfiguracja systemu tradingu
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingConfig {
    /// Początkowy balans portfela (w USD)
    pub initial_balance: Decimal,

    /// Maksymalna dźwignia
    pub max_leverage: u8,

    /// Minimalna dźwignia
    pub min_leverage: u8,

    /// Maksymalna liczba jednoczesnych pozycji
    pub max_concurrent_positions: u8,

    /// Maksymalny rozmiar pozycji jako procent portfela
    pub max_position_size_percent: Decimal,

    /// Minimalny rozmiar pozycji (w USD)
    pub min_position_size: Decimal,

    /// Timeout dla wykonania transakcji (w sekundach)
    pub execution_timeout: u64,

    /// Interwał sprawdzania pozycji (w sekundach)
    pub position_check_interval: u64,

    /// Czy włączyć tryb testowy (paper trading)
    pub paper_trading: bool,

    /// Konfiguracja dźwigni dla różnych poziomów pewności
    pub leverage_config: LeverageConfig,
}

/// Konfiguracja dźwigni dla różnych poziomów pewności sygnałów
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeverageConfig {
    /// Dźwignia dla sygnałów o niskiej pewności
    pub low_confidence: u8,

    /// Dźwignia dla sygnałów o średniej pewności
    pub medium_confidence: u8,

    /// Dźwignia dla sygnałów o wysokiej pewności
    pub high_confidence: u8,

    /// Dźwignia dla sygnałów o ekstremalnej pewności
    pub extreme_confidence: u8,
}

impl Default for TradingConfig {
    fn default() -> Self {
        Self {
            initial_balance: Decimal::new(50, 0), // $50
            max_leverage: 50,
            min_leverage: 2,
            max_concurrent_positions: 3,
            max_position_size_percent: Decimal::new(33, 2), // 33%
            min_position_size: Decimal::new(5, 0),          // $5
            execution_timeout: 30,
            position_check_interval: 5,
            paper_trading: true, // Domyślnie tryb testowy
            leverage_config: LeverageConfig::default(),
        }
    }
}

impl Default for LeverageConfig {
    fn default() -> Self {
        Self {
            low_confidence: 5,
            medium_confidence: 10,
            high_confidence: 20,
            extreme_confidence: 30,
        }
    }
}

impl TradingConfig {
    /// Waliduje konfigurację tradingu
    pub fn validate(&self) -> Result<()> {
        // Walidacja dźwigni
        if self.max_leverage < self.min_leverage {
            anyhow::bail!("max_leverage must be greater than or equal to min_leverage");
        }

        if self.min_leverage < 1 {
            anyhow::bail!("min_leverage must be at least 1");
        }

        if self.max_leverage > 100 {
            anyhow::bail!("max_leverage cannot exceed 100");
        }

        // Walidacja pozycji
        if self.max_concurrent_positions == 0 {
            anyhow::bail!("max_concurrent_positions must be greater than 0");
        }

        if self.max_concurrent_positions > 10 {
            anyhow::bail!("max_concurrent_positions cannot exceed 10");
        }

        // Walidacja rozmiarów pozycji
        if self.max_position_size_percent <= Decimal::ZERO {
            anyhow::bail!("max_position_size_percent must be greater than 0");
        }

        if self.max_position_size_percent > Decimal::ONE {
            anyhow::bail!("max_position_size_percent cannot exceed 1.0 (100%)");
        }

        if self.min_position_size <= Decimal::ZERO {
            anyhow::bail!("min_position_size must be greater than 0");
        }

        // Walidacja balansu
        if self.initial_balance <= Decimal::ZERO {
            anyhow::bail!("initial_balance must be greater than 0");
        }

        // Walidacja timeoutów
        if self.execution_timeout == 0 {
            anyhow::bail!("execution_timeout must be greater than 0");
        }

        if self.position_check_interval == 0 {
            anyhow::bail!("position_check_interval must be greater than 0");
        }

        // Walidacja konfiguracji dźwigni
        self.leverage_config.validate(self.max_leverage)?;

        Ok(())
    }

    /// Zwraca dźwignię dla danego poziomu pewności
    pub fn get_leverage_for_confidence(&self, confidence: &str) -> u8 {
        match confidence.to_lowercase().as_str() {
            "low" => self.leverage_config.low_confidence,
            "medium" => self.leverage_config.medium_confidence,
            "high" => self.leverage_config.high_confidence,
            "extreme" => self.leverage_config.extreme_confidence,
            _ => self.leverage_config.low_confidence, // Domyślnie niska pewność
        }
    }
}

impl LeverageConfig {
    /// Waliduje konfigurację dźwigni
    pub fn validate(&self, max_leverage: u8) -> Result<()> {
        let leverages = [
            ("low_confidence", self.low_confidence),
            ("medium_confidence", self.medium_confidence),
            ("high_confidence", self.high_confidence),
            ("extreme_confidence", self.extreme_confidence),
        ];

        for (name, leverage) in leverages {
            if leverage == 0 {
                anyhow::bail!("{} leverage must be greater than 0", name);
            }

            if leverage > max_leverage {
                anyhow::bail!(
                    "{} leverage cannot exceed max_leverage ({})",
                    name,
                    max_leverage
                );
            }
        }

        // Sprawdzenie czy dźwignie są w rosnącej kolejności
        if self.low_confidence > self.medium_confidence {
            anyhow::bail!("low_confidence leverage should not exceed medium_confidence");
        }

        if self.medium_confidence > self.high_confidence {
            anyhow::bail!("medium_confidence leverage should not exceed high_confidence");
        }

        if self.high_confidence > self.extreme_confidence {
            anyhow::bail!("high_confidence leverage should not exceed extreme_confidence");
        }

        Ok(())
    }
}
