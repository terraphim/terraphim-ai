//! Tokio-based update scheduler
//!
//! This module provides a scheduler for periodic update checks
//! using tokio's interval timer.

use crate::config::UpdateConfig;
use anyhow::{Context, Result};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

/// Notification type sent from scheduler to application
#[derive(Debug, Clone)]
pub enum UpdateNotification {
    /// An update is available
    UpdateAvailable {
        current_version: String,
        latest_version: String,
    },
    /// Update check failed
    CheckFailed { error: String },
    /// Scheduler stopped
    Stopped,
}

/// Update scheduler that runs periodic update checks
///
/// Uses tokio::time::interval for scheduling and sends notifications
/// through a channel when updates are available.
pub struct UpdateScheduler {
    config: Arc<UpdateConfig>,
    update_check_fn: Arc<dyn Fn() -> Result<UpdateCheckResult> + Send + Sync>,
    handle: Option<JoinHandle<()>>,
    notification_sender: Option<mpsc::UnboundedSender<UpdateNotification>>,
    is_running: bool,
}

/// Result of an update check
#[derive(Debug, Clone)]
pub enum UpdateCheckResult {
    /// No update available
    UpToDate,
    /// Update available with version info
    UpdateAvailable {
        current_version: String,
        latest_version: String,
    },
    /// Check failed with error
    Failed { error: String },
}

impl UpdateScheduler {
    /// Create a new update scheduler
    ///
    /// # Arguments
    /// * `config` - Update configuration
    /// * `update_check_fn` - Function to call for update checks
    ///
    /// # Returns
    /// * New UpdateScheduler instance
    ///
    /// # Example
    /// ```no_run
    /// use terraphim_update::scheduler::{UpdateScheduler, UpdateCheckResult};
    /// use terraphim_update::config::UpdateConfig;
    /// use std::sync::Arc;
    ///
    /// let config = UpdateConfig::default();
    /// let scheduler = UpdateScheduler::new(
    ///     Arc::new(config),
    ///     Arc::new(|| Ok(UpdateCheckResult::UpToDate))
    /// );
    /// ```
    pub fn new(
        config: Arc<UpdateConfig>,
        update_check_fn: Arc<dyn Fn() -> Result<UpdateCheckResult> + Send + Sync>,
    ) -> Self {
        Self {
            config,
            update_check_fn,
            handle: None,
            notification_sender: None,
            is_running: false,
        }
    }

    /// Create a channel for receiving update notifications
    ///
    /// Returns a receiver that will receive notifications when updates
    /// are available or checks fail.
    ///
    /// # Returns
    /// * `Ok(mpsc::UnboundedReceiver<UpdateNotification>)` - Notification receiver
    /// * `Err(anyhow::Error)` - Error if already started
    pub fn create_notification_channel(
        &mut self,
    ) -> Result<mpsc::UnboundedReceiver<UpdateNotification>> {
        if self.is_running {
            anyhow::bail!("Cannot create channel after scheduler is started");
        }

        let (sender, receiver) = mpsc::unbounded_channel();
        self.notification_sender = Some(sender);
        Ok(receiver)
    }

    /// Start the scheduler
    ///
    /// Begins periodic update checks based on the configured interval.
    /// Sends notifications through the created channel when updates are available.
    ///
    /// # Returns
    /// * `Ok(())` - Successfully started
    /// * `Err(anyhow::Error)` - Error starting scheduler
    ///
    /// # Example
    /// ```no_run
    /// # use terraphim_update::scheduler::{UpdateScheduler, UpdateCheckResult};
    /// # use terraphim_update::config::UpdateConfig;
    /// # use std::sync::Arc;
    /// #
    /// # async fn test() -> Result<(), anyhow::Error> {
    /// let config = UpdateConfig::default();
    /// let mut scheduler = UpdateScheduler::new(
    ///     Arc::new(config),
    ///     Arc::new(|| Ok(UpdateCheckResult::UpToDate))
    /// );
    ///
    /// let mut receiver = scheduler.create_notification_channel()?;
    /// scheduler.start().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn start(&mut self) -> Result<()> {
        if self.is_running {
            warn!("Scheduler is already running");
            return Ok(());
        }

        if !self.config.auto_update_enabled {
            info!("Auto-update is disabled, scheduler not starting");
            return Ok(());
        }

        let sender = self
            .notification_sender
            .clone()
            .context("No notification channel created. Call create_notification_channel() first")?;

        let config = Arc::clone(&self.config);
        let check_fn = Arc::clone(&self.update_check_fn);

        info!(
            "Starting update scheduler with interval: {:?}",
            config.auto_update_check_interval
        );

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(config.auto_update_check_interval);

            // Immediate first check
            let _ = Self::perform_check(&check_fn, &sender).await;

