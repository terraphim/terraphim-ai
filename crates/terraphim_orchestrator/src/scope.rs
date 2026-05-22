use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Instant;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::worktree_guard::WorktreeGuard;

const INHERITED_GIT_ENV: [&str; 4] = ["GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE", "GIT_PREFIX"];

fn clear_git_env(command: &mut tokio::process::Command) {
    for var in INHERITED_GIT_ENV {
        command.env_remove(var);
    }
}

fn clear_std_git_env(command: &mut std::process::Command) {
    for var in INHERITED_GIT_ENV {
        command.env_remove(var);
    }
}

/// Filename of the ownership manifest written at the root of every ADF
/// worktree.  Presence of this file with valid contents is the gate for
/// cleanup: sweep only deletes entries carrying this sentinel.
pub const WORKTREE_MANIFEST_FILENAME: &str = ".adf-worktree-manifest.json";

/// Ownership manifest stored at the root of each ADF worktree.
///
/// Existence of this file with matching repo and path fields is the
/// single gate for sweep/cleanup.  Directories without a valid manifest
/// are preserved regardless of name prefix or location.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorktreeManifest {
    /// Schema version for forward compatibility.
    pub version: u32,
    /// Git repository this worktree belongs to.
    pub repo_path: String,
    /// Absolute path of this worktree (self-referential).
    pub worktree_path: String,
    /// Name of the agent or component that created this worktree.
    pub creator: String,
    /// Session or correlation ID linking this worktree to its task.
    pub session_id: String,
    /// Process ID that performed the `git worktree add`.
    pub pid: u32,
    /// ISO-8601 timestamp of creation.
    pub created_at: String,
}

impl WorktreeManifest {
    /// Current schema version. Increment when the struct changes
    /// incompatibly.
    pub const CURRENT_VERSION: u32 = 1;

    /// Write a manifest to `dir / WORKTREE_MANIFEST_FILENAME`.
    pub fn write_to_dir(&self, dir: &Path) -> Result<(), std::io::Error> {
        let path = dir.join(WORKTREE_MANIFEST_FILENAME);
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(&path, json)?;
        debug!(path = %path.display(), "worktree manifest written");
        Ok(())
    }

    /// Read and validate a manifest from a directory.
    ///
    /// Returns `None` if the file is absent, unreadable, unparseable,
    /// or fails validation (wrong repo, wrong path, or future version).
    pub fn read_from_dir(dir: &Path) -> Option<Self> {
        let path = dir.join(WORKTREE_MANIFEST_FILENAME);
        let bytes = std::fs::read(&path).ok()?;
        let m: WorktreeManifest = serde_json::from_slice(&bytes).ok()?;
        if m.version > Self::CURRENT_VERSION {
            warn!(
                path = %path.display(),
                manifest_version = m.version,
                current_version = Self::CURRENT_VERSION,
                "worktree manifest version too new, skipping"
            );
            return None;
        }
        Some(m)
    }

    /// Check that the manifest's embedded paths match the expected
    /// repository and the actual directory on disk.
    pub fn validate(&self, expected_repo: &Path, dir_on_disk: &Path) -> bool {
        if self.repo_path != expected_repo.to_string_lossy() {
            warn!(
                manifest_repo = %self.repo_path,
                expected_repo = %expected_repo.display(),
                "worktree manifest repo mismatch"
            );
            return false;
        }
        if self.worktree_path != dir_on_disk.to_string_lossy() {
            warn!(
                manifest_path = %self.worktree_path,
                dir_on_disk = %dir_on_disk.display(),
                "worktree manifest path mismatch"
            );
            return false;
        }
        true
    }

    /// Convenience: read from `dir` and validate against `expected_repo`.
    pub fn read_valid(dir: &Path, expected_repo: &Path) -> Option<Self> {
        let m = Self::read_from_dir(dir)?;
        if m.validate(expected_repo, dir) {
            Some(m)
        } else {
            None
        }
    }
}

/// Directory-name prefix for compound-review worktrees.
///
/// Single source of truth referenced by:
/// - `compound.rs::run` when constructing `review-<uuid>` names.
/// - Layer 2 (`scope::WorktreeManager::sweep_stale`) when matching
///   stale entries on startup.
/// - Layer 3 (`scripts/adf-setup/adf-cleanup.sh`) for the operator
///   cleanup helper.
///
/// Changes here must be mirrored in the shell script.
pub const WORKTREE_REVIEW_PREFIX: &str = "review-";

/// Check if `prefix` is a proper path prefix of `path`.
/// Ensures "src/" matches "src/main.rs" but not "src-backup/".
pub(crate) fn is_path_prefix(prefix: &str, path: &str) -> bool {
    if prefix.is_empty() {
        return false;
    }
    path.starts_with(prefix)
        && (prefix.ends_with('/')
            || path.len() == prefix.len()
            || path.as_bytes().get(prefix.len()) == Some(&b'/'))
}

/// A single scope reservation tracking which agent owns which file patterns.
#[derive(Debug, Clone)]
pub struct ScopeReservation {
    /// Unique identifier for this reservation
    pub id: Uuid,
    /// Name of the agent that holds this reservation
    pub agent_name: String,
    /// File patterns (globs) covered by this reservation
    pub file_patterns: HashSet<String>,
    /// When the reservation was created
    pub created_at: Instant,
    /// Correlation ID linking related reservations (e.g., compound review)
    pub correlation_id: Uuid,
}

impl ScopeReservation {
    /// Create a new scope reservation.
    pub fn new(
        agent_name: impl Into<String>,
        file_patterns: HashSet<String>,
        correlation_id: Uuid,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            agent_name: agent_name.into(),
            file_patterns,
            created_at: Instant::now(),
            correlation_id,
        }
    }

    /// Check if this reservation's patterns overlap with another set of patterns.
    /// Simple string-based overlap check - patterns are considered overlapping
    /// if any pattern in this reservation is a prefix of or equals any pattern in the other set.
    pub fn overlaps(&self, other_patterns: &HashSet<String>) -> bool {
        for self_pattern in &self.file_patterns {
            for other_pattern in other_patterns {
                // Direct match
                if self_pattern == other_pattern {
                    return true;
                }
                // Prefix overlap: "src/" overlaps with "src/main.rs" but not "src-backup/"
                let self_prefix = self_pattern.trim_end_matches('*');
                let other_prefix = other_pattern.trim_end_matches('*');
                if is_path_prefix(self_prefix, other_pattern)
                    || is_path_prefix(other_prefix, self_pattern)
                {
                    return true;
                }
            }
        }
        false
    }
}

