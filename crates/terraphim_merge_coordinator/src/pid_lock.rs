//! File-based PID lock for merge-coordinator (Concurrency-1 per spec).
//!
//! Prevents concurrent invocations from racing on Gitea API calls.
//! Lock file lives at /tmp/merge-coordinator.lock; if another process
//! holds it for less than `stale_after_secs` (default 30 s), the new
//! invocation aborts with `LockHeld`. Beyond the stale threshold the
//! lock is forcibly stolen, defending against crashes that leak locks.

use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use fs2::FileExt;

use crate::types::MergeCoordinatorError;

/// RAII guard that releases the PID lock on drop.
#[derive(Debug)]
pub struct PidLockGuard {
    file: File,
}

impl Drop for PidLockGuard {
    fn drop(&mut self) {
        // Best-effort unlock; failure here is non-fatal (the file
        // will be released on process exit anyway).
        let _ = FileExt::unlock(&self.file);
    }
}

/// Acquire an exclusive PID lock at `path`. Returns `LockHeld` if a
/// non-stale lock exists (held within `stale_after_secs` seconds).
pub fn acquire_pid_lock(
    path: &Path,
    stale_after_secs: u64,
) -> Result<PidLockGuard, MergeCoordinatorError> {
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)?;

    match FileExt::try_lock_exclusive(&file) {
        Ok(()) => write_pid(&mut file)?,
        Err(_) => {
            // Lock is held; check whether the holder is stale.
            let mut buf = String::new();
            file.seek(SeekFrom::Start(0))?;
            file.read_to_string(&mut buf)?;
            let parsed = parse_lock_payload(&buf);

            let now = now_secs();
            let age = now.saturating_sub(parsed.acquired_at);
            if age < stale_after_secs {
                return Err(MergeCoordinatorError::LockHeld {
                    pid: parsed.pid,
                    age_secs: age,
                });
            }
            // Stale -- steal the lock.
            FileExt::lock_exclusive(&file)?;
            write_pid(&mut file)?;
        }
    }
    Ok(PidLockGuard { file })
}

#[derive(Debug, Clone, Copy)]
struct LockPayload {
    pid: i32,
    acquired_at: u64,
}

fn parse_lock_payload(s: &str) -> LockPayload {
    let mut parts = s.split_whitespace();
    let pid = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let acquired_at = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    LockPayload { pid, acquired_at }
}

fn write_pid(file: &mut File) -> std::io::Result<()> {
    let pid = std::process::id() as i32;
    let acquired_at = now_secs();
    file.set_len(0)?;
    file.seek(SeekFrom::Start(0))?;
    writeln!(file, "{pid} {acquired_at}")?;
    file.flush()?;
    Ok(())
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn acquire_creates_lock_file_and_writes_pid() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;
        let lock_path = dir.path().join("merge-coordinator.lock");
        let _guard = acquire_pid_lock(&lock_path, 30)?;
        let contents = std::fs::read_to_string(&lock_path)?;
        let payload = parse_lock_payload(&contents);
        assert!(
            payload.pid > 0,
            "pid should be written; got {}",
            payload.pid
        );
        assert!(payload.acquired_at > 0);
        Ok(())
    }

    #[test]
    fn second_acquire_within_stale_window_returns_lock_held()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;
        let lock_path = dir.path().join("merge-coordinator.lock");
        let _guard1 = acquire_pid_lock(&lock_path, 30)?;

        let Err(MergeCoordinatorError::LockHeld { pid, .. }) = acquire_pid_lock(&lock_path, 30)
        else {
            return Err("expected LockHeld".into());
        };
        assert!(pid > 0, "lock holder pid should be reported");
        Ok(())
    }

    #[test]
    fn second_acquire_after_stale_threshold_steals_lock() -> Result<(), Box<dyn std::error::Error>>
    {
        let dir = tempdir()?;
        let lock_path = dir.path().join("merge-coordinator.lock");
        // Write a stale lock payload (PID 999999 acquired 100s ago).
        let stale_ts = now_secs().saturating_sub(100);
        std::fs::write(&lock_path, format!("999999 {stale_ts}\n"))?;

        // 30s stale threshold; 100s elapsed -> steal succeeds.
        let _guard = acquire_pid_lock(&lock_path, 30)?;
        let contents = std::fs::read_to_string(&lock_path)?;
        let payload = parse_lock_payload(&contents);
        assert_ne!(payload.pid, 999999, "stale pid should be replaced");
        assert!(
            payload.acquired_at > stale_ts,
            "new acquired_at should be more recent than stale one"
        );
        Ok(())
    }
}
