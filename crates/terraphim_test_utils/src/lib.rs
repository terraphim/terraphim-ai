//! Shared test utilities for Terraphim crates.
//!
//! This crate provides safe wrappers around environment variable operations
//! that handle the Rust 1.92+ change making `std::env::set_var` and
//! `std::env::remove_var` unsafe.
//!
//! # Usage
//!
//! ```rust,ignore
//! use terraphim_test_utils::{set_env_var, remove_env_var, EnvVarGuard};
//!
//! // Direct usage
//! set_env_var("MY_VAR", "value");
//! remove_env_var("MY_VAR");
//!
//! // RAII guard for automatic cleanup
//! let _guard = EnvVarGuard::set("MY_VAR", "temp_value");
//! // MY_VAR is restored to its original value when _guard is dropped
//! ```
//!
//! # Safety
//!
//! These functions wrap potentially unsafe operations. Callers must ensure:
//! - No other threads are concurrently reading or writing the same env vars
//! - Tests using these functions should use `#[serial]` from `serial_test` crate

use std::ffi::{OsStr, OsString};

/// Sets an environment variable.
///
/// # Safety
///
/// On Rust 1.92+, this wraps the unsafe `std::env::set_var`. Callers must
/// ensure no other threads are concurrently accessing the environment.
#[cfg(rust_has_unsafe_env_setters)]
pub fn set_env_var<K: AsRef<OsStr>, V: AsRef<OsStr>>(key: K, value: V) {
    // SAFETY: Caller ensures no concurrent env access from other threads.
    // Tests using this should be marked with #[serial] to ensure isolation.
    unsafe {
        std::env::set_var(key, value);
    }
}

/// Sets an environment variable.
#[cfg(not(rust_has_unsafe_env_setters))]
pub fn set_env_var<K: AsRef<OsStr>, V: AsRef<OsStr>>(key: K, value: V) {
    std::env::set_var(key, value);
}

/// Removes an environment variable.
///
/// # Safety
///
/// On Rust 1.92+, this wraps the unsafe `std::env::remove_var`. Callers must
/// ensure no other threads are concurrently accessing the environment.
#[cfg(rust_has_unsafe_env_setters)]
pub fn remove_env_var<K: AsRef<OsStr>>(key: K) {
    // SAFETY: Caller ensures no concurrent env access from other threads.
    // Tests using this should be marked with #[serial] to ensure isolation.
    unsafe {
        std::env::remove_var(key);
    }
}

/// Removes an environment variable.
#[cfg(not(rust_has_unsafe_env_setters))]
pub fn remove_env_var<K: AsRef<OsStr>>(key: K) {
    std::env::remove_var(key);
}

/// RAII guard that restores an environment variable to its original state on drop.
///
/// # Example
///
/// ```rust,ignore
/// use terraphim_test_utils::EnvVarGuard;
///
/// let _guard = EnvVarGuard::set("MY_VAR", "test_value");
/// // MY_VAR is now "test_value"
/// // When _guard goes out of scope, MY_VAR is restored to its original value
/// // (or removed if it didn't exist before)
/// ```
pub struct EnvVarGuard {
    key: OsString,
    original: Option<OsString>,
}

impl EnvVarGuard {
    /// Sets an environment variable and returns a guard that will restore it on drop.
    pub fn set<K: AsRef<OsStr>, V: AsRef<OsStr>>(key: K, value: V) -> Self {
        let key = key.as_ref().to_os_string();
        let original = std::env::var_os(&key);
        set_env_var(&key, value);
        Self { key, original }
    }

    /// Removes an environment variable and returns a guard that will restore it on drop.
    pub fn remove<K: AsRef<OsStr>>(key: K) -> Self {
        let key = key.as_ref().to_os_string();
        let original = std::env::var_os(&key);
        remove_env_var(&key);
        Self { key, original }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        if let Some(value) = &self.original {
            set_env_var(&self.key, value);
        } else {
            remove_env_var(&self.key);
        }
    }
}

