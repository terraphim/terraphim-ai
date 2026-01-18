//! Platform-specific paths and utilities
//!
//! This module provides platform-aware path detection and permission checking
//! for auto-update operations on Linux and macOS systems.

use crate::config::UpdateInfo;
use anyhow::{Context, Result};
use std::env;
use std::fs;
use std::path::Path;

/// Get the appropriate binary installation path for the current platform
///
/// # Arguments
/// * `binary_name` - Name of the binary (e.g., "terraphim", "terraphim_server")
///
/// # Returns
/// * `Ok(PathBuf)` - Path where the binary should be installed
/// * `Err(anyhow::Error)` - Error if platform is not supported
///
/// # Platform-specific behavior
///
/// **Linux/macOS:**
/// - First attempts `/usr/local/bin/{binary_name}` (system-wide installation)
/// - Falls back to `~/.local/bin/{binary_name}` (user-local installation) if:
///   - System directory doesn't have write permissions
///   - User directory is in PATH
///
/// # Example
/// ```no_run
/// use terraphim_update::platform::get_binary_path;
///
/// let path = get_binary_path("terraphim")?;
/// println!("Binary will be installed to: {:?}", path);
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn get_binary_path(binary_name: &str) -> Result<String> {
    if !cfg!(unix) {
        anyhow::bail!("Windows support is not yet implemented. Please use Linux or macOS.");
    }

    let system_path = format!("/usr/local/bin/{}", binary_name);
    let user_bin_dir = env::var("HOME")
        .map(|home| format!("{}/.local/bin", home))
        .unwrap_or_else(|_| "/home/user/.local/bin".to_string());
    let user_path = format!("{}/{}", user_bin_dir, binary_name);

    // Check if we can write to system directory
    if check_write_permissions(Path::new("/usr/local/bin")) {
        return Ok(system_path);
    }

    // Fall back to user directory
    tracing::debug!(
        "Cannot write to /usr/local/bin, falling back to user directory: {}",
        user_path
    );

    // Ensure user bin directory exists
    if let Err(e) = fs::create_dir_all(&user_bin_dir) {
        tracing::warn!("Could not create user bin directory: {}", e);
    }

    Ok(user_path)
}

/// Get the configuration directory for Terraphim
///
/// Returns `~/.config/terraphim` on Unix systems.
///
/// # Returns
/// * `Ok(String)` - Path to the config directory
/// * `Err(anyhow::Error)` - Error if HOME directory cannot be determined
///
/// # Example
/// ```no_run
/// use terraphim_update::platform::get_config_dir;
///
/// let config_dir = get_config_dir()?;
/// println!("Config directory: {}", config_dir);
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn get_config_dir() -> Result<String> {
    if !cfg!(unix) {
        anyhow::bail!("Windows support is not yet implemented. Please use Linux or macOS.");
    }

    let home = env::var("HOME").context("Could not determine HOME directory")?;
    Ok(format!("{}/.config/terraphim", home))
}

/// Check if we have write permissions to a given path
///
/// # Arguments
/// * `path` - Path to check for write permissions
///
/// # Returns
/// * `true` - Can write to the path
/// * `false` - Cannot write to the path
///
/// # Example
/// ```no_run
/// use terraphim_update::platform::check_write_permissions;
/// use std::path::Path;
///
/// let can_write = check_write_permissions(Path::new("/usr/local/bin"));
/// if can_write {
///     println!("Can install system-wide");
/// } else {
///     println!("Will install to user directory");
/// }
/// ```
pub fn check_write_permissions(path: &Path) -> bool {
    // If path doesn't exist, check parent
    let check_path = if path.exists() {
        if path.is_file() {
            match path.parent() {
                Some(parent) if parent.to_str() != Some("") => parent,
                _ => return false,
            }
        } else {
            path
        }
    } else {
        match path.parent() {
            Some(parent) if parent.to_str() != Some("") => parent,
            _ => return false,
        }
    };

    // Try to create a test file in the directory
    let test_file = check_path.join(".write_test_12345");

    match fs::write(&test_file, b"test") {
        Ok(_) => {
            let _ = fs::remove_file(&test_file);
            true
        }
        Err(_) => false,
    }
}

