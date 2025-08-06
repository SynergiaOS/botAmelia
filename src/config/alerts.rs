use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Konfiguracja systemu alertów
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertsConfig {
    /// Czy włączyć system alertów
    pub enabled: bool,

    /// Konfiguracja Telegram
    pub telegram: TelegramConfig,

    /// Konfiguracja Discord
    pub discord: DiscordConfig,

    /// Konfiguracja email
    pub email: EmailConfig,

    /// Konfiguracja poziomów alertów
    pub levels: AlertLevelsConfig,

    /// Maksymalna częstotliwość alertów (w sekundach)
    pub rate_limit: u64,

    /// Czy grupować podobne alerty
    pub group_similar: bool,

    /// Czas grupowania alertów (w sekundach)
    pub grouping_window: u64,
}

/// Konfiguracja Telegram
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramConfig {
    /// Czy włączyć alerty Telegram
    pub enabled: bool,

    /// Token bota Telegram
    pub bot_token: Option<String>,

    /// ID czatu dla alertów
    pub chat_id: Option<String>,

    /// Czy wysyłać alerty krytyczne
    pub send_critical: bool,

    /// Czy wysyłać alerty wysokie
    pub send_high: bool,

    /// Czy wysyłać alerty średnie
    pub send_medium: bool,

    /// Czy wysyłać alerty niskie
    pub send_low: bool,
}

/// Konfiguracja Discord
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordConfig {
    /// Czy włączyć alerty Discord
    pub enabled: bool,

    /// Token bota Discord
    pub bot_token: Option<String>,

    /// ID kanału dla alertów
    pub channel_id: Option<String>,

    /// Czy wysyłać alerty krytyczne
    pub send_critical: bool,

    /// Czy wysyłać alerty wysokie
    pub send_high: bool,

    /// Czy wysyłać alerty średnie
    pub send_medium: bool,

    /// Czy wysyłać alerty niskie
    pub send_low: bool,
}

/// Konfiguracja email
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailConfig {
    /// Czy włączyć alerty email
    pub enabled: bool,

    /// Serwer SMTP
    pub smtp_server: Option<String>,

    /// Port SMTP
    pub smtp_port: u16,

    /// Nazwa użytkownika SMTP
    pub smtp_username: Option<String>,

    /// Hasło SMTP
    pub smtp_password: Option<String>,

    /// Adres nadawcy
    pub from_address: Option<String>,

    /// Adresy odbiorców
    pub to_addresses: Vec<String>,

    /// Czy używać TLS
    pub use_tls: bool,

    /// Czy wysyłać alerty krytyczne
    pub send_critical: bool,

    /// Czy wysyłać alerty wysokie
    pub send_high: bool,
}

/// Konfiguracja poziomów alertów
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertLevelsConfig {
    /// Próg aktywacji alertu krytycznego dla strat (w USD)
    pub critical_loss_threshold: rust_decimal::Decimal,

    /// Próg aktywacji alertu wysokiego dla strat (w USD)
    pub high_loss_threshold: rust_decimal::Decimal,

    /// Próg aktywacji alertu średniego dla strat (w USD)
    pub medium_loss_threshold: rust_decimal::Decimal,

    /// Próg aktywacji alertu dla użycia pamięci (w MB)
    pub memory_threshold: u64,

    /// Próg aktywacji alertu dla czasu odpowiedzi (w ms)
    pub response_time_threshold: u64,

    /// Liczba kolejnych błędów aktywujących alert
    pub consecutive_errors_threshold: u32,
}

impl Default for AlertsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            telegram: TelegramConfig::default(),
            discord: DiscordConfig::default(),
            email: EmailConfig::default(),
            levels: AlertLevelsConfig::default(),
            rate_limit: 60, // 1 minuta
            group_similar: true,
            grouping_window: 300, // 5 minut
        }
    }
}

impl Default for TelegramConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            bot_token: None,
            chat_id: None,
            send_critical: true,
            send_high: true,
            send_medium: true,
            send_low: false,
        }
    }
}

impl Default for DiscordConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            bot_token: None,
            channel_id: None,
            send_critical: true,
            send_high: true,
            send_medium: true,
            send_low: false,
        }
    }
}

