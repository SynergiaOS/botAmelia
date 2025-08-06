use anyhow::Result;
use std::pin::Pin;
use std::future::Future;

use crate::monitoring::SystemAlert;
use super::AlertSenderTrait;

/// Telegram alert sender
pub struct TelegramSender {
    bot_token: Option<String>,
    chat_id: Option<String>,
    enabled: bool,
}

impl TelegramSender {
    pub fn new(bot_token: Option<String>, chat_id: Option<String>) -> Self {
        let enabled = bot_token.is_some() && chat_id.is_some();
        
        Self {
            bot_token,
            chat_id,
            enabled,
        }
    }
}

#[async_trait::async_trait]
impl AlertSenderTrait for TelegramSender {
    async fn send_alert(&self, alert: &SystemAlert) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        // Mock implementation for now
        log::info!("Telegram alert sent: {:?}", alert);
        Ok(())
    }

    async fn is_available(&self) -> bool {
        self.enabled
    }

    fn channel_name(&self) -> &str {
        "telegram"
    }
}
