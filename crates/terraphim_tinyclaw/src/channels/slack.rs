//! Slack channel adapter using slack-morphism Socket Mode.

use crate::bus::{MessageBus, OutboundMessage};
use crate::channel::Channel;
use crate::config::SlackConfig;
use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::{Mutex, RwLock};

/// Queued outbound message for retry on reconnect or send failure.
struct QueuedMessage {
    chat_id: String,
    content: String,
}

impl QueuedMessage {
    /// Convert back to an OutboundMessage for retry.
    fn into_outbound(self) -> OutboundMessage {
        OutboundMessage::new("slack", self.chat_id, self.content)
    }
}

/// Shared state passed to the Socket Mode push events callback via `with_user_state`.
///
/// slack-morphism requires `fn` pointers (not closures) for callbacks, so all
/// captured state must be registered through `SlackClientEventsListenerEnvironment::with_user_state`
/// and retrieved inside the callback from `SlackClientEventsUserState`.
struct SlackListenerState {
    inbound_tx: tokio::sync::mpsc::Sender<crate::bus::InboundMessage>,
    allow_from: Vec<String>,
    bot_user_id: String,
    seen_events: Arc<RwLock<HashSet<String>>>,
    user_cache: Arc<RwLock<HashMap<String, String>>>,
    token: slack_morphism::prelude::SlackApiToken,
}

/// Slack channel adapter using slack-morphism Socket Mode.
pub struct SlackChannel {
    config: SlackConfig,
    running: Arc<AtomicBool>,
    outgoing_queue: Arc<Mutex<Vec<QueuedMessage>>>,
}

