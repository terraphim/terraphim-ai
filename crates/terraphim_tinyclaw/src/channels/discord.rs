//! Discord channel adapter using serenity.

use crate::bus::{MessageBus, OutboundMessage};
use crate::channel::Channel;
use crate::config::DiscordConfig;
use async_trait::async_trait;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Discord channel adapter.
pub struct DiscordChannel {
    config: DiscordConfig,
    running: Arc<AtomicBool>,
}

impl DiscordChannel {
    /// Create a new Discord channel.
    pub fn new(config: DiscordConfig) -> Self {
        Self {
            config,
            running: Arc::new(AtomicBool::new(false)),
        }
    }
}

#[async_trait]
impl Channel for DiscordChannel {
    fn name(&self) -> &str {
        "discord"
    }

    async fn start(&self, bus: Arc<MessageBus>) -> anyhow::Result<()> {
        log::info!("Discord channel starting");
        self.running.store(true, Ordering::SeqCst);

        #[cfg(feature = "discord")]
        {
            use crate::bus::InboundMessage;
            use serenity::async_trait as serenity_async_trait;
            use serenity::model::channel::Message as DiscordMessage;
            use serenity::model::gateway::Ready;
            use serenity::prelude::*;

            struct Handler {
                inbound_tx: tokio::sync::mpsc::Sender<InboundMessage>,
                allow_from: Vec<String>,
            }

            #[serenity_async_trait]
            impl EventHandler for Handler {
                async fn message(&self, _ctx: Context, msg: DiscordMessage) {
                    // Ignore bot messages
                    if msg.author.bot {
                        return;
                    }

                    let sender_id = msg.author.id.to_string();
                    let username = msg.author.name.clone();

                    // Check allowlist
                    if !self.allow_from.contains(&sender_id) && !self.allow_from.contains(&username)
                    {
                        log::warn!(
                            "Discord: rejected message from unauthorized user: {} ({})",
                            sender_id,
                            username
                        );
                        return;
                    }

                    let chat_id = msg.channel_id.to_string();
                    let inbound =
                        InboundMessage::new("discord", &sender_id, &chat_id, &msg.content);
                    if let Err(e) = self.inbound_tx.send(inbound).await {
                        log::error!("Failed to forward Discord message to bus: {}", e);
                    }
                }

                async fn ready(&self, _ctx: Context, ready: Ready) {
                    log::info!("Discord bot connected as {}", ready.user.name);
                }
            }

            let token = self.config.token.clone();
            let allow_from = self.config.allow_from.clone();
            let inbound_tx = bus.inbound_sender();
            let running = self.running.clone();

            log::info!("Discord bot starting");

            tokio::spawn(async move {
                let intents = GatewayIntents::GUILD_MESSAGES
                    | GatewayIntents::DIRECT_MESSAGES
                    | GatewayIntents::MESSAGE_CONTENT;

                let handler = Handler {
                    inbound_tx,
                    allow_from,
                };

                let mut client = match Client::builder(&token, intents)
                    .event_handler(handler)
                    .await
                {
                    Ok(c) => c,
                    Err(e) => {
                        log::error!("Failed to create Discord client: {}", e);
                        running.store(false, Ordering::SeqCst);
                        return;
                    }
                };

                if let Err(e) = client.start().await {
                    log::error!("Discord client error: {}", e);
                }
                running.store(false, Ordering::SeqCst);
            });

            Ok(())
        }

        #[cfg(not(feature = "discord"))]
        {
            let _ = bus;
            anyhow::bail!("Discord feature not enabled")
        }
    }

    async fn stop(&self) -> anyhow::Result<()> {
        log::info!("Discord channel stopping");
        self.running.store(false, Ordering::SeqCst);
        Ok(())
    }

    async fn send(&self, msg: OutboundMessage) -> anyhow::Result<()> {
        #[cfg(feature = "discord")]
        {
            use serenity::http::Http;

            let http = Http::new(&self.config.token);
            let channel_id = msg.chat_id.parse::<u64>()?;

            // Discord supports markdown natively, but chunk if too long
            let chunks = crate::format::chunk_message(&msg.content, 2000);

            for chunk in chunks {
                serenity::model::id::ChannelId::from(channel_id)
                    .say(&http, &chunk)
                    .await?;
            }
            Ok(())
        }

        #[cfg(not(feature = "discord"))]
        {
            let _ = msg;
            anyhow::bail!("Discord feature not enabled")
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
    fn test_discord_channel_name() {
        let config = DiscordConfig {
            token: "test".to_string(),
            allow_from: vec!["user1".to_string()],
        };
        let channel = DiscordChannel::new(config);
        assert_eq!(channel.name(), "discord");
    }

    #[test]
    fn test_discord_is_allowed() {
        let config = DiscordConfig {
            token: "test".to_string(),
            allow_from: vec!["user1".to_string(), "user2".to_string()],
        };
        let channel = DiscordChannel::new(config);
        assert!(channel.is_allowed("user1"));
        assert!(!channel.is_allowed("user3"));
    }
}
