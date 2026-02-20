//! Integration tests for gateway mode outbound dispatch
//!
//! Tests GAP-003: Verifies that outbound messages are properly dispatched
//! to channels in gateway mode.

use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

use terraphim_tinyclaw::bus::{MessageBus, OutboundMessage};
use terraphim_tinyclaw::channel::{Channel, ChannelManager};

/// A mock channel for testing that records all sent messages
struct MockChannel {
    name: String,
    sent_messages: Arc<tokio::sync::Mutex<Vec<OutboundMessage>>>,
    running: Arc<std::sync::atomic::AtomicBool>,
    allowed_senders: Vec<String>,
}

impl MockChannel {
    fn new(name: &str) -> (Self, Arc<tokio::sync::Mutex<Vec<OutboundMessage>>>) {
        let sent = Arc::new(tokio::sync::Mutex::new(Vec::new()));
        (
            Self {
                name: name.to_string(),
                sent_messages: sent.clone(),
                running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
                allowed_senders: vec!["*".to_string()], // Allow all by default
            },
            sent,
        )
    }

    fn get_sent_messages(&self,
    ) -> Arc<tokio::sync::Mutex<Vec<OutboundMessage>>> {
        self.sent_messages.clone()
    }
}

#[async_trait::async_trait]
impl Channel for MockChannel {
    fn name(&self) -> &str {
        &self.name
    }

