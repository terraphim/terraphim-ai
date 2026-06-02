//! Shared command-runtime helpers for Terraphim CLI front-ends.
//!
//! Extracts the configuration and role-resolution logic that was duplicated
//! between `terraphim_cli`'s `CliService` and `terraphim_agent`'s `TuiService`
//! (ADR-003, Gitea #1910). Both front-ends now delegate this common surface
//! here, so the behaviour can no longer drift between the two binaries.
//!
//! The helpers operate on a borrowed [`ConfigState`] (and `Persistable` for the
//! save path) rather than owning service state, so each front-end keeps its own
//! struct and simply forwards these calls.

use terraphim_config::{Config, ConfigState};
use terraphim_persistence::Persistable;
use terraphim_types::RoleName;

/// Snapshot the current configuration.
pub async fn get_config(config_state: &ConfigState) -> Config {
    let config = config_state.config.lock().await;
    config.clone()
}

/// Return the currently selected role.
pub async fn get_selected_role(config_state: &ConfigState) -> RoleName {
    let config = config_state.config.lock().await;
    config.selected_role.clone()
}

/// List all configured roles with their optional shortnames.
pub async fn list_roles_with_info(config_state: &ConfigState) -> Vec<(String, Option<String>)> {
    let config = config_state.config.lock().await;
    config
        .roles
        .iter()
        .map(|(name, role)| (name.to_string(), role.shortname.clone()))
        .collect()
}

/// Find a role by exact name, then by shortname (both case-insensitive).
pub async fn find_role_by_name_or_shortname(
    config_state: &ConfigState,
    query: &str,
) -> Option<RoleName> {
    let config = config_state.config.lock().await;
    let query_lower = query.to_lowercase();

    // First try exact match on name
    for (name, _role) in config.roles.iter() {
        if name.to_string().to_lowercase() == query_lower {
            return Some(name.clone());
        }
    }

    // Then try match on shortname
    for (name, role) in config.roles.iter() {
        if let Some(ref shortname) = role.shortname {
            if shortname.to_lowercase() == query_lower {
                return Some(name.clone());
            }
        }
    }

    None
}

/// Persist the current configuration to all configured backends.
pub async fn save_config(config_state: &ConfigState) -> anyhow::Result<()> {
    let config = config_state.config.lock().await;
    config.save().await?;
    Ok(())
}