/// Registry for tracking file scope reservations by agents.
///
/// In exclusive mode (nightly loop Phase 2), overlapping patterns are rejected.
/// In non-exclusive mode (compound review), overlapping reads are permitted.
#[derive(Debug)]
pub struct ScopeRegistry {
    reservations: HashMap<Uuid, ScopeReservation>,
    exclusive: bool,
}

impl ScopeRegistry {
    /// Create a new scope registry.
    ///
    /// * `exclusive` - If true, rejects reservations with overlapping patterns.
    ///   If false, allows overlapping reservations.
    pub fn new(exclusive: bool) -> Self {
        Self {
            reservations: HashMap::new(),
            exclusive,
        }
    }

    /// Attempt to reserve a scope for an agent.
    ///
    /// Returns the reservation ID on success, or an error message if the reservation
    /// cannot be made (e.g., overlapping patterns in exclusive mode).
    pub fn reserve(
        &mut self,
        agent_name: &str,
        file_patterns: HashSet<String>,
        correlation_id: Uuid,
    ) -> Result<Uuid, String> {
        if self.exclusive {
            // Check for overlapping patterns in exclusive mode
            for reservation in self.reservations.values() {
                if reservation.overlaps(&file_patterns) {
                    return Err(format!(
                        "Pattern overlap detected with existing reservation {} owned by {}",
                        reservation.id, reservation.agent_name
                    ));
                }
            }
        }

        let reservation = ScopeReservation::new(agent_name, file_patterns, correlation_id);
        let id = reservation.id;
        self.reservations.insert(id, reservation);

        debug!(
            reservation_id = %id,
            agent_name = %agent_name,
            correlation_id = %correlation_id,
            "scope reserved"
        );

        Ok(id)
    }

    /// Release a specific reservation by ID.
    ///
    /// Returns true if the reservation was found and removed, false otherwise.
    pub fn release(&mut self, reservation_id: Uuid) -> bool {
        let removed = self.reservations.remove(&reservation_id).is_some();
        if removed {
            debug!(reservation_id = %reservation_id, "scope released");
        }
        removed
    }

    /// Release all reservations associated with a correlation ID.
    ///
    /// Returns the number of reservations removed.
    pub fn release_by_correlation(&mut self, correlation_id: Uuid) -> usize {
        let to_remove: Vec<Uuid> = self
            .reservations
            .values()
            .filter(|r| r.correlation_id == correlation_id)
            .map(|r| r.id)
            .collect();

        let count = to_remove.len();
        for id in to_remove {
            self.reservations.remove(&id);
        }

        if count > 0 {
            debug!(correlation_id = %correlation_id, count = count, "scopes released by correlation");
        }

        count
    }

    /// Get all active reservations.
    pub fn active_reservations(&self) -> Vec<&ScopeReservation> {
        self.reservations.values().collect()
    }

    /// Check if an agent has any active reservations.
    pub fn has_reservation(&self, agent_name: &str) -> bool {
        self.reservations
            .values()
            .any(|r| r.agent_name == agent_name)
    }

    /// Get reservations for a specific agent.
    pub fn reservations_for_agent(&self, agent_name: &str) -> Vec<&ScopeReservation> {
        self.reservations
            .values()
            .filter(|r| r.agent_name == agent_name)
            .collect()
    }

    /// Check if the registry is in exclusive mode.
    pub fn is_exclusive(&self) -> bool {
        self.exclusive
    }

    /// Get the number of active reservations.
    pub fn len(&self) -> usize {
        self.reservations.len()
    }

    /// Check if there are no active reservations.
    pub fn is_empty(&self) -> bool {
        self.reservations.is_empty()
    }
}

/// Manages git worktrees for isolated agent workspaces.
///
/// Worktrees allow agents to work on different branches/refs without
/// interfering with the main working directory.
#[derive(Debug, Clone)]
pub struct WorktreeManager {
    repo_path: PathBuf,
    worktree_base: PathBuf,
}

impl WorktreeManager {
    /// Create a new worktree manager for a git repository.
    ///
    /// Worktrees will be created under `<repo>/.worktrees/<name>`.
    pub fn new(repo_path: impl AsRef<Path>) -> Self {
        let repo_path = repo_path.as_ref().to_path_buf();
        let worktree_base = repo_path.join(".worktrees");

        Self {
            repo_path,
            worktree_base,
        }
    }

    /// Create a worktree manager with a custom base directory.
    ///
    /// Worktrees will be created under `<worktree_base>/<name>`.
    pub fn with_base(repo_path: impl AsRef<Path>, worktree_base: impl AsRef<Path>) -> Self {
        let repo = repo_path.as_ref().to_path_buf();
        let base = worktree_base.as_ref().to_path_buf();
        // Resolve relative worktree_base against repo_path to avoid CWD-dependent behaviour
        let resolved_base = if base.is_relative() {
            repo.join(&base)
        } else {
            base
        };
        Self {
            repo_path: repo,
            worktree_base: resolved_base,
        }
    }

    /// Get the base path where worktrees are created.
    pub fn worktree_base(&self) -> &Path {
        &self.worktree_base
    }

    /// Get the repository path.
    pub fn repo_path(&self) -> &Path {
        &self.repo_path
    }

