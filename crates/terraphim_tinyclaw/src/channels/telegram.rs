//! Telegram channel adapter using teloxide.

use crate::bus::{MessageBus, OutboundMessage};
use crate::channel::Channel;
use crate::config::TelegramConfig;
use async_trait::async_trait;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Telegram channel adapter.
pub struct TelegramChannel {
    config: TelegramConfig,
    running: Arc<AtomicBool>,
}

impl TelegramChannel {
    /// Create a new Telegram channel.
    pub fn new(config: TelegramConfig) -> Self {
        Self {
            config,
            running: Arc::new(AtomicBool::new(false)),
        }
    }
}

#[async_trait]
impl Channel for TelegramChannel {
    fn name(&self) -> &str {
        "telegram"
    }

    async fn start(&self, _bus: Arc<MessageBus>) -> anyhow::Result<()> {
        log::info!("Telegram channel starting");
        self.running.store(true, Ordering::SeqCst);

        #[cfg(feature = "telegram")]
        {
            // Simplified implementation - just log for now
            log::info!(
                "Telegram bot would start here with token: {}...",
                &self.config.token[..self.config.token.len().min(10)]
            );
            Ok(())
        }

        #[cfg(not(feature = "telegram"))]
        {
            anyhow::bail!("Telegram feature not enabled")
        }
    }

    async fn stop(&self) -> anyhow::Result<()> {
        log::info!("Telegram channel stopping");
        self.running.store(false, Ordering::SeqCst);
        Ok(())
    }

    async fn send(&self, msg: OutboundMessage) -> anyhow::Result<()> {
        #[cfg(feature = "telegram")]
        {
            use teloxide::prelude::*;
            use teloxide::types::{ParseMode, Recipient};

            let bot = teloxide::Bot::new(&self.config.token);
            let chat_id = msg.chat_id.parse::<i64>()?;

            let formatted = crate::format::markdown_to_telegram_html(&msg.content);
            let chunks = crate::format::chunk_message(&formatted, 4096);

            for chunk in chunks {
                bot.send_message(Recipient::Id(ChatId(chat_id)), chunk)
                    .parse_mode(ParseMode::Html)
                    .await?;
            }
            Ok(())
        }

        #[cfg(not(feature = "telegram"))]
        {
            let _ = msg;
            anyhow::bail!("Telegram feature not enabled")
        }
    }

    fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    fn is_allowed(&self, sender_id: &str) -> bool {
        self.config.is_allowed(sender_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telegram_channel_name() {
        let config = TelegramConfig {
            token: "test".to_string(),
            allow_from: vec!["user1".to_string()],
        };
        let channel = TelegramChannel::new(config);
        assert_eq!(channel.name(), "telegram");
    }

    #[test]
    fn test_telegram_is_allowed() {
        let config = TelegramConfig {
            token: "test".to_string(),
            allow_from: vec!["user1".to_string(), "user2".to_string()],
        };
        let channel = TelegramChannel::new(config);
        assert!(channel.is_allowed("user1"));
        assert!(!channel.is_allowed("user3"));
    }
}
