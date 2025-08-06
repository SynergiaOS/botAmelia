use async_trait::async_trait;
use anyhow::Result;

use crate::config::AlertsConfig;
use crate::monitoring::{SystemAlert, AlertLevel};

/// Moduł alertów z integracją Sentry
pub mod sender;
pub mod telegram;
pub mod discord;
pub mod email;

// pub use sender::AlertSender;

/// Trait dla wysyłania alertów
#[async_trait::async_trait]
pub trait AlertSenderTrait: Send + Sync {
    /// Wysyła alert
    async fn send_alert(&self, alert: &SystemAlert) -> Result<()>;

    /// Sprawdza czy kanał jest dostępny
    async fn is_available(&self) -> bool;

    /// Zwraca nazwę kanału
    fn channel_name(&self) -> &str;
}

/// Manager alertów
pub struct AlertManager {
    config: AlertsConfig,
    senders: Vec<Box<dyn AlertSenderTrait>>,
    rate_limiter: RateLimiter,
}

/// Rate limiter dla alertów
pub struct RateLimiter {
    last_sent: std::collections::HashMap<String, i64>,
    rate_limit_seconds: u64,
}

impl AlertManager {
    /// Tworzy nowy manager alertów
    pub fn new(config: AlertsConfig) -> Self {
        let rate_limiter = RateLimiter::new(config.rate_limit);
        
        Self {
            config,
            senders: Vec::new(),
            rate_limiter,
        }
    }
    
    /// Dodaje sender alertów
    pub fn add_sender(&mut self, sender: Box<dyn AlertSenderTrait>) {
        self.senders.push(sender);
    }
    
    /// Wysyła alert przez wszystkie dostępne kanały
    pub async fn send_alert(&mut self, alert: &SystemAlert) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }
        
        // Sprawdzenie rate limiting
        if !self.rate_limiter.should_send(&alert.id) {
            tracing::debug!("Alert {} rate limited", alert.id);
            return Ok(());
        }
        
        // Wysłanie do Sentry jeśli poziom jest odpowiednio wysoki
        if alert.should_send_to_sentry() {
            self.send_to_sentry(alert).await?;
        }
        
        // Wysłanie przez wszystkie kanały
        let mut errors = Vec::new();
        
        for sender in &self.senders {
            if let Err(e) = sender.send_alert(alert).await {
                tracing::error!("Failed to send alert via {}: {:?}", sender.channel_name(), e);
                errors.push(e);
            } else {
                tracing::info!("Alert {} sent via {}", alert.id, sender.channel_name());
            }
        }
        
        // Aktualizacja rate limiter
        self.rate_limiter.mark_sent(&alert.id);
        
        if !errors.is_empty() {
            return Err(anyhow::anyhow!("Some alert senders failed: {:?}", errors));
        }
        
        Ok(())
    }
    
    /// Wysyła alert do Sentry
    async fn send_to_sentry(&self, alert: &SystemAlert) -> Result<()> {
        // Konfiguracja kontekstu Sentry
        sentry::configure_scope(|scope| {
            scope.set_tag("component", &alert.component);
            scope.set_tag("alert_level", format!("{:?}", alert.level));
            scope.set_tag("alert_id", &alert.id);
            
            // Dodanie metryk jako extra data
            for (key, value) in &alert.metrics {
                scope.set_extra(key, (*value).into());
            }
        });
        
        // Wysłanie eventu do Sentry
        match alert.level {
            AlertLevel::Critical => {
                sentry::capture_message(&alert.title, sentry::Level::Fatal);
            }
            AlertLevel::High => {
                sentry::capture_message(&alert.title, sentry::Level::Error);
            }
            AlertLevel::Medium => {
                sentry::capture_message(&alert.title, sentry::Level::Warning);
            }
            AlertLevel::Low => {
                sentry::capture_message(&alert.title, sentry::Level::Info);
            }
        }
        
        tracing::info!("Alert {} sent to Sentry", alert.id);
        Ok(())
    }
    
    /// Tworzy alert o błędzie krytycznym
    pub async fn critical_error(&mut self, title: String, description: String, component: String) -> Result<()> {
        let alert = SystemAlert::new(
            AlertLevel::Critical,
            title,
            description,
            component,
        );
        
        self.send_alert(&alert).await
    }
    
    /// Tworzy alert o wysokim priorytecie
    pub async fn high_priority(&mut self, title: String, description: String, component: String) -> Result<()> {
        let alert = SystemAlert::new(
            AlertLevel::High,
            title,
            description,
            component,
        );
        
        self.send_alert(&alert).await
    }
    
    /// Tworzy alert o średnim priorytecie
    pub async fn medium_priority(&mut self, title: String, description: String, component: String) -> Result<()> {
        let alert = SystemAlert::new(
            AlertLevel::Medium,
            title,
            description,
            component,
        );
        
        self.send_alert(&alert).await
    }
    
    /// Sprawdza dostępność wszystkich kanałów
    pub async fn check_channels(&self) -> Vec<(String, bool)> {
        let mut results = Vec::new();
        
        for sender in &self.senders {
            let available = sender.is_available().await;
            results.push((sender.channel_name().to_string(), available));
        }
        
        results
    }
}