            loop {
                interval.tick().await;

                debug!("Performing scheduled update check");
                if let Err(e) = Self::perform_check(&check_fn, &sender).await {
                    error!("Error in scheduled check: {}", e);
                }
            }
        });

        self.handle = Some(handle);
        self.is_running = true;

        info!("Update scheduler started");
        Ok(())
    }

    /// Stop the scheduler
    ///
    /// Gracefully stops the scheduler, waiting for any ongoing
    /// update check to complete.
    ///
    /// # Returns
    /// * `Ok(())` - Successfully stopped
    /// * `Err(anyhow::Error)` - Error stopping scheduler
    pub async fn stop(&mut self) -> Result<()> {
        if !self.is_running {
            debug!("Scheduler is not running");
            return Ok(());
        }

        info!("Stopping update scheduler");

        if let Some(handle) = self.handle.take() {
            handle.abort();
            let _ = tokio::time::timeout(Duration::from_secs(5), handle).await;
        }

        self.is_running = false;

        if let Some(sender) = &self.notification_sender {
            let _ = sender.send(UpdateNotification::Stopped);
        }

        info!("Update scheduler stopped");
        Ok(())
    }

    /// Check if scheduler is running
    pub fn is_running(&self) -> bool {
        self.is_running
    }

    /// Check if an update check should be performed based on interval
    ///
    /// This is a helper for manual check logic that respects the configured interval.
    ///
    /// # Arguments
    /// * `last_check_time` - Timestamp of the last update check
    ///
    /// # Returns
    /// * `true` - Check should be performed
    /// * `false` - Check should be skipped
    pub fn should_check(&self, last_check_time: Option<std::time::Instant>) -> bool {
        if !self.config.auto_update_enabled {
            return false;
        }

        match last_check_time {
            Some(last) => {
                let elapsed = last.elapsed();
                elapsed >= self.config.auto_update_check_interval
            }
            None => true,
        }
    }

    /// Perform a single update check
    ///
    /// Internal method called by the scheduler loop.
    async fn perform_check(
        check_fn: &Arc<dyn Fn() -> Result<UpdateCheckResult> + Send + Sync>,
        sender: &mpsc::UnboundedSender<UpdateNotification>,
    ) -> Result<()> {
        let result = tokio::task::spawn_blocking({
            let check_fn = Arc::clone(check_fn);
            move || check_fn()
        })
        .await
        .context("Failed to spawn blocking task for update check")?;

        let check_result = result.context("Update check function failed")?;

        match check_result {
            UpdateCheckResult::UpdateAvailable {
                current_version,
                latest_version,
            } => {
                info!(
                    "Update available: {} -> {}",
                    current_version, latest_version
                );
                sender
                    .send(UpdateNotification::UpdateAvailable {
                        current_version,
                        latest_version,
                    })
                    .context("Failed to send update notification")?;
            }
            UpdateCheckResult::UpToDate => {
                debug!("Up to date, no notification needed");
            }
            UpdateCheckResult::Failed { error } => {
                warn!("Update check failed: {}", error);
                sender
                    .send(UpdateNotification::CheckFailed { error })
                    .context("Failed to send error notification")?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Instant;

    #[test]
    fn test_scheduler_creation() {
        let config = UpdateConfig::default();
        let check_count = Arc::new(AtomicUsize::new(0));
        let check_count_clone = Arc::clone(&check_count);

        let check_fn = Arc::new(move || {
            check_count_clone.fetch_add(1, Ordering::SeqCst);
            Ok(UpdateCheckResult::UpToDate)
        });

        let scheduler = UpdateScheduler::new(Arc::new(config), check_fn);
        assert!(!scheduler.is_running());
    }

    #[test]
    fn test_should_check_with_interval() {
        let config = UpdateConfig {
            auto_update_enabled: true,
            auto_update_check_interval: Duration::from_secs(60),
        };

        let check_fn = Arc::new(|| Ok(UpdateCheckResult::UpToDate));
        let scheduler = UpdateScheduler::new(Arc::new(config), check_fn);

        // Should check when no last check time
        assert!(scheduler.should_check(None));

        // Should not check when interval hasn't elapsed
        let recent = Instant::now() - Duration::from_secs(30);
        assert!(!scheduler.should_check(Some(recent)));

        // Should check when interval has elapsed
        let old = Instant::now() - Duration::from_secs(120);
        assert!(scheduler.should_check(Some(old)));
    }

    #[test]
    fn test_should_check_when_disabled() {
        let config = UpdateConfig {
            auto_update_enabled: false,
            auto_update_check_interval: Duration::from_secs(60),
        };

        let check_fn = Arc::new(|| Ok(UpdateCheckResult::UpToDate));
        let scheduler = UpdateScheduler::new(Arc::new(config), check_fn);

        // Should not check when auto-update is disabled
        assert!(!scheduler.should_check(None));
    }

    #[tokio::test]
    async fn test_scheduler_starts_and_stops() {
        let config = UpdateConfig {
            auto_update_enabled: true,
            auto_update_check_interval: Duration::from_millis(100),
        };

        let check_fn = Arc::new(|| Ok(UpdateCheckResult::UpToDate));
        let mut scheduler = UpdateScheduler::new(Arc::new(config), check_fn);

        let mut receiver = scheduler.create_notification_channel().unwrap();
        scheduler.start().await.unwrap();
        assert!(scheduler.is_running());

        // Wait a bit for at least one check
        tokio::time::sleep(Duration::from_millis(200)).await;

        scheduler.stop().await.unwrap();
        assert!(!scheduler.is_running());

        // Should receive stopped notification
        let notification = receiver.try_recv().unwrap();
        matches!(notification, UpdateNotification::Stopped);
    }

    #[tokio::test]
    async fn test_scheduler_with_short_interval() {
        let config = UpdateConfig {
            auto_update_enabled: true,
            auto_update_check_interval: Duration::from_millis(50),
        };

        let check_fn = Arc::new(|| Ok(UpdateCheckResult::UpToDate));
        let mut scheduler = UpdateScheduler::new(Arc::new(config), check_fn);

        let _receiver = scheduler.create_notification_channel().unwrap();
        scheduler.start().await.unwrap();

        // Wait for a few checks
        tokio::time::sleep(Duration::from_millis(150)).await;

        scheduler.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_scheduler_notification_update_available() {
        let config = UpdateConfig {
            auto_update_enabled: true,
            auto_update_check_interval: Duration::from_secs(1000),
        };

        let check_fn = Arc::new(|| {
            Ok(UpdateCheckResult::UpdateAvailable {
                current_version: "1.0.0".to_string(),
                latest_version: "1.1.0".to_string(),
            })
        });

        let mut scheduler = UpdateScheduler::new(Arc::new(config), check_fn);

        let mut receiver = scheduler.create_notification_channel().unwrap();
        scheduler.start().await.unwrap();

        // Wait for initial check
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Should receive update notification
        let notification = receiver.try_recv().unwrap();
        match notification {
            UpdateNotification::UpdateAvailable {
                current_version,
                latest_version,
            } => {
                assert_eq!(current_version, "1.0.0");
                assert_eq!(latest_version, "1.1.0");
            }
            _ => panic!("Expected UpdateAvailable notification"),
        }

        scheduler.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_scheduler_notification_check_failed() {
        let config = UpdateConfig {
            auto_update_enabled: true,
            auto_update_check_interval: Duration::from_secs(1000),
        };

        let check_fn = Arc::new(|| {
            Ok(UpdateCheckResult::Failed {
                error: "Network error".to_string(),
            })
        });

        let mut scheduler = UpdateScheduler::new(Arc::new(config), check_fn);

        let mut receiver = scheduler.create_notification_channel().unwrap();
        scheduler.start().await.unwrap();

        // Wait for initial check
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Should receive error notification
        let notification = receiver.try_recv().unwrap();
        match notification {
            UpdateNotification::CheckFailed { error } => {
                assert_eq!(error, "Network error");
            }
            _ => panic!("Expected CheckFailed notification"),
        }

        scheduler.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_scheduler_disabled_does_not_start() {
        let config = UpdateConfig {
            auto_update_enabled: false,
            auto_update_check_interval: Duration::from_millis(50),
        };

        let check_fn = Arc::new(|| Ok(UpdateCheckResult::UpToDate));
        let mut scheduler = UpdateScheduler::new(Arc::new(config), check_fn);

        let _receiver = scheduler.create_notification_channel().unwrap();
        scheduler.start().await.unwrap();

        assert!(!scheduler.is_running());

        scheduler.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_scheduler_already_running() {
        let config = UpdateConfig {
            auto_update_enabled: true,
            auto_update_check_interval: Duration::from_millis(50),
        };

        let check_fn = Arc::new(|| Ok(UpdateCheckResult::UpToDate));
        let mut scheduler = UpdateScheduler::new(Arc::new(config), check_fn);

        let _receiver = scheduler.create_notification_channel().unwrap();
        scheduler.start().await.unwrap();

        // Starting again should be a no-op
        let result = scheduler.start().await;
        assert!(result.is_ok());
        assert!(scheduler.is_running());

        scheduler.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_scheduler_stop_when_not_running() {
        let config = UpdateConfig::default();
        let check_fn = Arc::new(|| Ok(UpdateCheckResult::UpToDate));
        let mut scheduler = UpdateScheduler::new(Arc::new(config), check_fn);

        // Stopping when not running should be a no-op
        let result = scheduler.stop().await;
        assert!(result.is_ok());
        assert!(!scheduler.is_running());
    }
}
