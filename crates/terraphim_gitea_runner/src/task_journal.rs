//! Durable, redacted per-task log journal.
//!
//! The journal is the single copy of a task's captured output that survives a
//! transport failure or a runner restart. It sits between process capture and
//! `UpdateLog`: rows are masked **before** they reach this file, so neither disk
//! nor the network ever sees an unredacted line (design §3, Refs #101).
//!
//! # Layout
//!
//! ```text
//! <root>/<task_id>/
//!   meta.json      instance metadata + cursors (atomically replaced)
//!   rows.ndjson    redacted rows, one JSON object per line, append-only
//!   final.json     immutable terminal snapshot, written once
//! ```
//!
//! Cursors are kept distinct on purpose:
//!
//! * `produced` -- next row sequence durably appended (and fsynced) here;
//! * `acked`    -- next row the server confirmed it committed.
//!
//! A batch is only acknowledged after its rows have been synced, so a crash can
//! replay rows but never lose them. Directory mode is 0700 and file mode 0600:
//! retained evidence is private to the runner user by default.

use crate::types::LogRow;
use crate::{Result, RunnerError};
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Directory permission bits for the journal root and task directories.
const DIR_MODE: u32 = 0o700;
/// File permission bits for every journal artefact.
const FILE_MODE: u32 = 0o600;

/// Instance identity recorded alongside a task's rows, separate from secrets
/// (design §3: identity is needed to reconcile after restart, secrets are not).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct JournalMeta {
    /// Runner UUID that produced these rows.
    pub runner_uuid: String,
    /// Gitea task id.
    #[serde(default)]
    pub task_id: i64,
    /// Repository (`owner/repo`) when known.
    pub repository: Option<String>,
    /// Commit SHA under evaluation when known.
    pub sha: Option<String>,
    /// Next row sequence produced (durable upper bound).
    #[serde(default)]
    pub produced: i64,
    /// Next row the server expects.
    #[serde(default)]
    pub acked: i64,
    /// True once the log stream has been sealed by an accepted finalise.
    ///
    /// Sealing is stored as state rather than derived: a zero-row log answers
    /// every re-sent finalise with ack 0, so `acked == produced` proves nothing.
    #[serde(default)]
    pub sealed: bool,
}

/// Append-only durable store for one task's redacted rows and cursors.
pub struct TaskJournal {
    dir: PathBuf,
    meta: JournalMeta,
    rows_path: PathBuf,
}

impl TaskJournal {
    /// Create (or resume) the journal for `task_id` under `root`.
    ///
    /// Existing metadata is loaded when present so a fresh process continues
    /// from the persisted cursors instead of restarting at row 0.
    pub fn open(root: &Path, task_id: i64, meta: JournalMeta) -> Result<Self> {
        let dir = root.join(task_id.to_string());
        std::fs::create_dir_all(&dir).map_err(|e| {
            RunnerError::State(format!("cannot create journal dir {}: {e}", dir.display()))
        })?;
        set_mode(&dir, DIR_MODE)?;

        let rows_path = dir.join("rows.ndjson");
        let mut journal = Self {
            dir,
            meta,
            rows_path,
        };
        journal.meta.task_id = task_id;

        if let Some(existing) = journal.load_meta()? {
            // Resume: cursors win over whatever the caller passed in, because the
            // file reflects what was actually synced.
            journal.meta.produced = existing.produced;
            journal.meta.acked = existing.acked;
            journal.meta.sealed = existing.sealed;
            journal.meta.runner_uuid = existing.runner_uuid;
            journal.meta.repository = existing.repository.or(journal.meta.repository);
            journal.meta.sha = existing.sha.or(journal.meta.sha);
        } else {
            journal.write_meta()?;
        }
        Ok(journal)
    }

    fn meta_path(&self) -> PathBuf {
        self.dir.join("meta.json")
    }

    fn load_meta(&self) -> Result<Option<JournalMeta>> {
        let path = self.meta_path();
        if !path.exists() {
            return Ok(None);
        }
        let text = std::fs::read_to_string(&path)
            .map_err(|e| RunnerError::State(format!("cannot read {}: {e}", path.display())))?;
        let meta: JournalMeta = serde_json::from_str(&text)
            .map_err(|e| RunnerError::State(format!("corrupt {}: {e}", path.display())))?;
        Ok(Some(meta))
    }

