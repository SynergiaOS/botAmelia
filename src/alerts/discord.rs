use anyhow::Result;
use std::pin::Pin;
use std::future::Future;

use crate::monitoring::SystemAlert;
use super::AlertSenderTrait;

/// Discord alert sender
pub struct DiscordSender {
    webhook_url: Option<String>,
    enabled: bool,
}

impl DiscordSender {
    pub fn new(webhook_url: Option<String>) -> Self {
        let enabled = webhook_url.is_some();
        
        Self {
            webhook_url,
            enabled,
        }
    }
}

#[async_trait::async_trait]
impl AlertSenderTrait for DiscordSender {
    async fn send_alert(&self, alert: &SystemAlert) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        // Mock implementation for now
        log::info!("Discord alert sent: {:?}", alert);
        Ok(())
    }

    async fn is_available(&self) -> bool {
        self.enabled
    }

    fn channel_name(&self) -> &str {
        "discord"
    }
}