    /// Create a new worktree.
    ///
    /// * `name` - Name of the worktree (used as directory name)
    /// * `git_ref` - Git reference (branch, tag, commit) to check out
    ///
    /// Returns a `WorktreeGuard` that owns cleanup of the worktree.
    /// When the guard is dropped without `.keep()` being called, it
    /// invokes `git worktree remove --force` against the repository
    /// (reconciling the `<repo>/.git/worktrees/<name>` admin entry)
    /// and falls back to filesystem removal on failure.
    pub async fn create_worktree(
        &self,
        name: &str,
        git_ref: &str,
    ) -> Result<WorktreeGuard, std::io::Error> {
        let worktree_path = self.worktree_base.join(name);

        // Create parent directory if needed
        if let Some(parent) = worktree_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        info!(
            repo_path = %self.repo_path.display(),
            worktree_path = %worktree_path.display(),
            git_ref = %git_ref,
            "creating git worktree"
        );

        let mut command = tokio::process::Command::new("git");
        clear_git_env(&mut command);
        let output = command
            .arg("-C")
            .arg(&self.repo_path)
            .arg("worktree")
            .arg("add")
            .arg(&worktree_path)
            .arg(git_ref)
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!(name = %name, stderr = %stderr, "git worktree add failed");
            return Err(std::io::Error::other(format!(
                "Failed to create worktree '{}': {}",
                name, stderr
            )));
        }

        info!(name = %name, path = %worktree_path.display(), "worktree created");

        // Write ownership manifest so sweep can safely identify this
        // worktree as ADF-managed.  Best-effort; a missing or invalid
        // manifest inhibits cleanup rather than breaking the worktree.
        let manifest = WorktreeManifest {
            version: WorktreeManifest::CURRENT_VERSION,
            repo_path: self.repo_path.to_string_lossy().to_string(),
            worktree_path: worktree_path.to_string_lossy().to_string(),
            creator: "orchestrator".to_string(),
            session_id: name.to_string(),
            pid: std::process::id(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        if let Err(e) = manifest.write_to_dir(&worktree_path) {
            warn!(
                path = %worktree_path.display(),
                error = %e,
                "failed to write worktree manifest; cleanup will skip this entry"
            );
        }

        Ok(WorktreeGuard::for_managed(&self.repo_path, worktree_path))
    }

    /// Remove a worktree.
    ///
    /// * `name` - Name of the worktree to remove
    pub async fn remove_worktree(&self, name: &str) -> Result<(), std::io::Error> {
        let worktree_path = self.worktree_base.join(name);

        if !worktree_path.exists() {
            warn!(name = %name, path = %worktree_path.display(), "worktree does not exist");
            return Ok(());
        }

        info!(name = %name, "removing git worktree");

        let mut command = tokio::process::Command::new("git");
        clear_git_env(&mut command);
        let output = command
            .arg("-C")
            .arg(&self.repo_path)
            .arg("worktree")
            .arg("remove")
            .arg(&worktree_path)
            .output()
            .await?;

        if !output.status.success() {
            // Try force removal if normal removal fails
            let mut command = tokio::process::Command::new("git");
            clear_git_env(&mut command);
            let output = command
                .arg("-C")
                .arg(&self.repo_path)
                .arg("worktree")
                .arg("remove")
                .arg("--force")
                .arg(&worktree_path)
                .output()
                .await?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                error!(name = %name, stderr = %stderr, "git worktree remove failed");
                return Err(std::io::Error::other(format!(
                    "Failed to remove worktree '{}': {}",
                    name, stderr
                )));
            }
        }

        // Clean up empty parent directories
        if let Some(parent) = worktree_path.parent() {
            let _ = tokio::fs::remove_dir(parent).await;
        }

        info!(name = %name, "worktree removed");
        Ok(())
    }

    /// Remove all worktrees managed by this manager.
    ///
    /// Returns the number of worktrees removed.
    pub async fn cleanup_all(&self) -> Result<usize, std::io::Error> {
        let worktrees = self.list_worktrees()?;
        let mut count = 0;

        for name in &worktrees {
            if let Err(e) = self.remove_worktree(name).await {
                error!(name = %name, error = %e, "failed to remove worktree during cleanup");
            } else {
                count += 1;
            }
        }

        info!(count = count, "cleaned up all worktrees");
        Ok(count)
    }