    /// Atomically replace `meta.json`: write a sibling, fsync it, rename, then
    /// fsync the directory so the rename itself survives a crash.
    fn write_meta(&self) -> Result<()> {
        let tmp = self.dir.join("meta.json.tmp");
        let body = serde_json::to_vec(&self.meta)
            .map_err(|e| RunnerError::State(format!("cannot encode journal meta: {e}")))?;
        write_synced(&tmp, &body)?;
        std::fs::rename(&tmp, self.meta_path())
            .map_err(|e| RunnerError::State(format!("cannot replace meta.json: {e}")))?;
        sync_dir(&self.dir)
    }

    /// Current durable produced cursor.
    pub fn produced(&self) -> i64 {
        self.meta.produced
    }

    /// Current server-acknowledged cursor.
    pub fn acked(&self) -> i64 {
        self.meta.acked
    }

    /// Whether the stream has been sealed.
    pub fn is_sealed(&self) -> bool {
        self.meta.sealed
    }

    /// The journal directory (for retention / garbage collection).
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Append already-redacted rows, syncing them before advancing `produced`.
    ///
    /// Returns the half-open window `[start, end)` the rows occupy.
    pub fn append_redacted(&mut self, rows: &[LogRow]) -> Result<(i64, i64)> {
        if self.meta.sealed {
            return Err(RunnerError::Protocol(format!(
                "journal for task {} is sealed; refusing further rows",
                self.meta.task_id
            )));
        }
        let start = self.meta.produced;
        if !rows.is_empty() {
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.rows_path)
                .map_err(|e| {
                    RunnerError::State(format!("cannot open {}: {e}", self.rows_path.display()))
                })?;
            apply_file_mode(&file)?;
            for row in rows {
                let line = serde_json::to_string(row)
                    .map_err(|e| RunnerError::State(format!("cannot encode row: {e}")))?;
                writeln!(file, "{line}")
                    .map_err(|e| RunnerError::State(format!("cannot append row: {e}")))?;
            }
            // Rows must be durable *before* the cursor that claims them moves.
            file.flush()
                .map_err(|e| RunnerError::State(format!("cannot flush rows: {e}")))?;
            file.sync_all()
                .map_err(|e| RunnerError::State(format!("cannot sync rows: {e}")))?;
            drop(file);
            sync_dir(&self.dir)?;
        }
        self.meta.produced = start + rows.len() as i64;
        self.write_meta()?;
        Ok((start, self.meta.produced))
    }

    /// Read up to `max_rows` rows starting at `start`, staying within
    /// `max_bytes` of serialized content (at least one row is returned whenever
    /// `start` is in range, so an oversized single row still makes progress).
    pub fn batch_from(&self, start: i64, max_rows: usize, max_bytes: usize) -> Vec<LogRow> {
        let all = match self.read_rows() {
            Ok(rows) => rows,
            Err(_) => return Vec::new(),
        };
        if start < 0 || start as usize >= all.len() || max_rows == 0 {
            return Vec::new();
        }
        let mut out: Vec<LogRow> = Vec::with_capacity(max_rows.min(all.len() - start as usize));
        let mut bytes = 0usize;
        for row in all[start as usize..].iter().take(max_rows) {
            let cost = row.content.len() + row.time.len() + 32;
            if !out.is_empty() && bytes + cost > max_bytes {
                break;
            }
            bytes += cost;
            out.push(row.clone());
        }
        out
    }

    /// Persist the server-acknowledged cursor.
    pub fn record_ack(&mut self, next: i64) -> Result<()> {
        if next < self.meta.acked {
            return Err(RunnerError::Protocol(format!(
                "journal ack would regress from {} to {next}",
                self.meta.acked
            )));
        }
        if next > self.meta.produced {
            return Err(RunnerError::Protocol(format!(
                "journal ack {next} exceeds produced {}",
                self.meta.produced
            )));
        }
        self.meta.acked = next;
        self.write_meta()
    }

    /// Record that the log stream has been sealed.
    pub fn record_sealed(&mut self) -> Result<()> {
        self.meta.sealed = true;
        self.write_meta()
    }

    /// Store the immutable terminal snapshot for this task.
    pub fn record_final(&mut self, snapshot_json: &str) -> Result<()> {
        let path = self.dir.join("final.json");
        let mut body = snapshot_json.as_bytes().to_vec();
        if !body.ends_with(b"\n") {
            body.push(b'\n');
        }
        write_synced(&path, &body)?;
        sync_dir(&self.dir)
    }

    /// Read back the terminal snapshot, if one was recorded.
    pub fn final_snapshot(&self) -> Result<Option<String>> {
        let path = self.dir.join("final.json");
        if !path.exists() {
            return Ok(None);
        }
        let text = std::fs::read_to_string(&path)
            .map_err(|e| RunnerError::State(format!("cannot read {}: {e}", path.display())))?;
        Ok(Some(text))
    }

    /// All durable rows, in production order.
    pub fn read_rows(&self) -> Result<Vec<LogRow>> {
        if !self.rows_path.exists() {
            return Ok(Vec::new());
        }
        let text = std::fs::read_to_string(&self.rows_path).map_err(|e| {
            RunnerError::State(format!("cannot read {}: {e}", self.rows_path.display()))
        })?;
        let mut out = Vec::new();
        for line in text.lines() {
            if line.trim().is_empty() {
                continue;
            }
            out.push(
                serde_json::from_str(line).map_err(|e| {
                    RunnerError::State(format!("corrupt journal row {line:?}: {e}"))
                })?,
            );
        }
        Ok(out)
    }
}