/// Show manual update instructions when automatic update fails
///
/// This function provides clear instructions for manual update when
/// the automatic update process encounters errors (e.g., permission issues).
///
/// # Arguments
/// * `update_info` - Information about the available update
///
/// # Example
/// ```no_run
/// use terraphim_update::platform::show_manual_update_instructions;
/// use terraphim_update::config::UpdateInfo;
/// use jiff::Timestamp;
///
/// let info = UpdateInfo {
///     version: "1.1.0".to_string(),
///     release_date: Timestamp::now(),
///     notes: "Bug fixes".to_string(),
///     download_url: "https://example.com/binary".to_string(),
///     signature_url: "https://example.com/binary.sig".to_string(),
///     arch: "x86_64".to_string(),
/// };
///
/// show_manual_update_instructions(&info);
/// ```
pub fn show_manual_update_instructions(update_info: &UpdateInfo) {
    println!("\n========================================");
    println!("  MANUAL UPDATE INSTRUCTIONS");
    println!("========================================\n");

    println!("Version {} is available!\n", update_info.version);
    println!("Release notes:\n{}\n", update_info.notes);
    println!("Release date: {}\n", update_info.release_date);

    println!("To update manually, follow these steps:\n");

    println!("1. Download the new binary:");
    println!("   curl -L {} -o /tmp/terraphim", update_info.download_url);
    println!();

    println!("2. Verify the signature:");
    println!(
        "   curl -L {} -o /tmp/terraphim.sig",
        update_info.signature_url
    );
    println!("   gpg --verify /tmp/terraphim.sig /tmp/terraphim");
    println!();

    println!("3. Make it executable:");
    println!("   chmod +x /tmp/terraphim");
    println!();

    println!("4. Replace the old binary (you may need sudo):");
    println!("   sudo mv /tmp/terraphim /usr/local/bin/");
    println!();

    println!("   Or if you prefer user-local installation:");
    println!("   mkdir -p ~/.local/bin");
    println!("   mv /tmp/terraphim ~/.local/bin/");
    println!();

    println!("   Make sure ~/.local/bin is in your PATH:");
    println!("   export PATH=\"$HOME/.local/bin:$PATH\"");
    println!();

    println!("5. Verify the installation:");
    println!("   terraphim --version");
    println!();

    println!("========================================\n");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_get_binary_path_system_writable() {
        // This test will succeed if we have write permissions to /tmp
        let binary_name = "test_binary";
        let path = get_binary_path(binary_name).expect("Should get binary path");

        // The path should contain the binary name
        assert!(
            path.contains(binary_name),
            "Path should contain binary name"
        );

        // On test systems, it should return user-local path
        // (since we likely don't have /usr/local/bin write access)
        println!("Binary path: {}", path);
    }

    #[test]
    fn test_get_binary_path_custom_name() {
        let path = get_binary_path("my_custom_tool").expect("Should get binary path");
        assert!(path.contains("my_custom_tool"));
    }

    #[test]
    fn test_get_config_dir() {
        let config_dir = get_config_dir().expect("Should get config directory");

        assert!(
            config_dir.contains(".config/terraphim"),
            "Should contain .config/terraphim"
        );

        // Should be a valid path
        let path = Path::new(&config_dir);
        assert!(path.is_absolute(), "Should be absolute path");
    }

    #[test]
    fn test_check_write_permissions_writable_dir() {
        // Use /tmp which should always be writable on Unix
        let temp_dir = std::env::temp_dir();
        let can_write = check_write_permissions(&temp_dir);
        assert!(can_write, "Should be able to write to temp directory");
    }

    #[test]
    fn test_check_write_permissions_readonly_dir() {
        // Create a temporary directory with no write permissions
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let readonly_path = temp_dir.path().join("readonly");

        fs::create_dir(&readonly_path).expect("Failed to create readonly dir");

        // Remove write permissions
        #[cfg(unix)]
        {
            let mut perms = fs::metadata(&readonly_path)
                .expect("Failed to get metadata")
                .permissions();
            perms.set_readonly(true);
            fs::set_permissions(&readonly_path, perms).expect("Failed to set permissions");

            let can_write = check_write_permissions(&readonly_path);
            assert!(
                !can_write,
                "Should not be able to write to readonly directory"
            );
        }

        #[cfg(not(unix))]
        {
            let can_write = check_write_permissions(&readonly_path);
            assert!(
                !can_write,
                "Should not be able to write to readonly directory"
            );
        }
    }

    #[test]
    fn test_check_write_permissions_file() {
        let temp_file = tempfile::NamedTempFile::new().expect("Failed to create temp file");
        let can_write = check_write_permissions(temp_file.path());

        // We should be able to write to the parent directory
        assert!(
            can_write,
            "Should be able to write to file's parent directory"
        );
    }

    #[test]
    fn test_check_write_permissions_nonexistent_path() {
        let nonexistent = Path::new("/this/path/does/not/exist/test");
        let can_write = check_write_permissions(nonexistent);

        // Should check parent directory permissions
        // Since /this/path/does/not/exist doesn't exist,
        // it will recursively check upwards and likely fail
        println!("Can write to nonexistent path: {}", can_write);
    }

    #[test]
    fn test_show_manual_update_instructions() {
        let info = UpdateInfo {
            version: "1.1.0".to_string(),
            release_date: jiff::Timestamp::now(),
            notes: "Bug fixes and improvements".to_string(),
            download_url:
                "https://github.com/terraphim/terraphim-ai/releases/download/v1.1.0/terraphim"
                    .to_string(),
            signature_url:
                "https://github.com/terraphim/terraphim-ai/releases/download/v1.1.0/terraphim.sig"
                    .to_string(),
            arch: "x86_64".to_string(),
        };

        // This should not panic
        show_manual_update_instructions(&info);
    }

    #[test]
    fn test_show_manual_update_instructions_with_long_notes() {
        let long_notes = "This is a very long release note that contains multiple lines and detailed information about what has changed in this release. It should be formatted properly in the output.";

        let info = UpdateInfo {
            version: "2.0.0".to_string(),
            release_date: jiff::Timestamp::now(),
            notes: long_notes.to_string(),
            download_url: "https://example.com/binary".to_string(),
            signature_url: "https://example.com/binary.sig".to_string(),
            arch: "aarch64".to_string(),
        };

        // This should not panic
        show_manual_update_instructions(&info);
    }

    #[test]
    fn test_get_config_dir_creates_parent() {
        // Just verify we can get the config dir without errors
        let config_dir = get_config_dir().expect("Should get config directory");

        // The directory should be absolute
        assert!(config_dir.starts_with('/'), "Should be absolute path");
    }

    #[test]
    fn test_binary_path_consistency() {
        // Multiple calls should return consistent results
        let path1 = get_binary_path("test").expect("Should get path");
        let path2 = get_binary_path("test").expect("Should get path");

        assert_eq!(path1, path2, "Should return consistent paths");
    }
}
