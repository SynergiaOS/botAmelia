use anyhow::Result;
use std::future::Future;
use std::pin::Pin;

use super::AlertSenderTrait;
use crate::monitoring::SystemAlert;

/// Email alert sender
pub struct EmailSender {
    smtp_server: Option<String>,
    from_email: Option<String>,
    to_email: Option<String>,
    enabled: bool,
}

impl EmailSender {
    pub fn new(
        smtp_server: Option<String>,
        from_email: Option<String>,
        to_email: Option<String>,
    ) -> Self {
        let enabled = smtp_server.is_some() && from_email.is_some() && to_email.is_some();

        Self {
            smtp_server,
            from_email,
            to_email,
            enabled,
        }
    }
}

#[async_trait::async_trait]
impl AlertSenderTrait for EmailSender {
    async fn send_alert(&self, alert: &SystemAlert) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        // Mock implementation for now
        log::info!("Email alert sent: {:?}", alert);
        Ok(())
    }

    async fn is_available(&self) -> bool {
        self.enabled
    }

    fn channel_name(&self) -> &str {
        "email"
    }
}