    async fn start(&self,
        _bus: Arc<MessageBus>,
    ) -> anyhow::Result<()> {
        self.running.store(true, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }

    async fn stop(&self) -> anyhow::Result<()> {
        self.running.store(false, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }

    async fn send(&self,
        msg: OutboundMessage,
    ) -> anyhow::Result<()> {
        self.sent_messages.lock().await.push(msg);
        Ok(())
    }

    fn is_running(&self) -> bool {
        self.running.load(std::sync::atomic::Ordering::SeqCst)
    }

    fn is_allowed(&self,
        _sender_id: &str,
    ) -> bool {
        self.allowed_senders.contains(&"*".to_string())
            || self.allowed_senders.contains(&_sender_id.to_string())
    }
}

/// Test that outbound messages are dispatched to the correct channel
#[tokio::test]
async fn test_outbound_message_dispatch() {
    // Create message bus
    let bus = Arc::new(MessageBus::new());

    // Create mock channels
    let (mock_channel, received_messages) = MockChannel::new("test-channel");

    // Create channel manager and register mock channel
    let mut channel_manager = ChannelManager::new();
    channel_manager.register(Box::new(mock_channel));

    // Spawn the dispatch loop (simulating gateway mode)
    let bus_clone = bus.clone();
    let _dispatch_handle = tokio::spawn(async move {
        let mut outbound_rx = bus_clone.outbound_rx.lock().await;
        while let Some(msg) = outbound_rx.recv().await {
            log::debug!("Dispatching outbound to channel: {}", msg.channel);
            if let Err(e) = channel_manager.send(msg).await {
                log::error!("Failed to dispatch outbound message: {}", e);
            }
        }
    });

    // Send an outbound message
    let test_message = OutboundMessage::new("test-channel", "chat-1", "Hello, World!");
    bus.outbound_sender().send(test_message.clone()).await.unwrap();

    // Wait for message to be dispatched
    let messages = timeout(Duration::from_secs(5), async {
        loop {
            let msgs = received_messages.lock().await;
            if !msgs.is_empty() {
                return msgs.clone();
            }
            drop(msgs);
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("Message should be dispatched within 5 seconds");

    // Verify message was received
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].channel, "test-channel");
    assert_eq!(messages[0].chat_id, "chat-1");
    assert_eq!(messages[0].content, "Hello, World!");
}

/// Test that messages are routed to the correct channel based on channel field
#[tokio::test]
async fn test_message_routing_to_multiple_channels() {
    // Create message bus
    let bus = Arc::new(MessageBus::new());

    // Create mock channels
    let (telegram_mock, telegram_messages) = MockChannel::new("telegram");
    let (discord_mock, discord_messages) = MockChannel::new("discord");

    // Create channel manager
    let mut channel_manager = ChannelManager::new();
    channel_manager.register(Box::new(telegram_mock));
    channel_manager.register(Box::new(discord_mock));

    // Spawn the dispatch loop
    let bus_clone = bus.clone();
    let _dispatch_handle = tokio::spawn(async move {
        let mut outbound_rx = bus_clone.outbound_rx.lock().await;
        while let Some(msg) = outbound_rx.recv().await {
            if let Err(e) = channel_manager.send(msg).await {
                log::error!("Failed to dispatch outbound message: {}", e);
            }
        }
    });

    // Send messages to different channels
    let telegram_msg = OutboundMessage::new("telegram", "tg-chat-1", "Hello Telegram!");
    let discord_msg = OutboundMessage::new("discord", "dc-chat-1", "Hello Discord!");

    bus.outbound_sender().send(telegram_msg).await.unwrap();
    bus.outbound_sender().send(discord_msg).await.unwrap();

    // Wait for both messages to be dispatched
    timeout(Duration::from_secs(5), async {
        loop {
            let tg = telegram_messages.lock().await;
            let dc = discord_messages.lock().await;
            if tg.len() == 1 && dc.len() == 1 {
                return (tg.clone(), dc.clone());
            }
            drop(tg);
            drop(dc);
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("Both messages should be dispatched within 5 seconds");

    // Verify routing
    let tg_msgs = telegram_messages.lock().await;
    let dc_msgs = discord_messages.lock().await;

    assert_eq!(tg_msgs.len(), 1);
    assert_eq!(tg_msgs[0].channel, "telegram");
    assert_eq!(tg_msgs[0].content, "Hello Telegram!");

    assert_eq!(dc_msgs.len(), 1);
    assert_eq!(dc_msgs[0].channel, "discord");
    assert_eq!(dc_msgs[0].content, "Hello Discord!");
}

/// Test that unknown channels are handled gracefully
#[tokio::test]
async fn test_unknown_channel_graceful_handling() {
    // Create message bus
    let bus = Arc::new(MessageBus::new());

    // Create channel manager with NO channels registered
    let channel_manager = ChannelManager::new();

    // Spawn the dispatch loop
    let bus_clone = bus.clone();
    let _dispatch_handle = tokio::spawn(async move {
        let mut outbound_rx = bus_clone.outbound_rx.lock().await;
        while let Some(msg) = outbound_rx.recv().await {
            // This should fail gracefully since no channel is registered
            if let Err(e) = channel_manager.send(msg).await {
                log::debug!("Expected error for unknown channel: {}", e);
            }
        }
    });

    // Send a message to an unknown channel
    let unknown_msg = OutboundMessage::new("unknown-channel", "chat-1", "Test");
    bus.outbound_sender().send(unknown_msg).await.unwrap();

    // Wait a bit to ensure the message is processed
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Test passes if we reach here without panicking
    // The error should be logged but not crash the system
}

/// Test high-throughput message dispatch
#[tokio::test]
async fn test_high_throughput_dispatch() {
    // Create message bus
    let bus = Arc::new(MessageBus::new());

    // Create mock channel
    let (mock_channel, received_messages) = MockChannel::new("high-throughput");

    // Create channel manager
    let mut channel_manager = ChannelManager::new();
    channel_manager.register(Box::new(mock_channel));

    // Spawn the dispatch loop
    let bus_clone = bus.clone();
    let _dispatch_handle = tokio::spawn(async move {
        let mut outbound_rx = bus_clone.outbound_rx.lock().await;
        while let Some(msg) = outbound_rx.recv().await {
            if let Err(e) = channel_manager.send(msg).await {
                log::error!("Failed to dispatch: {}", e);
            }
        }
    });

    // Send many messages
    let message_count = 100;
    for i in 0..message_count {
        let msg = OutboundMessage::new(
            "high-throughput",
            &format!("chat-{}", i % 10),
            format!("Message {}", i),
        );
        bus.outbound_sender().send(msg).await.unwrap();
    }

    // Wait for all messages to be dispatched
    timeout(Duration::from_secs(10), async {
        loop {
            let msgs = received_messages.lock().await;
            if msgs.len() == message_count {
                return msgs.len();
            }
            drop(msgs);
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("All messages should be dispatched within 10 seconds");

    // Verify all messages received
    let msgs = received_messages.lock().await;
    assert_eq!(msgs.len(), message_count);
}