/// Set permissions on a path we just created.
fn set_mode(path: &Path, mode: u32) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
        .map_err(|e| RunnerError::State(format!("cannot chmod {:o} {}: {e}", mode, path.display())))
}

/// Best-effort tightening of an open file's mode (already-created files included).
fn apply_file_mode(file: &File) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    file.set_permissions(std::fs::Permissions::from_mode(FILE_MODE))
        .map_err(|e| RunnerError::State(format!("cannot chmod journal file: {e}")))
}

/// Write `bytes` to `path` (truncating), fsync before returning so a later
/// rename never exposes a partially-written file.
fn write_synced(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)
        .map_err(|e| RunnerError::State(format!("cannot open {}: {e}", path.display())))?;
    apply_file_mode(&file)?;
    file.write_all(bytes)
        .map_err(|e| RunnerError::State(format!("cannot write {}: {e}", path.display())))?;
    file.sync_all()
        .map_err(|e| RunnerError::State(format!("cannot sync {}: {e}", path.display())))
}

/// fsync a directory so creates/renames inside it are durable.
fn sync_dir(path: &Path) -> Result<()> {
    File::open(path)
        .map_err(|e| RunnerError::State(format!("cannot open dir {}: {e}", path.display())))?
        .sync_all()
        .map_err(|e| RunnerError::State(format!("cannot sync dir {}: {e}", path.display())))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(content: &str) -> LogRow {
        LogRow {
            time: "2026-09-13T00:00:00Z".to_string(),
            content: content.to_string(),
        }
    }

    fn open_journal(root: &Path, task_id: i64) -> TaskJournal {
        TaskJournal::open(
            root,
            task_id,
            JournalMeta {
                runner_uuid: "uuid-1".to_string(),
                repository: Some("terraphim/proof".to_string()),
                sha: Some("abc123".to_string()),
                ..Default::default()
            },
        )
        .unwrap()
    }

    #[test]
    fn appends_return_windows_and_persist_cursors() {
        let tmp = tempfile::tempdir().unwrap();
        let mut j = open_journal(tmp.path(), 7);
        assert_eq!(j.append_redacted(&[row("a"), row("b")]).unwrap(), (0, 2));
        assert_eq!(j.produced(), 2);
        assert_eq!(j.append_redacted(&[row("c")]).unwrap(), (2, 3));
        assert_eq!(j.produced(), 3);
        assert_eq!(j.read_rows().unwrap().len(), 3);
    }

    #[test]
    fn cursors_survive_reopen_so_a_restart_can_replay() {
        let tmp = tempfile::tempdir().unwrap();
        {
            let mut j = open_journal(tmp.path(), 9);
            j.append_redacted(&[row("a"), row("b"), row("c")]).unwrap();
            j.record_ack(2).unwrap();
        }
        // Fresh process: same directory, same task.
        let reopened = open_journal(tmp.path(), 9);
        assert_eq!(reopened.produced(), 3);
        assert_eq!(reopened.acked(), 2, "un-acked tail is recoverable");
        let batch = reopened.batch_from(2, 256, 256 * 1024);
        assert_eq!(
            batch.iter().map(|r| r.content.as_str()).collect::<Vec<_>>(),
            vec!["c"],
            "exactly the rows the server never received"
        );
    }

    #[test]
    fn batch_from_respects_row_and_byte_budgets() {
        let tmp = tempfile::tempdir().unwrap();
        let mut j = open_journal(tmp.path(), 11);
        let rows: Vec<LogRow> = (0..10).map(|i| row(&format!("line {i}"))).collect();
        j.append_redacted(&rows).unwrap();

        assert_eq!(j.batch_from(0, 3, 1_000_000).len(), 3, "row budget");
        assert_eq!(j.batch_from(4, 3, 1_000_000)[0].content, "line 4");
        let small = j.batch_from(0, 100, 64);
        assert!(
            small.len() < 10,
            "byte budget bounds the batch: {}",
            small.len()
        );
        assert!(!small.is_empty(), "at least one row makes progress");
        assert!(j.batch_from(10, 5, 1_000_000).is_empty(), "past the end");
    }

    #[test]
    fn ack_cannot_regress_or_exceed_produced() {
        let tmp = tempfile::tempdir().unwrap();
        let mut j = open_journal(tmp.path(), 12);
        j.append_redacted(&[row("a"), row("b")]).unwrap();
        j.record_ack(2).unwrap();
        assert!(matches!(
            j.record_ack(1).unwrap_err(),
            RunnerError::Protocol(_)
        ));
        assert!(matches!(
            j.record_ack(9).unwrap_err(),
            RunnerError::Protocol(_)
        ));
        assert_eq!(j.acked(), 2, "rejected writes leave the cursor alone");
    }

    #[test]
    fn sealed_journal_refuses_further_rows() {
        let tmp = tempfile::tempdir().unwrap();
        let mut j = open_journal(tmp.path(), 13);
        j.append_redacted(&[row("a")]).unwrap();
        j.record_ack(1).unwrap();
        j.record_sealed().unwrap();
        assert!(j.is_sealed());
        assert!(matches!(
            j.append_redacted(&[row("b")]).unwrap_err(),
            RunnerError::Protocol(_)
        ));
        // Seal is durable across reopen: it cannot be re-derived from cursors.
        let reopened = open_journal(tmp.path(), 13);
        assert!(reopened.is_sealed());
    }

    #[test]
    fn final_snapshot_round_trips() {
        let tmp = tempfile::tempdir().unwrap();
        let mut j = open_journal(tmp.path(), 14);
        assert!(j.final_snapshot().unwrap().is_none());
        j.record_final(r#"{"result":"SUCCESS"}"#).unwrap();
        assert_eq!(
            j.final_snapshot().unwrap().unwrap().trim(),
            r#"{"result":"SUCCESS"}"#
        );
    }

    #[cfg(unix)]
    #[test]
    fn journal_files_are_private_to_the_owner() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let mut j = open_journal(tmp.path(), 15);
        j.append_redacted(&[row("secret-free")]).unwrap();
        j.record_final("{}").unwrap();

        let dir_mode = std::fs::metadata(j.dir()).unwrap().permissions().mode();
        assert_eq!(dir_mode & 0o777, DIR_MODE, "task dir is 0700");
        for name in ["meta.json", "rows.ndjson", "final.json"] {
            let m = std::fs::metadata(j.dir().join(name))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(m & 0o777, FILE_MODE, "{name} must be 0600, got {m:#o}");
        }
    }

    #[test]
    fn empty_append_still_advances_nothing() {
        let tmp = tempfile::tempdir().unwrap();
        let mut j = open_journal(tmp.path(), 16);
        assert_eq!(j.append_redacted(&[]).unwrap(), (0, 0));
        assert!(j.read_rows().unwrap().is_empty());
    }
}
