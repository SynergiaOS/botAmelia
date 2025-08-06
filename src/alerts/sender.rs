use anyhow::Result;
use std::future::Future;
use std::pin::Pin;

use super::AlertSenderTrait;
use crate::monitoring::SystemAlert;

/// Basic alert sender implementation
pub struct AlertSender {
    name: String,
    enabled: bool,
}

impl AlertSender {
    pub fn new(name: String) -> Self {
        Self {
            name,
            enabled: true,
        }
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

#[async_trait::async_trait]
impl AlertSenderTrait for AlertSender {
    async fn send_alert(&self, alert: &SystemAlert) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        // Basic implementation - just log the alert
        log::info!("Alert sent via {}: {:?}", self.name, alert);
        Ok(())
    }

    async fn is_available(&self) -> bool {
        self.enabled
    }

    fn channel_name(&self) -> &str {
        &self.name
    }
}
