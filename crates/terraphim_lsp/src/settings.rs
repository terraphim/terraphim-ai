//! Server settings, read from `initializationOptions` and
//! `workspace/didChangeConfiguration`.
//!
//! Every field is optional; unknown fields are ignored. The same object is
//! accepted bare or nested under a `terraphim` key, so editor settings such
//! as Zed's `lsp.terraphim-lsp.initialization_options` can pass either:
//!
//! ```json
//! {
//!   "inlayHints": false,
//!   "ghostDiagnostics": true,
//!   "lab": { "actions": [], "trigger": "save" }
//! }
//! ```
//!
//! | Setting | Default | Effect |
//! |---|---|---|
//! | `inlayHints` | `false` | `[i/n]` after each KG term with alternatives |
//! | `ghostDiagnostics` | `true` | ghosted text published as faded (`Unnecessary`) hints |
//! | `lab.actions` | `[]` (off) | Lab actions run automatically (names as in `terraphim.lab.mark`) |
//! | `lab.trigger` | `"save"` | when Lab marks and the trim preview are recomputed: `save` (on open and save) or `command` (only on `terraphim.lab.mark` / `terraphim.trim.preview`) |
//!
//! Lab marks are never computed on every keystroke: the Lab engine parses
//! the whole Markdown document per run, and marks flickering while typing
//! would be noise. Any edit drops the current marks (their ranges would be
//! stale) until the next save or command.

use serde::Deserialize;
use serde_json::Value;
use terraphim_lsp_core::LabAction;

/// Settings that change what the server publishes.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ServerSettings {
    /// Return `[i/n]` inlay hints (position of the current form among its
    /// concept's terms). Off by default: hints change line layout, and Zed
    /// keeps inlay hints off by default too.
    pub inlay_hints: bool,
    /// Publish ghosted spans from the annotation block as
    /// `DiagnosticTag::Unnecessary` hints, which clients fade. On by default:
    /// it is derived from the file itself and costs nothing extra.
    pub ghost_diagnostics: bool,
    /// Lab marks.
    pub lab: LabSettings,
}

impl Default for ServerSettings {
    fn default() -> Self {
        Self {
            inlay_hints: false,
            ghost_diagnostics: true,
            lab: LabSettings::default(),
        }
    }
}

/// Lab-mark settings.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LabSettings {
    /// Actions run automatically on every open document. Empty (off) by
    /// default; `terraphim.lab.mark` runs an action on demand regardless.
    pub actions: Vec<LabAction>,
    /// When Lab results are recomputed.
    pub trigger: LabTrigger,
}

/// When Lab marks and the trim preview are recomputed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LabTrigger {
    /// On open and on save (`textDocument/didSave`), and on command.
    #[default]
    Save,
    /// Only on `terraphim.lab.mark` / `terraphim.trim.preview`.
    Command,
}

impl ServerSettings {
    /// Settings from an `initializationOptions` or `settings` value: the
    /// object itself, or its `terraphim` member. `null` or a missing value
    /// gives the defaults; an invalid value is logged and gives the
    /// defaults rather than failing the request.
    pub fn from_value(value: Option<&Value>) -> Self {
        let Some(value) = value.filter(|value| !value.is_null()) else {
            return Self::default();
        };
        let value = value.get("terraphim").unwrap_or(value);
        serde_json::from_value(value.clone()).unwrap_or_else(|error| {
            log::warn!("terraphim_lsp: ignoring invalid settings ({error}): {value}");
            Self::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn defaults_when_absent_or_null() {
        assert_eq!(ServerSettings::from_value(None), ServerSettings::default());
        assert_eq!(
            ServerSettings::from_value(Some(&Value::Null)),
            ServerSettings::default()
        );
        let defaults = ServerSettings::default();
        assert!(!defaults.inlay_hints);
        assert!(defaults.ghost_diagnostics);
    }

    #[test]
    fn bare_or_nested_objects() {
        let bare = json!({"inlayHints": true, "unknown": 1});
        assert!(ServerSettings::from_value(Some(&bare)).inlay_hints);
        let nested = json!({"terraphim": {"ghostDiagnostics": false}});
        let settings = ServerSettings::from_value(Some(&nested));
        assert!(!settings.ghost_diagnostics);
        assert!(!settings.inlay_hints);
    }

    #[test]
    fn lab_settings() {
        let value = json!({"lab": {"actions": ["hedges_and_filler"], "trigger": "command"}});
        let settings = ServerSettings::from_value(Some(&value));
        assert_eq!(settings.lab.actions, [LabAction::HedgesAndFiller]);
        assert_eq!(settings.lab.trigger, LabTrigger::Command);
        assert_eq!(ServerSettings::default().lab.trigger, LabTrigger::Save);
        assert!(ServerSettings::default().lab.actions.is_empty());
    }

    #[test]
    fn invalid_values_fall_back_to_defaults() {
        let invalid = json!({"inlayHints": "yes"});
        assert_eq!(
            ServerSettings::from_value(Some(&invalid)),
            ServerSettings::default()
        );
    }
}
