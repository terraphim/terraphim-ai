use crate::bus::{InboundMessage, MessageBus, OutboundMessage};
use crate::channel::Channel;
use async_trait::async_trait;
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};

/// CLI channel adapter for interactive terminal use.
pub struct CliChannel {
    running: Arc<AtomicBool>,
}

impl CliChannel {
    /// Create a new CLI channel.
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl Default for CliChannel {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Channel for CliChannel {
    fn name(&self) -> &str {
        "cli"
    }

    async fn start(&self, bus: Arc<MessageBus>) -> anyhow::Result<()> {
        log::info!("CLI channel starting");
        self.running.store(true, Ordering::SeqCst);

        let inbound_tx = bus.inbound_sender();

        // Main input loop using tokio's async stdin
        let stdin = tokio::io::stdin();
        let reader = BufReader::new(stdin);
        let mut lines = reader.lines();

        println!("TinyClaw CLI Mode");
        println!("=================");
        println!("Type your messages and press Enter.");
        println!("Commands:");
        println!("  /quit or /exit - Exit the application");
        println!("  /reset - Clear session");
        println!();
        print!("> ");
        io::stdout().flush()?;

        while let Ok(Some(line)) = lines.next_line().await {
            let input = line.trim();

            if input.is_empty() {
                print!("> ");
                io::stdout().flush()?;
                continue;
            }

            if input == "/quit" || input == "/exit" {
                println!("Goodbye!");
                break;
            }

            if input == "/reset" {
                println!("Session reset (not yet implemented)");
                print!("> ");
                io::stdout().flush()?;
                continue;
            }

            // Create inbound message
            let msg = InboundMessage::new("cli", "local", "cli", input);

            if inbound_tx.send(msg).await.is_err() {
                log::error!("Failed to send message to bus");
                break;
            }

            print!("> ");
            io::stdout().flush()?;
        }

        self.running.store(false, Ordering::SeqCst);

        Ok(())
    }

    async fn stop(&self) -> anyhow::Result<()> {
        log::info!("CLI channel stopping");
        self.running.store(false, Ordering::SeqCst);
        Ok(())
    }

    async fn send(&self, msg: OutboundMessage) -> anyhow::Result<()> {
        println!("\n[{}]: {}\n", msg.channel, msg.content);
        print!("> ");
        io::stdout().flush()?;
        Ok(())
    }

    fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    fn is_allowed(&self, _sender_id: &str) -> bool {
        // CLI always allows the local user
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_channel_name() {
        let channel = CliChannel::new();
        assert_eq!(channel.name(), "cli");
    }

    #[test]
    fn test_cli_always_allowed() {
        let channel = CliChannel::new();
        assert!(channel.is_allowed("anyone"));
        assert!(channel.is_allowed(""));
    }
}