impl Default for EmailConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            smtp_server: None,
            smtp_port: 587,
            smtp_username: None,
            smtp_password: None,
            from_address: None,
            to_addresses: Vec::new(),
            use_tls: true,
            send_critical: true,
            send_high: true,
        }
    }
}

impl Default for AlertLevelsConfig {
    fn default() -> Self {
        Self {
            critical_loss_threshold: rust_decimal::Decimal::new(15, 0), // $15
            high_loss_threshold: rust_decimal::Decimal::new(10, 0),     // $10
            medium_loss_threshold: rust_decimal::Decimal::new(5, 0),    // $5
            memory_threshold: 200,                                      // 200 MB
            response_time_threshold: 1000,                              // 1 sekunda
            consecutive_errors_threshold: 5,
        }
    }
}

impl AlertsConfig {
    /// Waliduje konfigurację alertów
    pub fn validate(&self) -> Result<()> {
        if self.enabled {
            // Sprawdzenie czy przynajmniej jeden kanał alertów jest włączony
            if !self.telegram.enabled && !self.discord.enabled && !self.email.enabled {
                anyhow::bail!("At least one alert channel must be enabled when alerts are enabled");
            }

            // Walidacja rate limiting
            if self.rate_limit == 0 {
                anyhow::bail!("rate_limit must be greater than 0");
            }

            if self.group_similar && self.grouping_window == 0 {
                anyhow::bail!(
                    "grouping_window must be greater than 0 when group_similar is enabled"
                );
            }
        }

        // Walidacja konfiguracji Telegram
        self.telegram.validate()?;

        // Walidacja konfiguracji Discord
        self.discord.validate()?;

        // Walidacja konfiguracji email
        self.email.validate()?;

        // Walidacja poziomów alertów
        self.levels.validate()?;

        Ok(())
    }
}

impl TelegramConfig {
    /// Waliduje konfigurację Telegram
    pub fn validate(&self) -> Result<()> {
        if self.enabled {
            if self.bot_token.is_none() {
                anyhow::bail!("telegram bot_token is required when Telegram alerts are enabled");
            }

            if self.chat_id.is_none() {
                anyhow::bail!("telegram chat_id is required when Telegram alerts are enabled");
            }
        }

        Ok(())
    }
}

impl DiscordConfig {
    /// Waliduje konfigurację Discord
    pub fn validate(&self) -> Result<()> {
        if self.enabled {
            if self.bot_token.is_none() {
                anyhow::bail!("discord bot_token is required when Discord alerts are enabled");
            }

            if self.channel_id.is_none() {
                anyhow::bail!("discord channel_id is required when Discord alerts are enabled");
            }
        }

        Ok(())
    }
}

impl EmailConfig {
    /// Waliduje konfigurację email
    pub fn validate(&self) -> Result<()> {
        if self.enabled {
            if self.smtp_server.is_none() {
                anyhow::bail!("email smtp_server is required when email alerts are enabled");
            }

            if self.smtp_port == 0 {
                anyhow::bail!("email smtp_port must be greater than 0");
            }

            if self.from_address.is_none() {
                anyhow::bail!("email from_address is required when email alerts are enabled");
            }

            if self.to_addresses.is_empty() {
                anyhow::bail!("email to_addresses cannot be empty when email alerts are enabled");
            }
        }

        Ok(())
    }
}

impl AlertLevelsConfig {
    /// Waliduje konfigurację poziomów alertów
    pub fn validate(&self) -> Result<()> {
        use rust_decimal::Decimal;

        if self.critical_loss_threshold <= Decimal::ZERO {
            anyhow::bail!("critical_loss_threshold must be greater than 0");
        }

        if self.high_loss_threshold <= Decimal::ZERO {
            anyhow::bail!("high_loss_threshold must be greater than 0");
        }

        if self.medium_loss_threshold <= Decimal::ZERO {
            anyhow::bail!("medium_loss_threshold must be greater than 0");
        }

        if self.memory_threshold == 0 {
            anyhow::bail!("memory_threshold must be greater than 0");
        }

        if self.response_time_threshold == 0 {
            anyhow::bail!("response_time_threshold must be greater than 0");
        }

        if self.consecutive_errors_threshold == 0 {
            anyhow::bail!("consecutive_errors_threshold must be greater than 0");
        }

        Ok(())
    }
}