    /// List all worktrees managed by this manager.
    ///
    /// Returns a list of worktree names (directory names, not full paths).
    pub fn list_worktrees(&self) -> Result<Vec<String>, std::io::Error> {
        if !self.worktree_base.exists() {
            return Ok(Vec::new());
        }

        let mut worktrees = Vec::new();

        for entry in std::fs::read_dir(&self.worktree_base)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                // Verify this is actually a git worktree by checking for .git file or directory
                if path.join(".git").exists() {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        worktrees.push(name.to_string());
                    }
                }
            }
        }

        Ok(worktrees)
    }

    /// Check if a worktree exists.
    pub fn worktree_exists(&self, name: &str) -> bool {
        self.worktree_base.join(name).join(".git").exists()
    }

    /// Sweep stale worktree residue left by a previous orchestrator
    /// instance (SIGKILL, OOM, panic-across-runtime, host reboot
    /// mid-review).
    ///
    /// Synchronous on purpose: `AgentOrchestrator::new` is a sync
    /// constructor and the sweep must complete before any tick thread
    /// is spawned. This is Layer 2 of the worktree lifecycle defence
    /// in depth (epic #1567):
    ///
    /// - Layer 1 (`WorktreeGuard::Drop`) handles the happy / cancelled
    ///   path while the process is still alive.
    /// - Layer 2 (this method) reconciles whatever survived the
    ///   previous process death.
    /// - Layer 3 (`scripts/adf-setup/adf-cleanup.sh` via
    ///   `ExecStartPre`) catches root-owned residue that the
    ///   orchestrator user cannot remove.
    ///
    /// Behaviour:
    /// 1. Walk `self.worktree_base` direct children whose name starts
    ///    with [`WORKTREE_REVIEW_PREFIX`] and attempt removal.
    /// 2. Walk every direct child of each `extra_roots` entry (no
    ///    prefix filter -- the per-agent `/tmp/adf-worktrees` root
    ///    convention has no prefix).
    /// 3. Removal tries `git worktree remove --force` first so the
    ///    `<repo>/.git/worktrees/<name>` admin entry is reconciled,
    ///    then falls back to `std::fs::remove_dir_all` on non-zero
    ///    exit.
    /// 4. `io::ErrorKind::PermissionDenied` (EACCES/EPERM on Linux)
    ///    increments `root_owned_skipped` rather than `failed_count`.
    ///    Layer 3 will catch those on the next service restart.
    /// 5. After walking, runs `git worktree prune --verbose` to drop
    ///    dead admin entries whose paths no longer exist on disk.
    ///
    /// Emits a single structured `info!` line with all
    /// [`SweepReport`] fields plus a synthetic
    /// `backlog_count = swept_count + root_owned_skipped`, so Quickwit
    /// dashboards can alert on residue backlog without parsing
    /// individual `warn!` lines.
    pub fn sweep_stale(&self, extra_roots: &[PathBuf]) -> SweepReport {
        let start = std::time::Instant::now();
        let mut report = SweepReport::default();

        // 1. Primary base: only review-prefixed entries.
        if self.worktree_base.is_dir() {
            match std::fs::read_dir(&self.worktree_base) {
                Ok(entries) => {
                    for entry in entries.flatten() {
                        let name = match entry.file_name().into_string() {
                            Ok(n) => n,
                            Err(_) => continue,
                        };
                        if !name.starts_with(WORKTREE_REVIEW_PREFIX) {
                            continue;
                        }
                        self.sweep_one(&entry.path(), &mut report);
                    }
                }
                Err(e) => {
                    warn!(
                        path = %self.worktree_base.display(),
                        error = %e,
                        "sweep_stale could not enumerate worktree base"
                    );
                }
            }
        }

        // 2. Extra roots (typically `/tmp/adf-worktrees`): every
        //    direct child, regardless of prefix.
        for root in extra_roots {
            if !root.is_dir() {
                continue;
            }
            match std::fs::read_dir(root) {
                Ok(entries) => {
                    for entry in entries.flatten() {
                        self.sweep_one(&entry.path(), &mut report);
                    }
                }
                Err(e) => {
                    warn!(
                        path = %root.display(),
                        error = %e,
                        "sweep_stale could not enumerate extra root"
                    );
                }
            }
        }

        // 3. Reconcile git's admin registry so half-killed worktree
        //    metadata under `<repo>/.git/worktrees/` is dropped.
        let mut command = std::process::Command::new("git");
        clear_std_git_env(&mut command);
        let prune = command
            .arg("-C")
            .arg(&self.repo_path)
            .arg("worktree")
            .arg("prune")
            .arg("--verbose")
            .output();
        report.prune_succeeded = matches!(&prune, Ok(o) if o.status.success());
        if let Ok(out) = prune {
            if !out.status.success() {
                let stderr = String::from_utf8_lossy(&out.stderr);
                warn!(stderr = %stderr, "git worktree prune failed during sweep");
            }
        } else if let Err(e) = prune {
            warn!(error = %e, "git worktree prune could not be invoked during sweep");
        }

        report.duration_ms = start.elapsed().as_millis() as u64;
        info!(
            swept_count = report.swept_count,
            failed_count = report.failed_count,
            root_owned_skipped = report.root_owned_skipped,
            no_manifest_skipped = report.no_manifest_skipped,
            prune_succeeded = report.prune_succeeded,
            duration_ms = report.duration_ms,
            backlog_count = report.swept_count + report.root_owned_skipped,
            "worktree sweep_stale complete"
        );
        report
    }

    /// Remove one worktree path, updating `report` in place.
    ///
    /// Before deletion, checks for a valid [`WorktreeManifest`]. If the
    /// manifest is absent, invalid, or mismatched, the directory is
    /// preserved regardless of naming convention.  This prevents
    /// accidental deletion of non-ADF directories that happen to share
    /// the `review-` prefix or live under `/tmp/adf-worktrees`.
    ///
    /// Tries `git worktree remove --force` first so the git admin
    /// registry stays in sync; falls back to `remove_dir_all` on
    /// non-zero exit. Permission-denied during the fallback path is
    /// counted as `root_owned_skipped` (Layer 3 territory) rather
    /// than a hard failure.
    fn sweep_one(&self, path: &Path, report: &mut SweepReport) {
        // Reject entries that do not carry a valid manifest.
        let manifest = match WorktreeManifest::read_valid(path, &self.repo_path) {
            Some(m) => m,
            None => {
                warn!(
                    path = %path.display(),
                    "sweep_stale skipping directory without valid ADF manifest"
                );
                report.no_manifest_skipped += 1;
                return;
            }
        };
        debug!(
            path = %path.display(),
            creator = %manifest.creator,
            session_id = %manifest.session_id,
            "sweep_stale found valid manifest, proceeding with removal"
        );
        let mut command = std::process::Command::new("git");
        clear_std_git_env(&mut command);
        let status = command
            .arg("-C")
            .arg(&self.repo_path)
            .arg("worktree")
            .arg("remove")
            .arg("--force")
            .arg(path)
            .status();

        if matches!(&status, Ok(s) if s.success()) {
            // Git removed both the worktree directory and the admin
            // entry. Some git versions leave an empty directory if the
            // worktree was already corrupt; tidy that up best-effort.
            if path.exists() {
                let _ = std::fs::remove_dir_all(path);
            }
            report.swept_count += 1;
            return;
        }

        // Git refused (or the path was never a registered worktree to
        // begin with -- common for residue under `/tmp/adf-worktrees`).
        // Fall back to a plain directory removal.
        match std::fs::remove_dir_all(path) {
            Ok(_) => report.swept_count += 1,
            Err(e) if matches!(e.kind(), std::io::ErrorKind::PermissionDenied) => {
                warn!(
                    path = %path.display(),
                    "sweep_stale skipping root-owned worktree -- Layer 3 will clean"
                );
                report.root_owned_skipped += 1;
            }
            Err(e) if matches!(e.kind(), std::io::ErrorKind::NotFound) => {
                // The path vanished between read_dir() and remove. Not
                // an error worth counting -- something else (Layer 3,
                // a sibling sweep) already handled it.
            }
            Err(e) => {
                warn!(
                    path = %path.display(),
                    error = %e,
                    "sweep_stale failed to remove worktree residue"
                );
                report.failed_count += 1;
            }
        }
    }
}

