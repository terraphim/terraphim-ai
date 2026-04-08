use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn workspace_root() -> Result<PathBuf> {
    let mut current = std::env::current_dir()?;

    loop {
        let cargo_toml = current.join("Cargo.toml");
        if cargo_toml.exists() {
            let content = fs::read_to_string(&cargo_toml)?;
            if content.contains("[workspace]") {
                return Ok(current);
            }
        }

        if !current.pop() {
            break;
        }
    }

    Err(anyhow::anyhow!("could not locate workspace root"))
}

fn create_unique_test_root() -> Result<PathBuf> {
    let nonce = COUNTER.fetch_add(1, Ordering::SeqCst);
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system time before unix epoch")?
        .as_nanos();

    let root = std::env::temp_dir().join(format!(
        "terraphim-agent-hermetic-tests-{}-{}-{}",
        std::process::id(),
        ts,
        nonce
    ));

    fs::create_dir_all(&root)?;
    Ok(root)
}

pub fn apply_hermetic_env(cmd: &mut Command) -> Result<()> {
    let root = create_unique_test_root()?;
    let home_dir = root.join("home");
    let xdg_config_home = root.join("xdg-config");
    let data_dir = root.join("data");

    fs::create_dir_all(&home_dir)?;
    fs::create_dir_all(&xdg_config_home)?;
    fs::create_dir_all(&data_dir)?;

    let workspace = workspace_root()?;
    let role_config = workspace.join("terraphim_server/default/terraphim_engineer_config.json");

    cmd.current_dir(&workspace)
        .env("HOME", &home_dir)
        .env("XDG_CONFIG_HOME", &xdg_config_home)
        .env("TERRAPHIM_DEFAULT_DATA_PATH", &data_dir)
        .env("TERRAPHIM_ROLE_CONFIG", &role_config);

    Ok(())
}
