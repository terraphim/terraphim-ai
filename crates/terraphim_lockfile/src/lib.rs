//! Kernel-held cross-process singleton lease primitive (#3295).
//!
//! Shared byte-contract crate used by BOTH the orchestrator
//! (`terraphim_orchestrator`) and the standalone merge-coordinator binary:
//!
//! - lock file path: `<dir>/{key}.lock`
//! - payload (informational diagnostics only, never used for correctness):
//!   `pid=<pid> acquired=<unix_secs>`
//! - exclusive `flock(2)` semantics: non-blocking, single attempt; never
//!   stolen, never unlinked; recovery is exclusively kernel fd-close
//!   (process crash, `SIGKILL`, host reboot).
//!
//! The cross-repo payload/path/no-steal contract is FROZEN — do not add
//! ULID/instance fields and do not replace `fs4` (0.13.1).

/// Errors from acquiring a singleton lease.
///
/// Exactly two variants: `LockHeld` is the single *benign* skip (kernel
/// contention); every I/O or configuration problem is `Io` and fails closed.
#[derive(Debug, thiserror::Error)]
pub enum LeaseError {
    /// Another holder holds the kernel lease. `holder_pid` is read
    /// best-effort from the payload and is diagnostics-only.
    #[error("lease held elsewhere (holder pid: {holder_pid:?})")]
    LockHeld { holder_pid: Option<u32> },
    /// I/O or configuration failure. Callers must fail closed.
    #[error("lease I/O failure: {0}")]
    Io(#[from] std::io::Error),
}

/// Validate one path component of a lease key against
/// `^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$`.
///
/// This kills path traversal (`../`, `/`, `\`, NUL) and hostile/mojibake
/// components before any path is built. The reserved legacy id `__global__`
/// does NOT satisfy the charset and is rejected here; only
/// [`map_reserved_component`] turns it into the safe `global` component.
pub fn validate_key_component(component: &str) -> Result<(), String> {
    let mut chars = component.chars();
    let Some(first) = chars.next() else {
        return Err("key component must not be empty".to_string());
    };
    if !first.is_ascii_alphanumeric() {
        return Err(format!(
            "key component {component:?} must start with an ASCII alphanumeric character"
        ));
    }
    let len = component.chars().count();
    if !(1..=127).contains(&len) {
        return Err(format!(
            "key component {component:?} must be 1..=127 characters, got {len}"
        ));
    }
    if !component
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_')
    {
        return Err(format!(
            "key component {component:?} may only contain ASCII alphanumerics, '.', '-' and '_'"
        ));
    }
    Ok(())
}

/// Map the reserved legacy id `__global__` to the safe disk component
/// `global`. Every other component passes through verbatim.
pub fn map_reserved_component(component: &str) -> &str {
    match component {
        "__global__" => "global",
        other => other,
    }
}

/// RAII guard for a kernel-held exclusive lease. Dropping releases
/// (best-effort unlock; the kernel also releases on fd close).
#[derive(Debug)]
pub struct LeaseGuard {
    file: std::fs::File,
    path: std::path::PathBuf,
}

impl LeaseGuard {
    /// Path of the held lease file (`<dir>/{key}.lock`).
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }
}

impl Drop for LeaseGuard {
    fn drop(&mut self) {
        // Best-effort unlock. The file is NEVER unlinked (INV4): residual
        // lease files are a few bytes and inert, and unlinking would reopen
        // the swap-inode race the /tmp design suffered from.
        tracing::debug!(lock_path = %self.path.display(), "singleton lease released");
        let _ = fs4::fs_std::FileExt::unlock(&self.file);
    }
}

/// Maximum length of a full lease key (file stem), enforced as
/// defence-in-depth on top of per-component validation. 255 bytes is the
/// common on-disk filename limit.
const MAX_KEY_BYTES: usize = 255;

/// Acquire the exclusive kernel lease at `<dir>/{key}.lock`.
///
/// - Non-blocking, single attempt: `fs4::fs_std::FileExt::try_lock_exclusive`
///   (Linux `flock(2)`; the kernel releases on last fd close).
/// - Contention ([`LeaseError::LockHeld`]) is the ONLY benign outcome; every
///   other error fails closed.
/// - On success the payload `pid=<pid> acquired=<unix_secs>` is rewritten —
///   informational diagnostics only, never used for correctness (no stealing).
/// - Callers validate components with [`validate_key_component`]; this
///   function adds a defence-in-depth check against separator/NUL/empty or
///   oversized keys (mapped to [`LeaseError::Io`], fail-closed).
pub struct LeaseLock;

