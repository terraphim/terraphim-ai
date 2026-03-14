//! WORKFLOW.md file watcher for dynamic configuration reload.
//!
//! Uses `notify` to watch for filesystem changes to the WORKFLOW.md file
//! and signals the orchestrator when a reload is needed.

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

/// Notification that the WORKFLOW.md file has changed.
#[derive(Debug, Clone)]
pub struct WorkflowChanged {
    /// Path to the changed file.
    pub path: PathBuf,
}

/// Watch a WORKFLOW.md file for changes and send notifications.
pub struct WorkflowWatcher {
    _watcher: RecommendedWatcher,
    rx: mpsc::Receiver<WorkflowChanged>,
}

impl WorkflowWatcher {
    /// Start watching the given workflow file.
    ///
    /// Returns a watcher that must be kept alive and a receiver for change notifications.
    pub fn start(workflow_path: &Path) -> crate::Result<Self> {
        let canonical = workflow_path
            .canonicalize()
            .map_err(|e| crate::SymphonyError::MissingWorkflowFile {
                path: format!("{}: {e}", workflow_path.display()),
            })?;

        let (tx, rx) = mpsc::channel(16);
        let watched_path = canonical.clone();

        let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            match res {
                Ok(event) => {
                    let dominated = matches!(
                        event.kind,
                        EventKind::Modify(_) | EventKind::Create(_)
                    );
                    if dominated {
                        // Check if the event is for our file
                        let relevant = event.paths.iter().any(|p| {
                            p.canonicalize()
                                .ok()
                                .as_deref()
                                == Some(watched_path.as_path())
                        });
                        if relevant {
                            debug!("workflow file changed");
                            let _ = tx.blocking_send(WorkflowChanged {
                                path: watched_path.clone(),
                            });
                        }
                    }
                }
                Err(e) => {
                    warn!("file watch error: {e}");
                }
            }
        })
        .map_err(|e| crate::SymphonyError::Io(std::io::Error::other(
            format!("failed to create file watcher: {e}"),
        )))?;

        // Watch the parent directory (notify often misses direct file watches)
        let watch_dir = canonical
            .parent()
            .unwrap_or_else(|| Path::new("."));

        watcher
            .watch(watch_dir, RecursiveMode::NonRecursive)
            .map_err(|e| crate::SymphonyError::Io(std::io::Error::other(
                format!("failed to watch {}: {e}", watch_dir.display()),
            )))?;

        info!(path = %canonical.display(), "watching workflow file for changes");

        Ok(Self {
            _watcher: watcher,
            rx,
        })
    }

    /// Receive the next change notification.
    pub async fn recv(&mut self) -> Option<WorkflowChanged> {
        self.rx.recv().await
    }
}
