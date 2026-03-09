//! Integration tests for Slack channel adapter.
//!
//! These tests require live Slack credentials and are gated behind
//! environment variables. Run with:
//!
//! ```bash
//! SLACK_BOT_TOKEN=xoxb-... SLACK_APP_TOKEN=xapp-... SLACK_TEST_CHANNEL=C... \
//!     cargo test -p terraphim_tinyclaw --features slack --test slack_integration -- --ignored
//! ```

#[cfg(feature = "slack")]
mod slack_tests {
    use std::sync::Arc;
    use terraphim_tinyclaw::bus::MessageBus;
    use terraphim_tinyclaw::channel::Channel;

    fn slack_config_from_env() -> Option<terraphim_tinyclaw::config::SlackConfig> {
        let bot_token = std::env::var("SLACK_BOT_TOKEN").ok()?;
        let app_token = std::env::var("SLACK_APP_TOKEN").ok()?;
        Some(terraphim_tinyclaw::config::SlackConfig {
            bot_token,
            app_token,
            allow_from: vec!["*".to_string()],
        })
    }

    fn test_channel_id() -> Option<String> {
        std::env::var("SLACK_TEST_CHANNEL").ok()
    }

    #[tokio::test]
    #[ignore]
    async fn test_slack_auth_and_start() {
        let config = slack_config_from_env()
            .expect("Set SLACK_BOT_TOKEN and SLACK_APP_TOKEN to run this test");

        let channel = terraphim_tinyclaw::channels::slack::SlackChannel::new(config);
        assert_eq!(channel.name(), "slack");
        assert!(!channel.is_running());

        let bus = Arc::new(MessageBus::new());
        channel
            .start(bus)
            .await
            .expect("Slack channel should start with valid credentials");
        assert!(channel.is_running());

        channel
            .stop()
            .await
            .expect("Slack channel should stop cleanly");
    }

    #[tokio::test]
    #[ignore]
    async fn test_slack_send_message() {
        let config = slack_config_from_env()
            .expect("Set SLACK_BOT_TOKEN and SLACK_APP_TOKEN to run this test");
        let channel_id = test_channel_id().expect("Set SLACK_TEST_CHANNEL to run this test");

        let channel = terraphim_tinyclaw::channels::slack::SlackChannel::new(config);

        let msg = terraphim_tinyclaw::bus::OutboundMessage::new(
            "slack",
            &channel_id,
            "Integration test message from terraphim_tinyclaw",
        );
        channel
            .send(msg)
            .await
            .expect("Should send message to Slack channel");
    }
}