/// Summary of a [`WorktreeManager::sweep_stale`] invocation.
///
/// Emitted via structured `tracing::info!` so Quickwit can compute
/// backlog gauges (`swept_count + root_owned_skipped`) and alert on
/// large residue indicative of a prior crash storm.
#[derive(Debug, Clone, Default)]
pub struct SweepReport {
    /// Number of worktree directories successfully removed (either
    /// via `git worktree remove --force` or filesystem fallback).
    pub swept_count: usize,
    /// Number of removal attempts that returned a non-PermissionDenied
    /// error. Indicates filesystem-level problems worth investigating.
    pub failed_count: usize,
    /// Number of entries skipped because the orchestrator user lacked
    /// permission. These belong to Layer 3 (`adf-cleanup.sh` run as
    /// root via `ExecStartPre`).
    pub root_owned_skipped: usize,
    /// Number of entries skipped because no valid ADF worktree
    /// manifest was found in the directory.  Protects non-ADF data
    /// from accidental deletion.
    pub no_manifest_skipped: usize,
    /// Whether `git worktree prune --verbose` exited zero. False
    /// implies the git admin registry under `<repo>/.git/worktrees`
    /// may still hold dangling entries; the next sweep will retry.
    pub prune_succeeded: bool,
    /// Wall-clock duration of the sweep in milliseconds, for
    /// observability and startup-time budgeting.
    pub duration_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::process::Command;
    use tempfile::TempDir;

    // ==================== ScopeRegistry Tests ====================

    #[test]
    fn test_reserve_and_release() {
        let mut registry = ScopeRegistry::new(true);
        let correlation_id = Uuid::new_v4();
        let patterns: HashSet<String> = ["src/".to_string(), "tests/".to_string()].into();

        let id = registry
            .reserve("agent1", patterns.clone(), correlation_id)
            .expect("should reserve");

        assert!(registry.has_reservation("agent1"));
        assert!(!registry.has_reservation("agent2"));
        assert_eq!(registry.len(), 1);

        let released = registry.release(id);
        assert!(released);
        assert!(!registry.has_reservation("agent1"));
        assert_eq!(registry.len(), 0);

        // Release again should return false
        assert!(!registry.release(id));
    }