/// Helper for managing multiple environment variables in tests.
///
/// Stores original values and restores them on drop.
pub struct TestEnv {
    original_vars: Vec<(OsString, Option<OsString>)>,
}

impl TestEnv {
    /// Creates a new TestEnv.
    pub fn new() -> Self {
        Self {
            original_vars: Vec::new(),
        }
    }

    /// Sets an environment variable and stores the original value for restoration.
    pub fn set<K: AsRef<OsStr>, V: AsRef<OsStr>>(&mut self, key: K, value: V) {
        let key = key.as_ref().to_os_string();
        self.original_vars
            .push((key.clone(), std::env::var_os(&key)));
        set_env_var(&key, value);
    }

    /// Removes an environment variable and stores the original value for restoration.
    pub fn remove<K: AsRef<OsStr>>(&mut self, key: K) {
        let key = key.as_ref().to_os_string();
        self.original_vars
            .push((key.clone(), std::env::var_os(&key)));
        remove_env_var(&key);
    }

    /// Removes multiple LLM-related environment variables.
    ///
    /// Useful for test isolation when testing LLM proxy configurations.
    pub fn cleanup_llm_env_vars(&mut self) {
        let llm_vars = [
            "ANTHROPIC_BASE_URL",
            "ANTHROPIC_AUTH_TOKEN",
            "ANTHROPIC_API_KEY",
            "OPENROUTER_BASE_URL",
            "OPENROUTER_API_KEY",
            "OLLAMA_API_BASE",
            "OLLAMA_API_KEY",
        ];

        for var in &llm_vars {
            if std::env::var(var).is_ok() {
                self.remove(var);
            }
        }
    }
}

impl Default for TestEnv {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        // Restore original environment variables in reverse order
        for (key, original_value) in self.original_vars.drain(..).rev() {
            match original_value {
                Some(value) => set_env_var(&key, value),
                None => remove_env_var(&key),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_and_remove_env_var() {
        let key = "TERRAPHIM_TEST_UTILS_TEST_VAR";

        // Ensure clean state
        remove_env_var(key);
        assert!(std::env::var(key).is_err());

        // Set and verify
        set_env_var(key, "test_value");
        assert_eq!(std::env::var(key).unwrap(), "test_value");

        // Remove and verify
        remove_env_var(key);
        assert!(std::env::var(key).is_err());
    }

    #[test]
    fn test_env_var_guard_restores_original() {
        let key = "TERRAPHIM_TEST_UTILS_GUARD_TEST";

        // Set initial value
        set_env_var(key, "original");

        {
            let _guard = EnvVarGuard::set(key, "modified");
            assert_eq!(std::env::var(key).unwrap(), "modified");
        }

        // Original value should be restored
        assert_eq!(std::env::var(key).unwrap(), "original");

        // Cleanup
        remove_env_var(key);
    }

    #[test]
    fn test_env_var_guard_removes_if_not_present() {
        let key = "TERRAPHIM_TEST_UTILS_GUARD_NEW_VAR";

        // Ensure it doesn't exist
        remove_env_var(key);

        {
            let _guard = EnvVarGuard::set(key, "temporary");
            assert_eq!(std::env::var(key).unwrap(), "temporary");
        }

        // Should be removed after guard drops
        assert!(std::env::var(key).is_err());
    }

    #[test]
    fn test_test_env_restores_all() {
        let key1 = "TERRAPHIM_TEST_ENV_1";
        let key2 = "TERRAPHIM_TEST_ENV_2";

        // Set initial values
        set_env_var(key1, "original1");
        remove_env_var(key2);

        {
            let mut env = TestEnv::new();
            env.set(key1, "modified1");
            env.set(key2, "new_value");

            assert_eq!(std::env::var(key1).unwrap(), "modified1");
            assert_eq!(std::env::var(key2).unwrap(), "new_value");
        }

        // Original values should be restored
        assert_eq!(std::env::var(key1).unwrap(), "original1");
        assert!(std::env::var(key2).is_err());

        // Cleanup
        remove_env_var(key1);
    }
}
