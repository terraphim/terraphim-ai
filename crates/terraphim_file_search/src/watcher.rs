//! Knowledge-graph directory watcher for hot-reload of thesaurus files.
//!
//! [`KgWatcher`] monitors a directory for file changes using
//! `notify-debouncer-full` and calls [`KgPathScorer::update_thesaurus`]
//! whenever the content changes.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use notify_debouncer_full::notify::RecommendedWatcher;
use notify_debouncer_full::{
    DebounceEventResult, Debouncer, RecommendedCache, new_debouncer, notify::RecursiveMode,
};
use terraphim_automata::load_thesaurus_from_json;
use terraphim_types::{NormalizedTerm, NormalizedTermValue, Thesaurus};
use tracing::{error, info, warn};

use crate::kg_scorer::KgPathScorer;

/// Watches a directory for changes and hot-reloads the [`KgPathScorer`]
/// thesaurus when files are created, modified, or removed.
///
/// The watcher runs in a background thread. Drop it to stop watching.
pub struct KgWatcher {
    _debouncer: Debouncer<RecommendedWatcher, RecommendedCache>,
    watch_path: PathBuf,
}

impl KgWatcher {
    /// Spawn a watcher on `watch_path` with a 500 ms debounce window.
    ///
    /// When any file under `watch_path` changes, the watcher reloads all JSON
    /// thesaurus files from that directory and calls
    /// [`KgPathScorer::update_thesaurus`] with the merged result.
    pub fn new(
        watch_path: impl Into<PathBuf>,
        scorer: Arc<KgPathScorer>,
    ) -> Result<Self, notify_debouncer_full::notify::Error> {
        let watch_path = watch_path.into();
        let reload_path = watch_path.clone();

        let mut debouncer = new_debouncer(
            Duration::from_millis(500),
            None,
            move |result: DebounceEventResult| match result {
                Ok(events) if !events.is_empty() => {
                    info!(
                        path = %reload_path.display(),
                        events = events.len(),
                        "KgWatcher: filesystem change detected, reloading thesaurus"
                    );
                    match load_thesaurus_from_dir(&reload_path) {
                        Ok(thesaurus) => {
                            scorer.update_thesaurus(thesaurus);
                            info!(
                                path = %reload_path.display(),
                                "KgWatcher: thesaurus reloaded"
                            );
                        }
                        Err(err) => {
                            warn!(
                                path = %reload_path.display(),
                                error = %err,
                                "KgWatcher: failed to reload thesaurus"
                            );
                        }
                    }
                }
                Ok(_) => {}
                Err(errors) => {
                    for e in errors {
                        error!(error = %e, "KgWatcher: notify error");
                    }
                }
            },
        )?;

        debouncer.watch(&watch_path, RecursiveMode::Recursive)?;

        Ok(Self {
            _debouncer: debouncer,
            watch_path,
        })
    }

    /// The directory being watched.
    pub fn watch_path(&self) -> &Path {
        &self.watch_path
    }
}

/// Load a [`Thesaurus`] by merging all JSON files in `dir`.
///
/// Each file is read and parsed via [`load_thesaurus_from_json`].
/// Files that fail to parse are skipped with a warning.
pub(crate) fn load_thesaurus_from_dir(dir: &Path) -> Result<Thesaurus, std::io::Error> {
    let entries = std::fs::read_dir(dir)?;
    let mut combined = Thesaurus::new(dir.to_string_lossy().to_string());

    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if ext == "json" || ext == "jsonl" {
                match std::fs::read_to_string(&path) {
                    Ok(content) => match load_thesaurus_from_json(&content) {
                        Ok(t) => {
                            for (k, v) in &t {
                                let k: NormalizedTermValue = k.clone();
                                let v: NormalizedTerm = v.clone();
                                combined.insert(k, v);
                            }
                        }
                        Err(err) => {
                            warn!(
                                path = %path.display(),
                                error = %err,
                                "KgWatcher: failed to parse thesaurus file, skipping"
                            );
                        }
                    },
                    Err(err) => {
                        warn!(
                            path = %path.display(),
                            error = %err,
                            "KgWatcher: failed to read thesaurus file, skipping"
                        );
                    }
                }
            }
        }
    }

    Ok(combined)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use tempfile::TempDir;
    use terraphim_types::Thesaurus;

    use super::*;
    use crate::kg_scorer::KgPathScorer;

    fn empty_thesaurus() -> Thesaurus {
        Thesaurus::new("test".to_string())
    }

    /// Write a minimal thesaurus JSON in the new `{name, data}` format so
    /// that `load_thesaurus_from_json` can parse it.
    ///
    /// ```json
    /// {"name":"test","data":{"rust":{"id":"1","nterm":"rust","url":null}}}
    /// ```
    fn write_thesaurus_json(dir: &Path, filename: &str, key: &str, nterm: &str) {
        let content = format!(
            r#"{{"name":"test","data":{{"{key}":{{"id":1,"nterm":"{nterm}","url":null}}}}}}"#,
            key = key,
            nterm = nterm,
        );
        std::fs::write(dir.join(filename), content).unwrap();
    }

    /// Verify that `KgWatcher::new` succeeds on a valid directory.
    #[test]
    fn watcher_creates_without_error() {
        let tmp = TempDir::new().unwrap();
        let scorer = Arc::new(KgPathScorer::new(empty_thesaurus()));
        let watcher = KgWatcher::new(tmp.path(), scorer);
        assert!(
            watcher.is_ok(),
            "KgWatcher::new should succeed on a valid dir"
        );
    }

    /// Verify that writing a JSON file into the watched directory triggers a
    /// thesaurus reload via `update_thesaurus`.
    ///
    /// The test polls for up to 10 s (debounce 500 ms + FSEvents latency on macOS).
    #[test]
    fn watcher_triggers_update_on_file_write() {
        let tmp = TempDir::new().unwrap();
        let scorer = Arc::new(KgPathScorer::new(empty_thesaurus()));
        let _watcher =
            KgWatcher::new(tmp.path(), Arc::clone(&scorer)).expect("KgWatcher::new should succeed");

        // Small delay to let the watcher settle before writing.
        std::thread::sleep(std::time::Duration::from_millis(500));

        // Write a thesaurus file — the watcher should pick this up.
        write_thesaurus_json(tmp.path(), "kg.json", "rust", "rust");

        // Poll: the debounce window is 500 ms; allow up to 10 s total
        // (macOS FSEvents can have variable latency under load).
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            use fff_search::external_scorer::ExternalScorer;
            use fff_search::types::FileItem;
            let item = FileItem::new_raw(
                std::path::PathBuf::from("src/rust.rs"),
                "src/rust.rs".to_string(),
                "rust.rs".to_string(),
                0,
                0,
                None,
                false,
            );
            if scorer.score(&item) > 0 {
                return; // thesaurus was reloaded successfully
            }
            if std::time::Instant::now() >= deadline {
                panic!("KgWatcher did not reload thesaurus within 10 seconds");
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }
}