impl RateLimiter {
    /// Tworzy nowy rate limiter
    pub fn new(rate_limit_seconds: u64) -> Self {
        Self {
            last_sent: std::collections::HashMap::new(),
            rate_limit_seconds,
        }
    }
    
    /// Sprawdza czy alert może być wysłany
    pub fn should_send(&self, alert_id: &str) -> bool {
        let now = chrono::Utc::now().timestamp();
        
        if let Some(&last_sent) = self.last_sent.get(alert_id) {
            now - last_sent >= self.rate_limit_seconds as i64
        } else {
            true
        }
    }
    
    /// Oznacza alert jako wysłany
    pub fn mark_sent(&mut self, alert_id: &str) {
        let now = chrono::Utc::now().timestamp();
        self.last_sent.insert(alert_id.to_string(), now);
    }
    
    /// Czyści stare wpisy
    pub fn cleanup(&mut self) {
        let now = chrono::Utc::now().timestamp();
        let cutoff = now - (self.rate_limit_seconds as i64 * 2); // Usuń wpisy starsze niż 2x rate limit
        
        self.last_sent.retain(|_, &mut last_sent| last_sent > cutoff);
    }
}

/// Formatuje alert do wysłania
pub fn format_alert(alert: &SystemAlert) -> String {
    let emoji = match alert.level {
        AlertLevel::Critical => "🚨",
        AlertLevel::High => "⚠️",
        AlertLevel::Medium => "⚡",
        AlertLevel::Low => "ℹ️",
    };
    
    let mut message = format!(
        "{} **{}**\n\n{}\n\n**Component:** {}\n**Time:** {}",
        emoji,
        alert.title,
        alert.description,
        alert.component,
        chrono::DateTime::from_timestamp(alert.created_at, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
            .unwrap_or_else(|| "Unknown".to_string())
    );
    
    if !alert.metrics.is_empty() {
        message.push_str("\n\n**Metrics:**");
        for (key, value) in &alert.metrics {
            message.push_str(&format!("\n• {}: {:.2}", key, value));
        }
    }
    
    message
}

/// Formatuje alert dla Telegram (z ograniczeniem długości)
pub fn format_alert_telegram(alert: &SystemAlert) -> String {
    let base_message = format_alert(alert);
    
    // Telegram ma limit 4096 znaków
    if base_message.len() <= 4096 {
        base_message
    } else {
        let truncated = &base_message[..4090];
        format!("{}...", truncated)
    }
}

/// Formatuje alert dla Discord (z ograniczeniem długości)
pub fn format_alert_discord(alert: &SystemAlert) -> String {
    let base_message = format_alert(alert);
    
    // Discord ma limit 2000 znaków
    if base_message.len() <= 2000 {
        base_message
    } else {
        let truncated = &base_message[..1995];
        format!("{}...", truncated)
    }
}
