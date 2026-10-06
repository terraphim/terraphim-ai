//! Server settings, read from `initializationOptions` and
//! `workspace/didChangeConfiguration`.
//!
//! Every field is optional; unknown fields are ignored. The same object is
//! accepted bare or nested under a `terraphim` key, so editor settings such
//! as Zed's `lsp.terraphim-lsp.initialization_options` can pass either; when
//! both appear, their keys are merged and the nested value wins per
//! top-level key:
//!
//! ```json
//! {
//!   "thesaurus": "/path/to/thesaurus.json",
//!   "inlayHints": false,
//!   "ghostDiagnostics": true,
//!   "unknownTerms": false,
//!   "lab": { "actions": [], "trigger": "save" }
//! }
//! ```
//!
//! | Setting | Default | Effect |
//! |---|---|---|
//! | `thesaurus` | none | path of a thesaurus JSON file; overrides `--thesaurus` and `TERRAPHIM_THESAURUS`; an `initializationOptions` value survives later settings without one (see [`crate::thesaurus`]); reloaded when it changes |
//! | `inlayHints` | `false` | `[i/n]` after each KG term with alternatives |
//! | `ghostDiagnostics` | `true` | ghosted text published as faded (`Unnecessary`) hints |
//! | `unknownTerms` | `false` | a Warning on every occurrence of a word that matches no thesaurus term |
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
    /// Path of the thesaurus JSON file. `None` (or blank) falls back to the
    /// launch path; see [`crate::thesaurus`].
    pub thesaurus: Option<String>,
    /// Return `[i/n]` inlay hints (position of the current form among its
    /// concept's terms). Off by default: hints change line layout, and Zed
    /// keeps inlay hints off by default too.
    pub inlay_hints: bool,
    /// Publish ghosted spans from the annotation block as
    /// `DiagnosticTag::Unnecessary` hints, which clients fade. On by default:
    /// it is derived from the file itself and costs nothing extra.
    pub ghost_diagnostics: bool,
    /// Publish a Warning on every occurrence of a word that is not part of
    /// any thesaurus match. Off by default: against a real thesaurus almost
    /// every ordinary word is unknown, which buries the useful diagnostics.
    pub unknown_terms: bool,
    /// Lab marks.
    pub lab: LabSettings,
}

impl Default for ServerSettings {
    fn default() -> Self {
        Self {
            thesaurus: None,
            inlay_hints: false,
            ghost_diagnostics: true,
            unknown_terms: false,
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
    /// object's own keys merged with those of its `terraphim` member, the
    /// nested value winning per top-level key (a nested `lab` replaces a
    /// bare `lab` as a whole). `null` or a missing value
    /// gives the defaults; an invalid value is logged and gives the
    /// defaults rather than failing the request.
    pub fn from_value(value: Option<&Value>) -> Self {
        let Some(value) = value.filter(|value| !value.is_null()) else {
            return Self::default();
        };
        let value = merge_nested(value);
        serde_json::from_value(value.clone()).unwrap_or_else(|error| {
            log::warn!("terraphim_lsp: ignoring invalid settings ({error}): {value}");
            Self::default()
        })
    }
}

/// `value`'s keys (without `terraphim`) overlaid with those of
/// `value.terraphim` when both are objects; otherwise the nested value if
/// present, else `value` itself.
fn merge_nested(value: &Value) -> Value {
    match (value, value.get("terraphim")) {
        (Value::Object(bare), Some(Value::Object(nested))) => {
            let mut merged = bare.clone();
            merged.remove("terraphim");
            merged.extend(
                nested
                    .iter()
                    .map(|(key, value)| (key.clone(), value.clone())),
            );
            Value::Object(merged)
        }
        (_, Some(nested)) => nested.clone(),
        (_, None) => value.clone(),
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
        assert!(!defaults.unknown_terms);
    }

    #[test]
    fn thesaurus_path_bare_or_nested() {
        let bare = json!({"thesaurus": "/kg/t.json"});
        assert_eq!(
            ServerSettings::from_value(Some(&bare)).thesaurus.as_deref(),
            Some("/kg/t.json")
        );
        let nested = json!({"terraphim": {"thesaurus": "~/t.json"}});
        assert_eq!(
            ServerSettings::from_value(Some(&nested))
                .thesaurus
                .as_deref(),
            Some("~/t.json")
        );
        assert_eq!(ServerSettings::default().thesaurus, None);
    }

    #[test]
    fn unknown_terms_opt_in() {
        let value = json!({"terraphim": {"unknownTerms": true}});
        assert!(ServerSettings::from_value(Some(&value)).unknown_terms);
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
    fn bare_and_nested_keys_merge_with_nested_winning() {
        let mixed = json!({
            "thesaurus": "/kg.json",
            "inlayHints": true,
            "terraphim": {"unknownTerms": true, "inlayHints": false}
        });
        let settings = ServerSettings::from_value(Some(&mixed));
        assert_eq!(settings.thesaurus.as_deref(), Some("/kg.json"));
        assert!(settings.unknown_terms);
        assert!(!settings.inlay_hints, "nested wins");
        assert!(settings.ghost_diagnostics, "default kept");
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