impl LeaseLock {
    pub fn acquire(dir: impl AsRef<std::path::Path>, key: &str) -> Result<LeaseGuard, LeaseError> {
        if key.is_empty() || key.len() > MAX_KEY_BYTES {
            return Err(LeaseError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("invalid lease key {key:?}: must be 1..={MAX_KEY_BYTES} bytes"),
            )));
        }
        if key.contains('/') || key.contains('\\') || key.contains('\0') {
            return Err(LeaseError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("invalid lease key {key:?}: path separators and NUL are forbidden"),
            )));
        }

        let dir = dir.as_ref();
        std::fs::create_dir_all(dir)?;
        let path = dir.join(format!("{key}.lock"));
        let mut file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)?;

        match fs4::fs_std::FileExt::try_lock_exclusive(&file) {
            Ok(true) => {
                write_payload(&mut file)?;
                Ok(LeaseGuard { file, path })
            }
            // Contended: read the holder pid from the payload best-effort.
            // Never steal, never unlink (INV3/INV4).
            Ok(false) => {
                let holder_pid = read_holder_pid(&mut file);
                Err(LeaseError::LockHeld { holder_pid })
            }
            Err(e) => Err(LeaseError::Io(e)),
        }
    }
}

/// Rewrite the informational payload: `pid=<pid> acquired=<unix_secs>`.
///
/// Exactly this byte contract — frozen across repos. No ULID/instance
/// fields; the timestamp is diagnostics-only and never drives correctness.
fn write_payload(file: &mut std::fs::File) -> std::io::Result<()> {
    let pid = std::process::id();
    let acquired = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    use std::io::{Seek, Write};
    file.set_len(0)?;
    file.rewind()?;
    write!(file, "pid={pid} acquired={acquired}")?;
    file.flush()
}

/// Parse `pid=` out of an existing payload, best-effort.
fn read_holder_pid(file: &mut std::fs::File) -> Option<u32> {
    use std::io::{Read, Seek};
    let mut buf = String::new();
    let _ = file.rewind();
    let _ = file.read_to_string(&mut buf);
    parse_holder_pid(&buf)
}