impl SlackChannel {
    /// Create a new Slack channel adapter.
    pub fn new(config: SlackConfig) -> Self {
        Self {
            config,
            running: Arc::new(AtomicBool::new(false)),
            outgoing_queue: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Drain all queued outbound messages for retry.
    pub async fn drain_queued_messages(&self) -> Vec<OutboundMessage> {
        let mut queue = self.outgoing_queue.lock().await;
        queue.drain(..).map(QueuedMessage::into_outbound).collect()
    }
}

#[async_trait]
impl Channel for SlackChannel {
    fn name(&self) -> &str {
        "slack"
    }

    async fn start(&self, bus: Arc<MessageBus>) -> anyhow::Result<()> {
        log::info!("Slack channel starting");
        self.running.store(true, Ordering::SeqCst);

        use slack_morphism::prelude::*;

        let client = Arc::new(SlackClient::new(
            SlackClientHyperConnector::new()
                .map_err(|e| anyhow::anyhow!("Failed to create Slack connector: {}", e))?,
        ));
        let token = SlackApiToken::new(self.config.bot_token.clone().into());

        // Fetch bot user ID for self-detection
        let session = client.open_session(&token);
        let auth_response = session
            .auth_test()
            .await
            .map_err(|e| anyhow::anyhow!("Slack auth.test failed: {}", e))?;
        let bot_user_id = auth_response.user_id.to_string();
        log::info!("Slack bot user ID: {}", bot_user_id);

        let app_token = SlackApiToken::new(self.config.app_token.clone().into());
        let running = self.running.clone();

        // Build listener state for the callback fn
        let listener_state = SlackListenerState {
            inbound_tx: bus.inbound_sender(),
            allow_from: self.config.allow_from.clone(),
            bot_user_id,
            seen_events: Arc::new(RwLock::new(HashSet::new())),
            user_cache: Arc::new(RwLock::new(HashMap::new())),
            token: token.clone(),
        };

        let listener_client = client.clone();

        tokio::spawn(async move {
            let socket_mode_callbacks =
                SlackSocketModeListenerCallbacks::new().with_push_events(push_events_handler);

            let listener_env = Arc::new(
                SlackClientEventsListenerEnvironment::new(listener_client)
                    .with_user_state(listener_state),
            );

            let listener = SlackClientSocketModeListener::new(
                &SlackClientSocketModeConfig::new(),
                listener_env,
                socket_mode_callbacks,
            );

            if let Err(e) = listener.listen_for(&app_token).await {
                log::error!("Slack Socket Mode listen_for error: {}", e);
                running.store(false, Ordering::SeqCst);
                return;
            }

            listener.serve().await;
            running.store(false, Ordering::SeqCst);
        });

        Ok(())
    }

    async fn stop(&self) -> anyhow::Result<()> {
        log::info!("Slack channel stopping");
        self.running.store(false, Ordering::SeqCst);
        Ok(())
    }

    async fn send(&self, msg: OutboundMessage) -> anyhow::Result<()> {
        use slack_morphism::prelude::*;

        let client = SlackClient::new(
            SlackClientHyperConnector::new()
                .map_err(|e| anyhow::anyhow!("Failed to create Slack connector: {}", e))?,
        );
        let token = SlackApiToken::new(self.config.bot_token.clone().into());
        let session = client.open_session(&token);

        // Retry previously queued messages first
        let queued = self.drain_queued_messages().await;
        for retry_msg in queued {
            let retry_channel_id: SlackChannelId = retry_msg.chat_id.clone().into();
            let req = SlackApiChatPostMessageRequest::new(
                retry_channel_id,
                SlackMessageContent::new().with_text(retry_msg.content.clone()),
            );
            if let Err(e) = session.chat_post_message(&req).await {
                log::warn!("Slack retry send failed, re-queuing: {}", e);
                self.outgoing_queue.lock().await.push(QueuedMessage {
                    chat_id: retry_msg.chat_id,
                    content: retry_msg.content,
                });
            }
        }

        // Send the new message
        let formatted = crate::format::markdown_to_slack_mrkdwn(&msg.content);
        let chunks = crate::format::chunk_message(&formatted, 4000);

        let channel_id: SlackChannelId = msg.chat_id.clone().into();

        for chunk in chunks {
            let req = SlackApiChatPostMessageRequest::new(
                channel_id.clone(),
                SlackMessageContent::new().with_text(chunk.clone()),
            );
            // Retry-on-failure: queue instead of propagating error (NanoClaw pattern)
            if let Err(e) = session.chat_post_message(&req).await {
                log::warn!("Slack send failed, queuing for retry: {}", e);
                self.outgoing_queue.lock().await.push(QueuedMessage {
                    chat_id: msg.chat_id.clone(),
                    content: chunk,
                });
            }
        }
        Ok(())
    }

    fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    fn is_allowed(&self, sender_id: &str) -> bool {
        self.config.is_allowed(sender_id)
    }
}

// --- Standalone callback fn (required by slack-morphism fn pointer constraint) ---

/// Push events callback for Socket Mode.
///
/// This is a standalone `async fn` (not a closure) because slack-morphism's
/// `with_push_events` requires a `fn` pointer. All shared state is retrieved
/// from `SlackClientEventsUserState` which was registered via `with_user_state`.
async fn push_events_handler(
    event: slack_morphism::prelude::SlackPushEventCallback,
    client: Arc<slack_morphism::prelude::SlackHyperClient>,
    states: slack_morphism::prelude::SlackClientEventsUserState,
) -> slack_morphism::UserCallbackResult<()> {
    use slack_morphism::prelude::*;

    // Extract our listener state from the user state storage
    let (tx, allowed, bot_id, seen, cache, token) = {
        let states_read = states.read().await;
        let s = states_read
            .get_user_state::<SlackListenerState>()
            .expect("SlackListenerState not registered in user state");
        (
            s.inbound_tx.clone(),
            s.allow_from.clone(),
            s.bot_user_id.clone(),
            s.seen_events.clone(),
            s.user_cache.clone(),
            s.token.clone(),
        )
    };

    // Dedup by event ID
    let event_id = event.event_id.to_string();
    {
        let mut seen_guard = seen.write().await;
        if is_duplicate_event(&event_id, &mut seen_guard) {
            return Ok(());
        }
    }

    // Extract message or app_mention events
    let (sender_user, sender_bot_id, channel_id, text) = match &event.event {
        SlackEventCallbackBody::Message(msg) => {
            let user = msg.sender.user.as_ref().map(|u| u.to_string());
            let bot = msg.sender.bot_id.as_ref().map(|b| b.to_string());
            let channel = msg.origin.channel.as_ref().map(|c| c.to_string());
            let text = msg.content.as_ref().and_then(|c| c.text.clone());
            (user, bot, channel, text)
        }
        SlackEventCallbackBody::AppMention(mention) => {
            let user = Some(mention.user.to_string());
            let channel = Some(mention.channel.to_string());
            let text = mention.content.text.clone();
            (user, None, channel, text)
        }
        _ => return Ok(()),
    };

    let sender_id = match sender_user {
        Some(ref id) if !id.is_empty() => id.clone(),
        _ => return Ok(()), // No sender, skip
    };

    let chat_id = match channel_id {
        Some(id) => id,
        None => return Ok(()),
    };

    let text = match text {
        Some(t) if !t.is_empty() => t,
        _ => return Ok(()),
    };

    // Check allowlist
    if !crate::channel::is_sender_allowed(&allowed, &sender_id) {
        log::warn!(
            "Slack: rejected message from unauthorized user: {}",
            sender_id
        );
        return Ok(());
    }

    // Check if own message -- set is_from_me metadata, don't filter out
    let from_me = is_own_message(&sender_id, sender_bot_id.as_deref(), &bot_id);

    // Strip @mention from text
    let cleaned_text = strip_bot_mention(&text, &bot_id);

    // Resolve user name from cache
    let _display_name = resolve_user_name(&client, &token, &sender_id, &cache).await;

    // Build InboundMessage with metadata
    let mut inbound = crate::bus::InboundMessage::new("slack", &sender_id, &chat_id, &cleaned_text);
    if from_me {
        inbound
            .metadata
            .insert("is_from_me".to_string(), "true".to_string());
        inbound
            .metadata
            .insert("is_bot_message".to_string(), "true".to_string());
    }

    if let Err(e) = tx.send(inbound).await {
        log::error!("Failed to forward Slack message to bus: {}", e);
    }

    Ok(())
}

// --- Helper functions (testable without network) ---

/// Strip bot @mention from incoming message text.
/// Converts "<@U_BOT_ID> hello" to "hello".
fn strip_bot_mention(text: &str, bot_user_id: &str) -> String {
    let mention = format!("<@{}>", bot_user_id);
    text.replace(&mention, "").trim().to_string()
}

/// Check if a message event is from the bot itself.
fn is_own_message(event_user: &str, event_bot_id: Option<&str>, bot_user_id: &str) -> bool {
    event_user == bot_user_id || event_bot_id.is_some_and(|id| !id.is_empty())
}

/// Check if an event has already been processed (dedup).
fn is_duplicate_event(event_id: &str, seen: &mut HashSet<String>) -> bool {
    !seen.insert(event_id.to_string())
}

/// Resolve a Slack user ID to a display name, using cache.
async fn resolve_user_name(
    client: &Arc<slack_morphism::prelude::SlackHyperClient>,
    token: &slack_morphism::prelude::SlackApiToken,
    user_id: &str,
    cache: &RwLock<HashMap<String, String>>,
) -> String {
    // Check cache first
    {
        let guard = cache.read().await;
        if let Some(name) = guard.get(user_id) {
            return name.clone();
        }
    }

    // Fetch from API
    use slack_morphism::prelude::*;
    let session = client.open_session(token);
    let req = SlackApiUsersInfoRequest::new(user_id.into());
    match session.users_info(&req).await {
        Ok(resp) => {
            let name = resp
                .user
                .profile
                .as_ref()
                .and_then(|p| p.display_name.clone())
                .or_else(|| resp.user.profile.as_ref().and_then(|p| p.real_name.clone()))
                .or(resp.user.name)
                .unwrap_or_else(|| user_id.to_string());

            // Cache it
            cache
                .write()
                .await
                .insert(user_id.to_string(), name.clone());
            name
        }
        Err(e) => {
            log::warn!("Failed to resolve Slack user {}: {}", user_id, e);
            user_id.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slack_channel_name() {
        let config = SlackConfig {
            bot_token: "xoxb-test".to_string(),
            app_token: "xapp-test".to_string(),
            allow_from: vec!["U111".to_string()],
        };
        let channel = SlackChannel::new(config);
        assert_eq!(channel.name(), "slack");
    }

    #[test]
    fn test_strip_bot_mention() {
        let result = strip_bot_mention("<@UBOT123> hello world", "UBOT123");
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_strip_bot_mention_no_match() {
        let result = strip_bot_mention("hello world", "UBOT123");
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_strip_bot_mention_multiple() {
        let result = strip_bot_mention("<@UBOT123> hey <@UBOT123> there", "UBOT123");
        assert_eq!(result, "hey  there");
    }

    #[test]
    fn test_is_own_message_by_user_id() {
        assert!(is_own_message("UBOT123", None, "UBOT123"));
    }

    #[test]
    fn test_is_own_message_by_bot_id() {
        assert!(is_own_message("UOTHER", Some("BBOT456"), "UBOT123"));
    }

    #[test]
    fn test_is_own_message_other_user() {
        assert!(!is_own_message("UOTHER", None, "UBOT123"));
    }

    #[test]
    fn test_is_duplicate_event() {
        let mut seen = HashSet::new();
        assert!(!is_duplicate_event("evt1", &mut seen));
        assert!(is_duplicate_event("evt1", &mut seen));
        assert!(!is_duplicate_event("evt2", &mut seen));
    }

    #[tokio::test]
    async fn test_outgoing_queue_on_disconnect() {
        let config = SlackConfig {
            bot_token: "xoxb-test".to_string(),
            app_token: "xapp-test".to_string(),
            allow_from: vec!["U111".to_string()],
        };
        let channel = SlackChannel::new(config);

        // Simulate pushing to outgoing queue
        channel.outgoing_queue.lock().await.push(QueuedMessage {
            chat_id: "C123".to_string(),
            content: "queued message".to_string(),
        });

        let drained = channel.drain_queued_messages().await;
        assert_eq!(drained.len(), 1);
        assert_eq!(drained[0].chat_id, "C123");
        assert_eq!(drained[0].content, "queued message");
        assert_eq!(drained[0].channel, "slack");

        // Queue should be empty after drain
        let drained_again = channel.drain_queued_messages().await;
        assert!(drained_again.is_empty());
    }

    #[test]
    fn test_is_from_me_metadata() {
        // Verify that is_from_me metadata is set correctly on InboundMessage
        let mut inbound =
            crate::bus::InboundMessage::new("slack", "UBOT123", "C456", "hello from bot");
        let from_me = is_own_message("UBOT123", None, "UBOT123");
        assert!(from_me);
        if from_me {
            inbound
                .metadata
                .insert("is_from_me".to_string(), "true".to_string());
        }
        assert_eq!(inbound.metadata.get("is_from_me").unwrap(), "true");
    }

    #[test]
    fn test_strip_bot_mention_with_surrounding_whitespace() {
        let result = strip_bot_mention("  <@UBOT123>  hello  ", "UBOT123");
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_strip_bot_mention_mention_only() {
        let result = strip_bot_mention("<@UBOT123>", "UBOT123");
        assert_eq!(result, "");
    }

    #[test]
    fn test_is_own_message_empty_bot_id() {
        // An empty string bot_id (Some("")) should NOT trigger is_own_message.
        // Slack may set bot_id to "" for certain app integrations.
        assert!(
            !is_own_message("UOTHER", Some(""), "UBOT123"),
            "Empty bot_id should not be treated as own message"
        );
    }

    #[tokio::test]
    async fn test_outgoing_queue_multiple_messages() {
        let config = SlackConfig {
            bot_token: "xoxb-test".to_string(),
            app_token: "xapp-test".to_string(),
            allow_from: vec!["U111".to_string()],
        };
        let channel = SlackChannel::new(config);

        {
            let mut queue = channel.outgoing_queue.lock().await;
            queue.push(QueuedMessage {
                chat_id: "C_FIRST".to_string(),
                content: "first message".to_string(),
            });
            queue.push(QueuedMessage {
                chat_id: "C_SECOND".to_string(),
                content: "second message".to_string(),
            });
            queue.push(QueuedMessage {
                chat_id: "C_THIRD".to_string(),
                content: "third message".to_string(),
            });
        }

        let drained = channel.drain_queued_messages().await;
        assert_eq!(drained.len(), 3, "All three messages should be drained");

        // Verify FIFO order
        assert_eq!(drained[0].chat_id, "C_FIRST");
        assert_eq!(drained[0].content, "first message");
        assert_eq!(drained[0].channel, "slack");

        assert_eq!(drained[1].chat_id, "C_SECOND");
        assert_eq!(drained[1].content, "second message");
        assert_eq!(drained[1].channel, "slack");

        assert_eq!(drained[2].chat_id, "C_THIRD");
        assert_eq!(drained[2].content, "third message");
        assert_eq!(drained[2].channel, "slack");

        // Queue must be empty after drain
        let drained_again = channel.drain_queued_messages().await;
        assert!(
            drained_again.is_empty(),
            "Queue should be empty after drain"
        );
    }
}
