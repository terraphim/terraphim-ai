//! File-watching provider discovery for hot-reload.
//!
//! This module watches a directory of provider markdown files and
//! automatically reloads the registry when files change. Requires
//! the `file-watch` feature.

use crate::registry::ProviderRegistry;
use crate::types::RoutingError;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

/// Watches a directory of provider markdown files for changes.
///
/// When files are created, modified, or deleted, the registry
/// is automatically reloaded after a debounce period.
pub struct ProviderDirectoryWatcher {
    /// Path being watched
    path: PathBuf,
    /// Watcher handle (kept alive to continue receiving events)
    _watcher: RecommendedWatcher,
    /// Handle to stop the reload task
    stop_sender: mpsc::Sender<()>,
}

impl ProviderDirectoryWatcher {
    /// Start watching a directory for provider file changes.
    ///
    /// The `registry` will be reloaded whenever files in `path` change,
    /// with a 500ms debounce to batch rapid changes.
    pub fn start(
        path: PathBuf,
        registry: Arc<Mutex<ProviderRegistry>>,
    ) -> Result<Self, RoutingError> {
        let (tx, rx) = std::sync::mpsc::channel();
        let (stop_sender, stop_receiver) = mpsc::channel::<()>(1);

        let watcher = notify::recommended_watcher(tx)
            .map_err(|e| RoutingError::Io(format!("Failed to create file watcher: {}", e)))?;

        let mut watcher = watcher;
        watcher
            .watch(&path, RecursiveMode::NonRecursive)
            .map_err(|e| {
                RoutingError::Io(format!("Failed to watch directory {:?}: {}", path, e))
            })?;

        let reload_path = path.clone();
        tokio::spawn(Self::reload_task(rx, stop_receiver, reload_path, registry));

        Ok(Self {
            path,
            _watcher: watcher,
            stop_sender,
        })
    }

    /// The background task that debounces file events and reloads.
    async fn reload_task(
        event_rx: std::sync::mpsc::Receiver<Result<notify::Event, notify::Error>>,
        mut stop_rx: mpsc::Receiver<()>,
        path: PathBuf,
        registry: Arc<Mutex<ProviderRegistry>>,
    ) {
        loop {
            // Check for stop signal or file events
            tokio::select! {
                _ = stop_rx.recv() => {
                    tracing::info!(path = ?path, "Provider directory watcher stopped");
                    break;
                }
                _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {
                    // Drain any pending events
                    let mut has_events = false;
                    while event_rx.try_recv().is_ok() {
                        has_events = true;
                    }

                    if has_events {
                        // Debounce: wait 500ms for more events
                        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                        // Drain again after debounce
                        while event_rx.try_recv().is_ok() {}

                        tracing::info!(path = ?path, "Reloading providers from directory");
                        let mut reg = registry.lock().await;
                        match reg.load_from_dir(&path).await {
                            Ok(count) => {
                                tracing::info!(
                                    path = ?path,
                                    provider_count = count,
                                    "Providers reloaded successfully"
                                );
                            }
                            Err(e) => {
                                tracing::warn!(
                                    path = ?path,
                                    error = %e,
                                    "Failed to reload providers"
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    /// Get the watched path.
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    /// Stop watching (the watcher is also stopped when dropped).
    pub async fn stop(self) {
        let _ = self.stop_sender.send(()).await;
    }
}

impl std::fmt::Debug for ProviderDirectoryWatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProviderDirectoryWatcher")
            .field("path", &self.path)
            .finish()
    }
}