/// Payload parser kept separate so tests can exercise it directly.
pub(crate) fn parse_holder_pid(payload: &str) -> Option<u32> {
    payload.split_whitespace().find_map(|token| {
        token
            .strip_prefix("pid=")
            .and_then(|pid| pid.parse::<u32>().ok())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tempdir() -> tempfile::TempDir {
        tempfile::tempdir().expect("tempdir")
    }

    // ---- acquire / payload / guard-drop ----

    #[test]
    fn acquire_creates_lock_file_with_exact_payload() {
        let dir = tempdir();
        let guard = LeaseLock::acquire(dir.path(), "agent-terraphim--merge-coordinator")
            .expect("first acquire must succeed");

        let path = dir.path().join("agent-terraphim--merge-coordinator.lock");
        assert!(path.exists(), "lock file must exist at <dir>/{{key}}.lock");
        let payload = std::fs::read_to_string(&path).unwrap();
        let expected = format!("pid={} acquired=", std::process::id());
        assert!(
            payload.starts_with(&expected),
            "payload must be exactly 'pid=<pid> acquired=<unix_secs>'; got {payload:?}"
        );
        let acquired: String = payload
            .strip_prefix(&expected)
            .expect("acquired= suffix checked above")
            .to_string();
        assert!(
            acquired.chars().all(|c| c.is_ascii_digit()) && !acquired.is_empty(),
            "acquired must be unix seconds; got {acquired:?}"
        );
        assert_eq!(guard.path(), path.as_path(), "guard exposes the lease path");
        drop(guard);
    }

    #[test]
    fn second_acquire_in_same_process_returns_lock_held_with_holder_pid() {
        let dir = tempdir();
        let _guard = LeaseLock::acquire(dir.path(), "k").expect("first acquire");

        match LeaseLock::acquire(dir.path(), "k") {
            Err(LeaseError::LockHeld { holder_pid }) => {
                // Payload is written by this same process, so the pid must
                // be discoverable. (A foreign holder without a parsable
                // payload yields None -- never a correctness decision.)
                assert_eq!(holder_pid, Some(std::process::id()));
            }
            other => panic!("expected LockHeld, got {other:?}"),
        }
    }

    #[test]
    fn guard_drop_releases_lock() {
        let dir = tempdir();
        let guard = LeaseLock::acquire(dir.path(), "k").expect("first acquire");
        drop(guard);
        let _second = LeaseLock::acquire(dir.path(), "k")
            .expect("drop of the guard must release the kernel lease");
    }

    #[test]
    fn lock_file_is_never_unlinked() {
        // INV4: the software never unlinks lease files. Residual files are
        // inert; dropping the guard must leave the path in place.
        let dir = tempdir();
        let path = dir.path().join("k.lock");
        let guard = LeaseLock::acquire(dir.path(), "k").expect("acquire");
        drop(guard);
        assert!(
            path.exists(),
            "lease file must survive guard drop (never unlinked)"
        );
    }

    #[test]
    fn foreign_fd_close_releases_lock() {
        // Kernel crash semantics: any holder that merely closes its fd
        // (crash, SIGKILL) releases the lease. Simulate a foreign holder by
        // locking the file directly with a raw fd, then dropping it.
        let dir = tempdir();
        let path = dir.path().join("k.lock");
        let foreign = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .expect("open foreign fd");
        fs4::fs_std::FileExt::try_lock_exclusive(&foreign)
            .expect("foreign try_lock_exclusive io")
            .then_some(())
            .expect("foreign lock must be acquired");

        match LeaseLock::acquire(dir.path(), "k") {
            Err(LeaseError::LockHeld { .. }) => {}
            other => panic!("expected LockHeld while foreign fd holds the lease, got {other:?}"),
        }

        drop(foreign); // fd close, no explicit unlock
        let _guard = LeaseLock::acquire(dir.path(), "k")
            .expect("kernel must release the lease when the holder fd closes");
    }

    #[test]
    fn stale_payload_is_never_stolen() {
        // No timestamp-based stealing (INV3): a forged 100 s-old payload must
        // not change the LockHeld verdict.
        let dir = tempdir();
        let path = dir.path().join("k.lock");
        let stale = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .saturating_sub(100);
        std::fs::write(&path, format!("pid=999999 acquired={stale}")).unwrap();

        let holder = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .truncate(false)
            .open(&path)
            .unwrap();
        fs4::fs_std::FileExt::try_lock_exclusive(&holder)
            .expect("holder try_lock_exclusive io")
            .then_some(())
            .expect("holder lock");

        match LeaseLock::acquire(dir.path(), "k") {
            Err(LeaseError::LockHeld { holder_pid }) => {
                assert_eq!(
                    holder_pid,
                    Some(999_999),
                    "forged payload pid is read best-effort"
                );
            }
            other => panic!("expected LockHeld despite stale payload, got {other:?}"),
        }
    }

    #[test]
    fn acquire_creates_missing_lock_dir() {
        let dir = tempdir();
        let nested = dir.path().join("locks").join("deep");
        let _guard = LeaseLock::acquire(&nested, "k").expect("create_dir_all then acquire");
        assert!(nested.join("k.lock").exists());
    }

    #[test]
    fn acquire_fails_closed_when_dir_path_is_a_regular_file() {
        let dir = tempdir();
        let not_a_dir = dir.path().join("occupied");
        std::fs::write(&not_a_dir, b"x").unwrap();
        match LeaseLock::acquire(&not_a_dir, "k") {
            Err(LeaseError::Io(_)) => {}
            other => panic!("expected Io (fail-closed), got {other:?}"),
        }
    }

    #[test]
    fn acquire_rejects_keys_with_path_separators() {
        let dir = tempdir();
        for bad in ["../evil", "a/b", "a\\b", "", "a\0b"] {
            match LeaseLock::acquire(dir.path(), bad) {
                Err(LeaseError::Io(_)) => {}
                other => panic!("expected Io for invalid key {bad:?}, got {other:?}"),
            }
        }
        assert!(
            !dir.path().join("evil.lock").exists(),
            "path traversal must not create files outside the lock dir"
        );
    }

    #[test]
    fn distinct_keys_use_distinct_lock_files() {
        let dir = tempdir();
        let a = LeaseLock::acquire(dir.path(), "merge-coordinator-terraphim--terraphim-ai")
            .expect("first repo key");
        let b = LeaseLock::acquire(dir.path(), "merge-coordinator-other--repo")
            .expect("second repo key");
        assert_ne!(a.path(), b.path(), "per-repo keys must not collide");
    }

    // ---- key component validation ----

    #[test]
    fn validate_key_component_accepts_safe_components() {
        for ok in [
            "a",
            "a.b-c_9",
            "global",
            "terraphim-ai",
            "agent-terraphim--merge-coordinator",
            "A9",
            "z".repeat(127).as_str(),
        ] {
            assert!(
                validate_key_component(ok).is_ok(),
                "component {ok:?} must be accepted"
            );
        }
    }

    #[test]
    fn validate_key_component_rejects_unsafe_components() {
        for bad in [
            "",
            "../",
            "..",
            "a/b",
            "a\\b",
            "a\0b",
            "café",
            "café",
            "__global__",
            "-leading-dot-dash",
            ".leading",
            "9".repeat(128).as_str(),
        ] {
            assert!(
                validate_key_component(bad).is_err(),
                "component {bad:?} must be rejected"
            );
        }
    }

    #[test]
    fn map_reserved_component_maps_only_the_legacy_global_id() {
        assert_eq!(map_reserved_component("__global__"), "global");
        assert_eq!(map_reserved_component("terraphim"), "terraphim");
        assert_eq!(map_reserved_component(""), "");
    }
}
