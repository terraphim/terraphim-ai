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

    async fn start(&self, bus: Arc<MessageBus>) -> anyhow::Result<()> {
        log::info!("Telegram channel starting");
        self.running.store(true, Ordering::SeqCst);

        #[cfg(feature = "telegram")]
        {
            use crate::bus::InboundMessage;
            use teloxide::prelude::*;

            let bot = teloxide::Bot::new(&self.config.token);
            let allow_from = self.config.allow_from.clone();
            let inbound_tx = bus.inbound_sender();
            let running = self.running.clone();

            log::info!(
                "Telegram bot starting (token: {}...)",
                &self.config.token[..self.config.token.len().min(10)]
            );

            tokio::spawn(async move {
                let handler = Update::filter_message().endpoint(
                    move |msg: teloxide::types::Message, _bot: teloxide::Bot| {
                        let tx = inbound_tx.clone();
                        let allowed = allow_from.clone();
                        async move {
                            let sender_id = msg
                                .from
                                .as_ref()
                                .map(|u| u.id.to_string())
                                .unwrap_or_default();
                            let username = msg
                                .from
                                .as_ref()
                                .and_then(|u| u.username.clone())
                                .unwrap_or_default();

                            // Check allowlist by user ID or username
                            if !allowed.contains(&sender_id) && !allowed.contains(&username) {
                                log::warn!(
                                    "Telegram: rejected message from unauthorized user: {} ({})",
                                    sender_id,
                                    username
                                );
                                return Ok(());
                            }

                            if let Some(text) = msg.text() {
                                let chat_id = msg.chat.id.to_string();
                                let inbound =
                                    InboundMessage::new("telegram", &sender_id, &chat_id, text);
                                if let Err(e) = tx.send(inbound).await {
                                    log::error!("Failed to forward Telegram message to bus: {}", e);
                                }
                            }
                            Ok::<(), anyhow::Error>(())
                        }
                    },
                );

                let mut dispatcher = Dispatcher::builder(bot, handler)
                    .default_handler(|_| async {})
                    .build();

                dispatcher.dispatch().await;
                running.store(false, Ordering::SeqCst);
            });

            Ok(())
        }

        #[cfg(not(feature = "telegram"))]
        {
            let _ = bus;
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
