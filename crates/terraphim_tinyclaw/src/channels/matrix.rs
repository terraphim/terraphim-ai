//! Matrix channel adapter for WhatsApp bridge integration.
//!
//! This channel connects to a Matrix homeserver and listens for messages
//! from the mautrix-whatsapp bridge, enabling WhatsApp integration.

use crate::bus::{InboundMessage, MessageBus, OutboundMessage};
use crate::channel::Channel;
use crate::config::MatrixConfig;
use async_trait::async_trait;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Matrix channel adapter for WhatsApp bridge.
pub struct MatrixChannel {
    config: MatrixConfig,
    running: Arc<AtomicBool>,
}

impl MatrixChannel {
    /// Create a new Matrix channel.
    pub fn new(config: MatrixConfig) -> Self {
        Self {
            config,
            running: Arc::new(AtomicBool::new(false)),
        }
    }
}

#[async_trait]
impl Channel for MatrixChannel {
    fn name(&self) -> &str {
        "matrix"
    }

    async fn start(&self, _bus: Arc<MessageBus>) -> anyhow::Result<()> {
        log::info!("Matrix channel starting");
        self.running.store(true, Ordering::SeqCst);

        #[cfg(feature = "matrix")]
        {
            use matrix_sdk::{
                config::SyncSettings,
                room::Room,
                ruma::events::room::message::{MessageType, OriginalSyncRoomMessageEvent},
                Client,
            };

            // Build client
            let client = Client::builder()
                .homeserver_url(&self.config.homeserver_url)
                .build()
                .await?;

            // Login
            client
                .matrix_auth()
                .login_username(&self.config.username, &self.config.password)
                .await?;

            log::info!("Matrix client logged in as {}", self.config.username);

            let bus = _bus.clone();
            let allow_from = self.config.allow_from.clone();
            let running = self.running.clone();

            // Register event handler for messages
            client.add_event_handler(
                move |ev: OriginalSyncRoomMessageEvent, room: Room| {
                    let bus = bus.clone();
                    let allow_from = allow_from.clone();
                    async move {
                        if !running.load(Ordering::SeqCst) {
                            return;
                        }

                        // Get sender
                        let sender = ev.sender.to_string();

                        // Check whitelist
                        if !allow_from.contains(&sender) {
                            log::warn!("Unauthorized Matrix message from: {}", sender);
                            return;
                        }

                        // Extract content based on message type
                        let content = match &ev.content.msgtype {
                            MessageType::Text(text_content) => text_content.body.clone(),
                            MessageType::Audio(audio) => {
                                // Voice message - we'll handle transcription later
                                format!("[Voice message: {}]", audio.body)
                            }
                            MessageType::Image(img) => {
                                format!("[Image: {}]", img.body)
                            }
                            MessageType::File(file) => {
                                format!("[File: {}]", file.body)
                            }
                            _ => {
                                log::debug!("Unsupported Matrix message type");
                                return;
                            }
                        };

                        // Get room ID
                        let room_id = room.room_id().to_string();

                        log::info!("Matrix message from {} in room {}", sender, room_id);

                        // Create inbound message
                        let inbound = InboundMessage::new("matrix", &sender, &room_id, &content);

                        // Send to bus
                        if let Err(e) = bus.inbound_sender().send(inbound).await {
                            log::error!("Failed to send message to bus: {}", e);
                        }
                    }
                },
            );

            // Start sync
            let settings = SyncSettings::default();
            tokio::spawn(async move {
                if let Err(e) = client.sync(settings).await {
                    log::error!("Matrix sync error: {}", e);
                }
            });

            Ok(())
        }

        #[cfg(not(feature = "matrix"))]
        {
            anyhow::bail!("Matrix feature not enabled")
        }
    }

    async fn stop(&self) -> anyhow::Result<()> {
        log::info!("Matrix channel stopping");
        self.running.store(false, Ordering::SeqCst);
        Ok(())
    }

    async fn send(&self, msg: OutboundMessage) -> anyhow::Result<()> {
        #[cfg(feature = "matrix")]
        {
            use matrix_sdk::{
                room::Room,
                ruma::RoomId,
                Client,
            };

            // Build client (reconnect for send)
            let client = Client::builder()
                .homeserver_url(&self.config.homeserver_url)
                .build()
                .await?;

            // Login
            client
                .matrix_auth()
                .login_username(&self.config.username, &self.config.password)
                .await?;

            // Parse room ID
            let room_id = msg
                .chat_id
                .parse::<Box<RoomId>>()
                .map_err(|e| anyhow::anyhow!("Invalid room ID: {}", e))?;

            // Get room
            if let Some(room) = client.get_room(&room_id) {
                // Send message
                room.send_plain_text(&msg.content).await?;
                log::debug!("Sent message to Matrix room {}", room_id);
            } else {
                anyhow::bail!("Room {} not found", room_id);
            }

            Ok(())
        }

        #[cfg(not(feature = "matrix"))]
        {
            let _ = msg;
            anyhow::bail!("Matrix feature not enabled")
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
    fn test_matrix_channel_name() {
        let config = MatrixConfig {
            homeserver_url: "https://example.com".to_string(),
            username: "test".to_string(),
            password: "pass".to_string(),
            allow_from: vec!["@user:example.com".to_string()],
        };
        let channel = MatrixChannel::new(config);
        assert_eq!(channel.name(), "matrix");
    }

    #[test]
    fn test_matrix_is_allowed() {
        let config = MatrixConfig {
            homeserver_url: "https://example.com".to_string(),
            username: "test".to_string(),
            password: "pass".to_string(),
            allow_from: vec![
                "@user1:example.com".to_string(),
                "@user2:example.com".to_string(),
            ],
        };
        let channel = MatrixChannel::new(config);
        assert!(channel.is_allowed("@user1:example.com"));
        assert!(channel.is_allowed("@user2:example.com"));
        assert!(!channel.is_allowed("@user3:example.com"));
    }
}