    #[test]
    fn test_reserve_exclusive_conflict() {
        let mut registry = ScopeRegistry::new(true); // exclusive mode
        let correlation_id = Uuid::new_v4();

        let patterns1: HashSet<String> = ["src/".to_string()].into();
        registry
            .reserve("agent1", patterns1, correlation_id)
            .expect("first reserve should succeed");

        // Overlapping pattern should fail in exclusive mode
        let patterns2: HashSet<String> = ["src/main.rs".to_string()].into();
        let result = registry.reserve("agent2", patterns2, correlation_id);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("overlap"));
    }

    #[test]
    fn test_reserve_non_exclusive_overlap_allowed() {
        let mut registry = ScopeRegistry::new(false); // non-exclusive mode
        let correlation_id = Uuid::new_v4();

        let patterns1: HashSet<String> = ["src/".to_string()].into();
        registry
            .reserve("agent1", patterns1, correlation_id)
            .expect("first reserve should succeed");

        // Overlapping pattern should succeed in non-exclusive mode
        let patterns2: HashSet<String> = ["src/main.rs".to_string()].into();
        let result = registry.reserve("agent2", patterns2, correlation_id);
        assert!(result.is_ok());
        assert_eq!(registry.len(), 2);
    }

    #[test]
    fn test_release_by_correlation() {
        let mut registry = ScopeRegistry::new(true);
        let correlation_id1 = Uuid::new_v4();
        let correlation_id2 = Uuid::new_v4();

        let patterns1: HashSet<String> = ["src/".to_string()].into();
        let patterns2: HashSet<String> = ["tests/".to_string()].into();
        let patterns3: HashSet<String> = ["docs/".to_string()].into();

        registry
            .reserve("agent1", patterns1, correlation_id1)
            .unwrap();
        registry
            .reserve("agent2", patterns2, correlation_id1)
            .unwrap();
        registry
            .reserve("agent3", patterns3, correlation_id2)
            .unwrap();

        assert_eq!(registry.len(), 3);

        let released = registry.release_by_correlation(correlation_id1);
        assert_eq!(released, 2);
        assert_eq!(registry.len(), 1);
        assert!(!registry.has_reservation("agent1"));
        assert!(!registry.has_reservation("agent2"));
        assert!(registry.has_reservation("agent3"));
    }

    #[test]
    fn test_active_reservations() {
        let mut registry = ScopeRegistry::new(true);
        let correlation_id = Uuid::new_v4();

        let patterns1: HashSet<String> = ["src/".to_string()].into();
        let patterns2: HashSet<String> = ["tests/".to_string()].into();

        registry
            .reserve("agent1", patterns1, correlation_id)
            .unwrap();
        registry
            .reserve("agent2", patterns2, correlation_id)
            .unwrap();

        let active = registry.active_reservations();
        assert_eq!(active.len(), 2);

        let agent_names: Vec<&str> = active.iter().map(|r| r.agent_name.as_str()).collect();
        assert!(agent_names.contains(&"agent1"));
        assert!(agent_names.contains(&"agent2"));
    }

    #[test]
    fn test_has_reservation() {
        let mut registry = ScopeRegistry::new(true);
        let correlation_id = Uuid::new_v4();

        assert!(!registry.has_reservation("agent1"));

        let patterns: HashSet<String> = ["src/".to_string()].into();
        registry
            .reserve("agent1", patterns, correlation_id)
            .unwrap();

        assert!(registry.has_reservation("agent1"));
        assert!(!registry.has_reservation("agent2"));
    }

    #[test]
    fn test_reservations_for_agent() {
        let mut registry = ScopeRegistry::new(true);
        let correlation_id = Uuid::new_v4();

        let patterns1: HashSet<String> = ["src/".to_string()].into();
        let patterns2: HashSet<String> = ["lib/".to_string()].into();

        registry
            .reserve("agent1", patterns1, correlation_id)
            .unwrap();
        registry
            .reserve("agent1", patterns2, correlation_id)
            .unwrap();
        registry
            .reserve("agent2", ["tests/".to_string()].into(), correlation_id)
            .unwrap();

        let agent1_reservations = registry.reservations_for_agent("agent1");
        assert_eq!(agent1_reservations.len(), 2);

        let agent2_reservations = registry.reservations_for_agent("agent2");
        assert_eq!(agent2_reservations.len(), 1);

        let agent3_reservations = registry.reservations_for_agent("agent3");
        assert!(agent3_reservations.is_empty());
    }

    #[test]
    fn test_reservation_overlap_detection() {
        let res1 = ScopeReservation::new("agent1", ["src/".to_string()].into(), Uuid::new_v4());

        // Exact overlap
        assert!(res1.overlaps(&["src/".to_string()].into()));

        // Sub-path overlap
        assert!(res1.overlaps(&["src/main.rs".to_string()].into()));

        // No overlap
        assert!(!res1.overlaps(&["tests/".to_string()].into()));

        // Sibling overlap check
        let res2 =
            ScopeReservation::new("agent2", ["src/main.rs".to_string()].into(), Uuid::new_v4());
        assert!(res2.overlaps(&["src/".to_string()].into()));
    }

    #[test]
    fn test_exclusive_mode_rejects_exact_match() {
        let mut registry = ScopeRegistry::new(true);
        let correlation_id = Uuid::new_v4();

        let patterns: HashSet<String> = ["src/main.rs".to_string()].into();
        registry
            .reserve("agent1", patterns.clone(), correlation_id)
            .unwrap();

        // Exact same pattern should fail
        let result = registry.reserve("agent2", patterns, correlation_id);
        assert!(result.is_err());
    }

    // ==================== WorktreeManager Tests ====================

    fn isolated_git() -> Command {
        let mut command = Command::new("git");
        for var in INHERITED_GIT_ENV {
            command.env_remove(var);
        }
        command
    }

    fn setup_git_repo() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().expect("failed to create temp dir");
        let repo_path = temp_dir.path().to_path_buf();

        // Initialize git repo
        let output = isolated_git()
            .arg("init")
            .arg(&repo_path)
            .output()
            .expect("failed to run git init");
        assert!(output.status.success(), "git init failed");

        // Configure git user for commits
        isolated_git()
            .arg("-C")
            .arg(&repo_path)
            .arg("config")
            .arg("user.email")
            .arg("test@test.com")
            .output()
            .expect("failed to config git email");

        isolated_git()
            .arg("-C")
            .arg(&repo_path)
            .arg("config")
            .arg("user.name")
            .arg("Test User")
            .output()
            .expect("failed to config git name");

        // Create initial commit
        std::fs::write(repo_path.join("README.md"), "# Test Repo").expect("failed to write file");

        isolated_git()
            .arg("-C")
            .arg(&repo_path)
            .arg("add")
            .arg(".")
            .output()
            .expect("failed to git add");

        isolated_git()
            .arg("-C")
            .arg(&repo_path)
            .arg("commit")
            .arg("-m")
            .arg("Initial commit")
            .output()
            .expect("failed to git commit");

        (temp_dir, repo_path)
    }

    /// Write a valid ADF worktree manifest into `dir` for testing.
    /// Used by sweep tests to ensure test directories are recognised
    /// as ADF-managed worktrees.
    fn write_test_manifest(dir: &Path, repo_path: &Path) {
        let manifest = WorktreeManifest {
            version: WorktreeManifest::CURRENT_VERSION,
            repo_path: repo_path.to_string_lossy().to_string(),
            worktree_path: dir.to_string_lossy().to_string(),
            creator: "test".to_string(),
            session_id: "test-session".to_string(),
            pid: std::process::id(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        manifest.write_to_dir(dir).expect("write test manifest");
    }

    #[tokio::test]
    async fn test_create_worktree() {
        let (_temp_dir, repo_path) = setup_git_repo();
        let manager = WorktreeManager::new(&repo_path);

        let guard_result = manager.create_worktree("feature-branch", "HEAD").await;
        assert!(
            guard_result.is_ok(),
            "create_worktree failed: {:?}",
            guard_result.err()
        );

        let guard = guard_result.unwrap();
        let path = guard.path().to_path_buf();
        assert!(path.exists());
        assert!(path.join(".git").exists());
        assert!(path.join("README.md").exists());
        // Disarm so guard's Drop doesn't fight the temp_dir teardown.
        guard.keep();
    }

    #[tokio::test]
    async fn test_remove_worktree() {
        let (_temp_dir, repo_path) = setup_git_repo();
        let manager = WorktreeManager::new(&repo_path);

        // Create worktree; keep the guard so the manual remove path
        // below is what removes it (not the guard's Drop).
        let guard = manager.create_worktree("to-remove", "HEAD").await.unwrap();
        let path = manager.worktree_base().join("to-remove");
        assert!(path.exists());
        guard.keep();

        // Remove worktree
        let result = manager.remove_worktree("to-remove").await;
        assert!(result.is_ok(), "remove_worktree failed: {:?}", result.err());
        assert!(!path.exists());
    }

    #[tokio::test]
    async fn test_remove_nonexistent_worktree() {
        let (_temp_dir, repo_path) = setup_git_repo();
        let manager = WorktreeManager::new(&repo_path);

        // Should succeed (no-op) for non-existent worktree
        let result = manager.remove_worktree("nonexistent").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_cleanup_all() {
        let (_temp_dir, repo_path) = setup_git_repo();
        let manager = WorktreeManager::new(&repo_path);

        // Create multiple worktrees; disarm each guard so cleanup_all
        // is what removes them (this test asserts the manager's
        // own bulk path, not the guard's Drop).
        manager.create_worktree("wt1", "HEAD").await.unwrap().keep();
        manager.create_worktree("wt2", "HEAD").await.unwrap().keep();
        manager.create_worktree("wt3", "HEAD").await.unwrap().keep();

        let worktrees = manager.list_worktrees().unwrap();
        assert_eq!(worktrees.len(), 3);

        // Cleanup all
        let cleaned = manager.cleanup_all().await.unwrap();
        assert_eq!(cleaned, 3);

        let worktrees = manager.list_worktrees().unwrap();
        assert!(worktrees.is_empty());
    }

    #[tokio::test]
    async fn test_list_worktrees() {
        let (_temp_dir, repo_path) = setup_git_repo();
        let manager = WorktreeManager::new(&repo_path);

        // Empty initially
        let worktrees = manager.list_worktrees().unwrap();
        assert!(worktrees.is_empty());

        // Create worktrees; keep the guards so the worktrees survive
        // long enough for list_worktrees to enumerate them.
        let _g_a = manager.create_worktree("wt-a", "HEAD").await.unwrap();
        let _g_b = manager.create_worktree("wt-b", "HEAD").await.unwrap();
        // Disarm so guard Drop doesn't race the TempDir teardown.
        _g_a.keep();
        _g_b.keep();

        let worktrees = manager.list_worktrees().unwrap();
        assert_eq!(worktrees.len(), 2);
        assert!(worktrees.contains(&"wt-a".to_string()));
        assert!(worktrees.contains(&"wt-b".to_string()));
    }

    #[tokio::test]
    async fn test_worktree_exists() {
        let (_temp_dir, repo_path) = setup_git_repo();
        let manager = WorktreeManager::new(&repo_path);

        assert!(!manager.worktree_exists("test-wt"));

        let guard = manager.create_worktree("test-wt", "HEAD").await.unwrap();
        assert!(manager.worktree_exists("test-wt"));
        guard.keep();

        manager.remove_worktree("test-wt").await.unwrap();
        assert!(!manager.worktree_exists("test-wt"));
    }

    #[test]
    fn test_worktree_paths() {
        let (_temp_dir, repo_path) = setup_git_repo();
        let manager = WorktreeManager::new(&repo_path);

        assert_eq!(manager.repo_path(), repo_path);
        assert_eq!(manager.worktree_base(), repo_path.join(".worktrees"));
    }

    #[tokio::test]
    async fn test_create_duplicate_worktree_fails() {
        let (_temp_dir, repo_path) = setup_git_repo();
        let manager = WorktreeManager::new(&repo_path);

        let guard = manager.create_worktree("duplicate", "HEAD").await.unwrap();

        // Creating duplicate should fail
        let result = manager.create_worktree("duplicate", "HEAD").await;
        assert!(result.is_err());
        guard.keep();
    }

    // ==================== sweep_stale Tests (Layer 2 / #1570) ====================

    #[test]
    fn test_sweep_stale_empty_dir() {
        // Base dir exists but is empty. Report should be all zeros and
        // prune should still succeed against a clean repo.
        let (_temp_dir, repo_path) = setup_git_repo();
        let base = repo_path.join(".worktrees");
        std::fs::create_dir_all(&base).expect("create empty base");
        let manager = WorktreeManager::with_base(&repo_path, &base);

        let report = manager.sweep_stale(&[]);
        assert_eq!(report.swept_count, 0);
        assert_eq!(report.failed_count, 0);
        assert_eq!(report.root_owned_skipped, 0);
        assert!(report.prune_succeeded, "prune should succeed on clean repo");
    }

    #[test]
    fn test_sweep_stale_no_base() {
        // Base dir does not exist; method must return successfully.
        let (_temp_dir, repo_path) = setup_git_repo();
        let manager =
            WorktreeManager::with_base(&repo_path, repo_path.join("does-not-exist-anywhere"));

        let report = manager.sweep_stale(&[]);
        assert_eq!(report.swept_count, 0);
        assert_eq!(report.failed_count, 0);
        assert_eq!(report.root_owned_skipped, 0);
        assert!(report.prune_succeeded);
    }

    #[test]
    fn test_sweep_stale_removes_review_prefix() {
        // Seed three `review-*` directories as plain dirs (not real
        // worktrees -- sweep_one falls back to remove_dir_all when git
        // refuses an unregistered path, which is exactly the
        // residue-after-SIGKILL shape).  Each must carry a valid
        // manifest or the sweep will preserve it.
        let (_temp_dir, repo_path) = setup_git_repo();
        let base = repo_path.join(".worktrees");
        std::fs::create_dir_all(&base).unwrap();

        for i in 0..3 {
            let dir = base.join(format!("{}{}", WORKTREE_REVIEW_PREFIX, i));
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("dummy.txt"), "residue").unwrap();
            write_test_manifest(&dir, &repo_path);
        }

        let manager = WorktreeManager::with_base(&repo_path, &base);
        let report = manager.sweep_stale(&[]);

        assert_eq!(report.swept_count, 3, "all three review dirs swept");
        assert_eq!(report.failed_count, 0);
        assert_eq!(report.root_owned_skipped, 0);
        assert_eq!(report.no_manifest_skipped, 0);

        for i in 0..3 {
            let dir = base.join(format!("{}{}", WORKTREE_REVIEW_PREFIX, i));
            assert!(!dir.exists(), "review-{} should be removed", i);
        }
    }

    #[test]
    fn test_sweep_stale_preserves_non_review_prefix() {
        // Only `review-` prefixed entries with valid manifests are
        // swept from worktree_base.  `keep-me` lacks BOTH a review
        // prefix AND a manifest, so it must survive.
        let (_temp_dir, repo_path) = setup_git_repo();
        let base = repo_path.join(".worktrees");
        std::fs::create_dir_all(&base).unwrap();

        let review_dir = base.join(format!("{}victim", WORKTREE_REVIEW_PREFIX));
        let keep_dir = base.join("keep-me");
        std::fs::create_dir_all(&review_dir).unwrap();
        std::fs::create_dir_all(&keep_dir).unwrap();
        std::fs::write(keep_dir.join("important.txt"), "data").unwrap();
        write_test_manifest(&review_dir, &repo_path);

        let manager = WorktreeManager::with_base(&repo_path, &base);
        let report = manager.sweep_stale(&[]);

        assert_eq!(report.swept_count, 1);
        assert!(!review_dir.exists(), "review-victim should be swept");
        assert!(keep_dir.exists(), "keep-me must be preserved");
        assert!(
            keep_dir.join("important.txt").exists(),
            "keep-me contents must survive"
        );
    }

    #[test]
    fn test_sweep_stale_runs_prune() {
        // `prune_succeeded` must be true after a clean sweep against a
        // healthy repo. This guards against silently regressing the
        // git registry reconciliation step.
        let (_temp_dir, repo_path) = setup_git_repo();
        let base = repo_path.join(".worktrees");
        std::fs::create_dir_all(&base).unwrap();
        let manager = WorktreeManager::with_base(&repo_path, &base);

        let report = manager.sweep_stale(&[]);
        assert!(
            report.prune_succeeded,
            "prune step must run and succeed after sweep"
        );
    }

    #[test]
    fn test_sweep_stale_extra_roots_no_prefix_filter() {
        // Entries under extra_roots are swept regardless of prefix --
        // the per-agent root convention has no naming convention.
        // Each must carry a valid manifest.
        let (_temp_dir, repo_path) = setup_git_repo();
        let base = repo_path.join(".worktrees");
        std::fs::create_dir_all(&base).unwrap();

        // Unique temp dir to mimic `/tmp/adf-worktrees` without
        // colliding with a parallel real orchestrator on this host.
        let extra = std::env::temp_dir().join(format!("adf-worktrees-test-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&extra).unwrap();
        let agent_dir = extra.join("agent-alpha");
        std::fs::create_dir_all(&agent_dir).unwrap();
        std::fs::write(agent_dir.join("scratch.txt"), "tmp").unwrap();
        write_test_manifest(&agent_dir, &repo_path);

        let manager = WorktreeManager::with_base(&repo_path, &base);
        let report = manager.sweep_stale(std::slice::from_ref(&extra));

        assert_eq!(report.swept_count, 1, "agent-alpha should be swept");
        assert!(!agent_dir.exists(), "extra-root child should be removed");

        // Tidy up the now-empty extra root.
        let _ = std::fs::remove_dir_all(&extra);
    }

    /// Root-owned residue belongs to Layer 3 (`adf-cleanup.sh` via
    /// `ExecStartPre`); Layer 2 must skip it gracefully without
    /// counting it as a hard failure.
    ///
    /// This test is Linux-only and only meaningful when the test
    /// process runs as root. In every other environment it is a no-op
    /// that nonetheless documents the contract (the assertion would
    /// be vacuously skipped).
    #[test]
    #[cfg_attr(not(target_os = "linux"), ignore)]
    fn test_sweep_stale_skips_root_owned() {
        // Runtime gate: skip when not root. Without elevated privilege
        // we cannot create a file the test user cannot delete, so the
        // PermissionDenied path is unreachable.
        let is_root = std::env::var("USER")
            .map(|u| u == "root")
            .unwrap_or(false)
            // SAFETY: getuid() is async-signal-safe and side-effect free.
            || unsafe { libc_getuid() } == 0;
        if !is_root {
            eprintln!("skipping test_sweep_stale_skips_root_owned: not root");
            return;
        }

        let (_temp_dir, repo_path) = setup_git_repo();
        let base = repo_path.join(".worktrees");
        std::fs::create_dir_all(&base).unwrap();

        let review_dir = base.join(format!("{}root-owned", WORKTREE_REVIEW_PREFIX));
        std::fs::create_dir_all(&review_dir).unwrap();

        // Make the parent dir read+exec but not writable for non-owner
        // so remove_dir_all from a dropped-privilege process would
        // EACCES. Real root-owned residue scenario: the file's owner
        // is root and the orchestrator runs as a service user.
        //
        // We cannot truly fake this from inside a root test process
        // (root bypasses DAC), so this test asserts only that the
        // method completes without panicking when handed a path it
        // cannot remove. The actual EACCES branch is exercised by
        // Layer 3 integration testing on the bigbox.
        let report = WorktreeManager::with_base(&repo_path, &base).sweep_stale(&[]);

        // Whether the dir was swept or marked root_owned_skipped
        // depends on the host's filesystem; both are acceptable. The
        // contract is just "no panic, no failed_count".
        assert_eq!(
            report.failed_count, 0,
            "root-owned residue must never count as a hard failure"
        );
    }

    // Direct FFI to `getuid(2)` so we do not pull `nix` or `libc`
    // crates just for one test. Linux only.
    #[cfg(target_os = "linux")]
    extern "C" {
        #[link_name = "getuid"]
        fn libc_getuid() -> u32;
    }

    #[cfg(not(target_os = "linux"))]
    #[allow(dead_code)]
    unsafe fn libc_getuid() -> u32 {
        // Non-Linux fallback that never claims root, so the runtime
        // gate above always skips. The `#[cfg_attr(ignore)]` on the
        // test means we will not reach this branch in practice.
        1
    }
}

/// Test helpers exposed at module scope (under the `test-helpers`
/// feature, or when compiling the lib's own test target) so
/// integration tests under `tests/` can re-use the git repo fixture.
/// These helpers have no production callers; they exist to share
/// setup code across `scope.rs::tests` and the cancellation property
/// test.
#[doc(hidden)]
#[cfg(any(test, feature = "test-helpers"))]
pub mod test_support {
    use std::path::PathBuf;
    use std::process::Command;
    use tempfile::TempDir;

    fn isolated_git() -> Command {
        let mut command = Command::new("git");
        for var in super::INHERITED_GIT_ENV {
            command.env_remove(var);
        }
        command
    }

    /// Initialise a real git repository in a fresh `TempDir` with one
    /// commit. Returns the `TempDir` (caller owns the lifetime) and
    /// the repo path.
    pub fn setup_git_repo() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().expect("failed to create temp dir");
        let repo_path = temp_dir.path().to_path_buf();

        let output = isolated_git()
            .arg("init")
            .arg(&repo_path)
            .output()
            .expect("failed to run git init");
        assert!(output.status.success(), "git init failed");

        isolated_git()
            .arg("-C")
            .arg(&repo_path)
            .arg("config")
            .arg("user.email")
            .arg("test@test.com")
            .output()
            .expect("failed to config git email");

        isolated_git()
            .arg("-C")
            .arg(&repo_path)
            .arg("config")
            .arg("user.name")
            .arg("Test User")
            .output()
            .expect("failed to config git name");

        std::fs::write(repo_path.join("README.md"), "# Test Repo").expect("failed to write file");

        isolated_git()
            .arg("-C")
            .arg(&repo_path)
            .arg("add")
            .arg(".")
            .output()
            .expect("failed to git add");

        isolated_git()
            .arg("-C")
            .arg(&repo_path)
            .arg("commit")
            .arg("-m")
            .arg("Initial commit")
            .output()
            .expect("failed to git commit");

        (temp_dir, repo_path)
    }
}
