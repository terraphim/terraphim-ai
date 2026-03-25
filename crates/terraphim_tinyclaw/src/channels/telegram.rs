//! Telegram channel adapter using teloxide.

use crate::bus::{MessageBus, OutboundMessage};
use crate::channel::Channel;
use crate::config::TelegramConfig;
use async_trait::async_trait;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Resolve a Telegram file_id to a download URL.
///
/// Calls the Telegram Bot API `getFile` and constructs the download URL.
/// Returns `None` on any failure (logs a warning).
#[cfg(feature = "telegram")]
async fn resolve_telegram_file_url(
    bot: &teloxide::Bot,
    token: &str,
    file_id: &teloxide::types::FileId,
) -> Option<String> {
    use teloxide::prelude::*;

    match bot.get_file(file_id.clone()).await {
        Ok(file) => {
            let url = format!("https://api.telegram.org/file/bot{}/{}", token, file.path);
            log::info!("Resolved Telegram file URL for file_id={}", file_id);
            Some(url)
        }
        Err(e) => {
            log::warn!("Failed to resolve Telegram file {}: {}", file_id, e);
            None
        }
    }
}

/// Extract media information from a Telegram message.
///
/// Checks for voice messages, audio files, and documents with audio MIME types.
/// Returns a content description and list of download URLs.
#[cfg(feature = "telegram")]
async fn extract_telegram_media(
    msg: &teloxide::types::Message,
    bot: &teloxide::Bot,
    token: &str,
) -> (Option<String>, Vec<String>) {
    let mut media_urls = Vec::new();

    // Voice message (ogg/opus)
    if let Some(voice) = msg.voice() {
        if let Some(url) = resolve_telegram_file_url(bot, token, &voice.file.id).await {
            media_urls.push(url);
        }
        return (Some("[Voice message]".to_string()), media_urls);
    }

    // Audio file (mp3, etc.)
    if let Some(audio) = msg.audio() {
        if let Some(url) = resolve_telegram_file_url(bot, token, &audio.file.id).await {
            media_urls.push(url);
        }
        let name = audio.file_name.as_deref().unwrap_or("audio");
        return (Some(format!("[Audio: {}]", name)), media_urls);
    }

    // Document with audio MIME type (mp3 sent as file, etc.)
    if let Some(doc) = msg.document() {
        let is_audio = doc
            .mime_type
            .as_ref()
            .map(|m| {
                let s = m.to_string();
                s.starts_with("audio/") || s == "video/ogg"
            })
            .unwrap_or(false);

        if is_audio {
            if let Some(url) = resolve_telegram_file_url(bot, token, &doc.file.id).await {
                media_urls.push(url);
            }
            let name = doc.file_name.as_deref().unwrap_or("document");
            return (Some(format!("[Audio file: {}]", name)), media_urls);
        }
    }

    (None, media_urls)
}

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
            let token = self.config.token.clone();
            let allow_from = self.config.allow_from.clone();
            let inbound_tx = bus.inbound_sender();
            let running = self.running.clone();

            log::info!("Telegram bot starting");

            tokio::spawn(async move {
                let handler = Update::filter_message().endpoint(
                    move |msg: teloxide::types::Message, bot: teloxide::Bot| {
                        let tx = inbound_tx.clone();
                        let allowed = allow_from.clone();
                        let token = token.clone();
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

                            // Check allowlist by user ID or username ("*" allows all)
                            if !crate::channel::is_sender_allowed(&allowed, &sender_id)
                                && !crate::channel::is_sender_allowed(&allowed, &username)
                            {
                                log::warn!(
                                    "Telegram: rejected message from unauthorized user: {} ({})",
                                    sender_id,
                                    username
                                );
                                return Ok(());
                            }

                            let chat_id = msg.chat.id.to_string();

                            // Try text message first
                            if let Some(text) = msg.text() {
                                let inbound =
                                    InboundMessage::new("telegram", &sender_id, &chat_id, text);
                                if let Err(e) = tx.send(inbound).await {
                                    log::error!("Failed to forward Telegram message to bus: {}", e);
                                }
                                return Ok(());
                            }

                            // Try voice/audio/document media
                            let (content, media_urls) =
                                extract_telegram_media(&msg, &bot, &token).await;

                            if let Some(content) = content {
                                let caption = msg.caption().unwrap_or("");
                                let full_content = if caption.is_empty() {
                                    content
                                } else {
                                    format!("{} {}", content, caption)
                                };

                                let inbound = InboundMessage::new(
                                    "telegram",
                                    &sender_id,
                                    &chat_id,
                                    &full_content,
                                )
                                .with_media(media_urls);

                                if let Err(e) = tx.send(inbound).await {
                                    log::error!(
                                        "Failed to forward Telegram media message to bus: {}",
                                        e
                                    );
                                }
                            } else {
                                log::debug!(
                                    "Telegram: unsupported message type from {}",
                                    sender_id
                                );
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
    fn test_telegram_file_url_format() {
        let token = "123456:ABC-DEF";
        let file_path = "voice/file_42.oga";
        let url = format!("https://api.telegram.org/file/bot{}/{}", token, file_path);
        assert_eq!(
            url,
            "https://api.telegram.org/file/bot123456:ABC-DEF/voice/file_42.oga"
        );
        assert!(url.starts_with("https://"));
        assert!(url.contains("/file/bot"));
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
