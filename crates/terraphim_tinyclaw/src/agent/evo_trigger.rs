//! Deterministic post-turn evolution trigger (#3228, T2).
//!
//! Port of AutoClaw's `evaluatePostTurn` heuristics: after each completed
//! turn the loop inspects per-turn signals (tool-call and tool-error counts,
//! preference language in the user message) and, when the configured
//! [`EvolutionIntensity`] admits it, invokes a proposer subagent bound to the
//! two-legal-outputs contract (`NOTHING_TO_SAVE` or a single proposal).
//! A proposer output of the second kind is emitted as an `evo.propose`
//! [`EngineEvent`] from the shared `terraphim_engine_events` vocabulary
//! (TACP spec 5.1, Desktop P1 freeze, #3232).
//!
//! Normative constraints honoured here:
//!
//! - **In-turn user veto binds at proposal time** (spec 5.1): if the user
//!   message for this turn contains veto language, no proposer invocation
//!   happens at all — the veto is evaluated before any proposal can exist,
//!   so it cannot be deferred to disposition.
//! - **Trust ladder**: heuristic triggers produce at most `L1` proposals
//!   (`L0`/`L1` only), so nothing here can modify behaviour-governing
//!   artefacts, which spec 5.1 reserves for `L3`.
//! - **Frozen boundary**: the ACP module (`crate::acp`) is a frozen
//!   Hermes-parity surface and is not touched; emission is a JSONL audit
//!   append under `<workspace>/.terraphim/evolution/proposals.jsonl`.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use terraphim_engine_events::{EngineEvent, EvolutionPropose, TargetKind, TrustLevel};

/// The exact string a proposer subagent must emit when nothing in the turn
/// is worth persisting (two-legal-outputs contract, output one).
pub const PROPOSER_NOTHING_TO_SAVE: &str = "NOTHING_TO_SAVE";

/// Strong preference keywords (AutoClaw port): an unambiguous durable
/// preference. Admitted at `cautious` intensity and above.
const STRONG_PREFERENCE_KEYWORDS: &[&str] = &[
    "always ",
    "never ",
    "from now on",
    "remember this",
    "remember that",
    "make sure you",
];

/// Weak preference keywords (AutoClaw port): a softer signal that only
/// `aggressive` intensity acts on.
const WEAK_PREFERENCE_KEYWORDS: &[&str] = &[
    "i prefer",
    "i usually",
    "i like",
    "i don't like",
    "i dislike",
    "tends to",
];

/// In-turn veto language (spec 5.1): binds at proposal time, so any veto
/// hit suppresses the trigger outright for this turn.
const VETO_KEYWORDS: &[&str] = &[
    "don't save",
    "do not save",
    "don't remember",
    "do not remember",
    "forget that",
    "forget this",
    "stop learning",
    "no learning",
];

/// Proposer intensity, ported from AutoClaw's `evolution.intensity` setting.
///
/// Default is `off`: the trigger is inert unless the operator opts in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvolutionIntensity {
    /// Never trigger. The default.
    #[default]
    Off,
    /// Trigger on strong signals only (strong keywords, thresholds).
    Cautious,
    /// Trigger on strong and weak signals, with lowered thresholds.
    Aggressive,
}

/// Configuration for the post-turn evolution trigger (`[evolution]`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct EvolutionConfig {
    /// Master switch. **Default: disabled.**
    pub enabled: bool,
    /// Proposer intensity (`off` | `cautious` | `aggressive`).
    pub intensity: EvolutionIntensity,
    /// Turns that must elapse after a proposal before another trigger is
    /// admitted (AutoClaw `cooldownTurns`).
    pub cooldown_turns: u32,
    /// Tool errors in a single turn at or above which a cautious trigger
    /// fires. Aggressive intensity triggers on a single error.
    pub tool_error_threshold: u32,
    /// Tool calls in a single turn at or above which a cautious trigger
    /// fires (a heavy multi-step procedure is a candidate durable
    /// workflow). Aggressive intensity halves this.
    pub tool_call_threshold: u32,
}

impl Default for EvolutionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            intensity: EvolutionIntensity::Off,
            cooldown_turns: 5,
            tool_error_threshold: 2,
            tool_call_threshold: 8,
        }
    }
}

/// Per-turn signals evaluated by [`evaluate_post_turn`].
#[derive(Debug, Clone, Default)]
pub struct PostTurnSignals {
    /// Tool calls executed during the turn.
    pub tool_calls: u32,
    /// Tool calls that returned an error during the turn.
    pub tool_errors: u32,
    /// The user message that opened the turn (checked for preference and
    /// veto language).
    pub user_text: String,
    /// The assistant's final response text (carried into the proposer
    /// prompt for context).
    pub assistant_text: String,
}

/// Why the heuristic admitted this turn to the proposer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerReason {
    /// Tool errors met the configured threshold.
    ToolErrors,
    /// Tool-call volume met the configured threshold.
    ToolCallVolume,
    /// Strong preference language in the user message.
    StrongPreference,
    /// Weak preference language (aggressive intensity only).
    WeakPreference,
}

/// A heuristic admission: the turn warrants a proposer invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TriggerHit {
    /// Why the turn was admitted.
    pub reason: TriggerReason,
    /// Excerpt of the signal source (keyword context or threshold summary)
    /// carried into the proposer prompt.
    pub evidence: String,
}

/// Cooldown bookkeeping for the trigger (AutoClaw `cooldownTurns`).
#[derive(Debug, Clone, Default)]
pub struct TriggerState {
    turns_since_proposal: u32,
}

impl TriggerState {
    /// Record a completed turn. Returns `true` when the cooldown has
    /// elapsed and a trigger may be admitted this turn.
    pub fn tick_and_check_cooldown(&mut self, cooldown_turns: u32) -> bool {
        self.turns_since_proposal = self.turns_since_proposal.saturating_add(1);
        self.turns_since_proposal > cooldown_turns
    }

    /// Record that a proposal was emitted; restarts the cooldown.
    pub fn record_proposal(&mut self) {
        self.turns_since_proposal = 0;
    }
}

/// True when `text` contains any `keywords` entry (case-insensitive).
fn contains_any(text: &str, keywords: &[&str]) -> Option<String> {
    let haystack = text.to_lowercase();
    keywords
        .iter()
        .find(|k| haystack.contains(**k))
        .map(|k| (*k).to_string())
}

/// The deterministic post-turn check (AutoClaw `evaluatePostTurn` port).
///
/// Pure: no I/O, no model involvement. Returns [`TriggerHit`] when the
/// turn should be handed to the proposer subagent, else `None`.
///
/// Evaluation order is normative: the veto is checked first (it binds at
/// proposal time), then intensity, then cooldown, then the signal
/// thresholds/keywords.
pub fn evaluate_post_turn(
    signals: &PostTurnSignals,
    config: &EvolutionConfig,
    state: &mut TriggerState,
) -> Option<TriggerHit> {
    // In-turn user veto binds at proposal time (spec 5.1): suppress the
    // trigger before any proposal can come into existence.
    if contains_any(&signals.user_text, VETO_KEYWORDS).is_some() {
        return None;
    }

    if !config.enabled || config.intensity == EvolutionIntensity::Off {
        return None;
    }

    if !state.tick_and_check_cooldown(config.cooldown_turns) {
        return None;
    }

    let aggressive = config.intensity == EvolutionIntensity::Aggressive;
    let error_threshold = if aggressive {
        1
    } else {
        config.tool_error_threshold
    };
    let call_threshold = if aggressive {
        config.tool_call_threshold.div_ceil(2)
    } else {
        config.tool_call_threshold
    };

    if signals.tool_errors >= error_threshold && signals.tool_errors > 0 {
        return Some(TriggerHit {
            reason: TriggerReason::ToolErrors,
            evidence: format!(
                "{} tool error(s) in turn (threshold {})",
                signals.tool_errors, error_threshold
            ),
        });
    }

    if signals.tool_calls >= call_threshold && signals.tool_calls > 0 {
        return Some(TriggerHit {
            reason: TriggerReason::ToolCallVolume,
            evidence: format!(
                "{} tool call(s) in turn (threshold {})",
                signals.tool_calls, call_threshold
            ),
        });
    }

    if let Some(keyword) = contains_any(&signals.user_text, STRONG_PREFERENCE_KEYWORDS) {
        return Some(TriggerHit {
            reason: TriggerReason::StrongPreference,
            evidence: format!("strong preference keyword `{keyword}`"),
        });
    }

    if aggressive && let Some(keyword) = contains_any(&signals.user_text, WEAK_PREFERENCE_KEYWORDS)
    {
        return Some(TriggerHit {
            reason: TriggerReason::WeakPreference,
            evidence: format!("weak preference keyword `{keyword}`"),
        });
    }

    None
}

/// The two legal outputs of the proposer subagent (spec: `NOTHING_TO_SAVE`
/// or a single proposal). Anything else is a contract violation.
#[derive(Debug, Clone, PartialEq)]
pub enum ProposerOutput {
    /// Output one: nothing in the turn is worth persisting.
    NothingToSave,
    /// Output two: a well-formed `evo.propose` payload.
    Proposal(Box<EvolutionPropose>),
}

/// Build the proposer subagent prompt. The contract is stated verbatim so
/// the subagent has exactly two legal outputs.
pub fn build_proposer_prompt(signals: &PostTurnSignals, hit: &TriggerHit) -> String {
    format!(
        "You are an evolution proposer subagent. A conversation turn has been \
         flagged by deterministic heuristics ({evidence}).\n\
         \n\
         ## Turn\n\
         User: {user}\n\
         Assistant: {assistant}\n\
         Tool calls: {calls}; tool errors: {errors}\n\
         \n\
         ## Contract (exactly two legal outputs)\n\
         1. `{nothing}` — when nothing in this turn is a durable preference, \
         correction, or workflow worth persisting.\n\
         2. A single JSON object with fields `signature` (stable kebab-case \
         dedup key), `target_kind` (`memory` | `behaviour` | `skill` | \
         `tool`), `target_ref` (string or null), `content` (section-scoped \
         merge content, never a wholesale replacement), `trust_level` \
         (`L0` or `L1`).\n\
         \n\
         Respond with one of the two legal outputs and nothing else.",
        evidence = hit.evidence,
        user = signals.user_text,
        assistant = signals.assistant_text,
        calls = signals.tool_calls,
        errors = signals.tool_errors,
        nothing = PROPOSER_NOTHING_TO_SAVE,
    )
}

/// Parse the proposer subagent's raw output against the two-legal-outputs
/// contract. Leading/trailing whitespace is tolerated; anything beyond the
/// two legal outputs is an error.
pub fn parse_proposer_output(raw: &str) -> anyhow::Result<ProposerOutput> {
    let trimmed = raw.trim();
    if trimmed == PROPOSER_NOTHING_TO_SAVE {
        return Ok(ProposerOutput::NothingToSave);
    }

    let propose: EvolutionPropose = serde_json::from_str(trimmed).map_err(|e| {
        anyhow::anyhow!(
            "proposer contract violation: output is neither `{PROPOSER_NOTHING_TO_SAVE}` \
             nor a valid evo.propose payload: {e}"
        )
    })?;

    // The heuristic trigger may only originate L0/L1 proposals; anything
    // claiming L2+ is rejected here rather than at disposition time.
    // (Wire form is uppercase `L0`..`L3` per the golden vectors.)
    // TACP spec 5.1: heuristic -> L0/L1 only; L2 requires evidence criteria
    // (applied_count>=3, agent_count>=2); L3 is human-only.
    if !matches!(propose.trust_level, TrustLevel::L0 | TrustLevel::L1) {
        anyhow::bail!(
            "proposer contract violation: trust_level must be l0 or l1, got {:?}",
            propose.trust_level
        );
    }

    // TACP spec 5.1 constraint 2: behaviour-governing artefacts are L3-only
    // and L3 is human-only, so a heuristic-triggered proposer capped at
    // L0/L1 can never legitimately propose a `behaviour` target. Reject it
    // here so a buggy proposer cannot smuggle one through the parser.
    if matches!(propose.target_kind, TargetKind::Behaviour) {
        anyhow::bail!(
            "proposer contract violation: target_kind `behaviour` requires L3 \
             (human-only); heuristic proposals are capped at L0/L1"
        );
    }

    Ok(ProposerOutput::Proposal(Box::new(propose)))
}

/// Append an `evo.propose` event to the workspace audit sink
/// (`<workspace>/.terraphim/evolution/proposals.jsonl`), one serialised
/// [`EngineEvent`] per line. Creates the directory on first use.
///
/// This is the emission boundary: the frozen ACP module is not involved,
/// and the JSONL file is the durable record a later gate (approve/reject)
/// consumes.
pub fn append_proposal(workspace: &Path, propose: &EvolutionPropose) -> std::io::Result<PathBuf> {
    use std::io::Write;

    let dir = workspace.join(".terraphim").join("evolution");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("proposals.jsonl");

    let event = EngineEvent::EvolutionProposed(propose.clone());
    let line = serde_json::to_string(&event)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    writeln!(file, "{line}")?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use terraphim_engine_events::TargetKind;

    fn signals(user_text: &str) -> PostTurnSignals {
        PostTurnSignals {
            tool_calls: 0,
            tool_errors: 0,
            user_text: user_text.to_string(),
            assistant_text: "acknowledged".to_string(),
        }
    }

    fn cautious_config() -> EvolutionConfig {
        EvolutionConfig {
            enabled: true,
            intensity: EvolutionIntensity::Cautious,
            ..Default::default()
        }
    }

    fn cooled_state() -> TriggerState {
        // A state whose cooldown has long elapsed.
        let mut state = TriggerState::default();
        for _ in 0..100 {
            state.tick_and_check_cooldown(5);
        }
        state
    }

    #[test]
    fn disabled_config_never_triggers() {
        let mut state = cooled_state();
        let config = EvolutionConfig::default(); // enabled: false
        let mut sig = signals("always use rg for search");
        sig.tool_errors = 10;
        assert_eq!(evaluate_post_turn(&sig, &config, &mut state), None);
    }

    #[test]
    fn intensity_off_never_triggers() {
        let mut state = cooled_state();
        let config = EvolutionConfig {
            enabled: true,
            intensity: EvolutionIntensity::Off,
            ..Default::default()
        };
        let mut sig = signals("never commit secrets");
        sig.tool_errors = 10;
        assert_eq!(evaluate_post_turn(&sig, &config, &mut state), None);
    }

    #[test]
    fn veto_binds_at_proposal_time() {
        // Even a strong preference keyword is suppressed by in-turn veto
        // language — the veto is evaluated before any proposal can exist.
        let mut state = cooled_state();
        let config = cautious_config();
        let sig = signals("always use rg — but don't save this");
        assert_eq!(evaluate_post_turn(&sig, &config, &mut state), None);

        let sig = signals("from now on run clippy first; do not remember this");
        assert_eq!(evaluate_post_turn(&sig, &config, &mut state), None);
    }

    #[test]
    fn cooldown_suppresses_then_readmits() {
        let config = cautious_config(); // cooldown_turns = 5
        let mut state = TriggerState::default();
        state.record_proposal();

        let sig = signals("always use rg for search");
        // Turns 1..=5 are inside the cooldown.
        for _ in 0..5 {
            assert_eq!(evaluate_post_turn(&sig, &config, &mut state), None);
        }
        // Turn 6 is past the cooldown.
        let hit = evaluate_post_turn(&sig, &config, &mut state);
        assert_eq!(hit.map(|h| h.reason), Some(TriggerReason::StrongPreference));
    }

    #[test]
    fn tool_error_threshold_per_intensity() {
        let mut sig = signals("run the build");
        sig.tool_errors = 1;

        // Cautious: one error is below the threshold of two.
        let mut state = cooled_state();
        assert_eq!(
            evaluate_post_turn(&sig, &cautious_config(), &mut state),
            None
        );

        // Aggressive: a single error admits the turn.
        let mut state = cooled_state();
        let aggressive = EvolutionConfig {
            enabled: true,
            intensity: EvolutionIntensity::Aggressive,
            ..Default::default()
        };
        let hit = evaluate_post_turn(&sig, &aggressive, &mut state);
        assert_eq!(hit.map(|h| h.reason), Some(TriggerReason::ToolErrors));
    }

    #[test]
    fn tool_call_volume_threshold_per_intensity() {
        let mut sig = signals("set up the project");
        sig.tool_calls = 5;

        // Cautious: five calls are below the threshold of eight.
        let mut state = cooled_state();
        assert_eq!(
            evaluate_post_turn(&sig, &cautious_config(), &mut state),
            None
        );

        // Aggressive: threshold halves to four; five calls admit the turn.
        let mut state = cooled_state();
        let aggressive = EvolutionConfig {
            enabled: true,
            intensity: EvolutionIntensity::Aggressive,
            ..Default::default()
        };
        let hit = evaluate_post_turn(&sig, &aggressive, &mut state);
        assert_eq!(hit.map(|h| h.reason), Some(TriggerReason::ToolCallVolume));
    }

    #[test]
    fn strong_keyword_triggers_at_cautious() {
        let mut state = cooled_state();
        let sig = signals("From now on, prefer nextest over cargo test.");
        let hit = evaluate_post_turn(&sig, &cautious_config(), &mut state);
        assert_eq!(hit.map(|h| h.reason), Some(TriggerReason::StrongPreference));
    }

    #[test]
    fn weak_keyword_only_at_aggressive() {
        let sig = signals("I prefer British English in docs.");

        let mut state = cooled_state();
        assert_eq!(
            evaluate_post_turn(&sig, &cautious_config(), &mut state),
            None
        );

        let mut state = cooled_state();
        let aggressive = EvolutionConfig {
            enabled: true,
            intensity: EvolutionIntensity::Aggressive,
            ..Default::default()
        };
        let hit = evaluate_post_turn(&sig, &aggressive, &mut state);
        assert_eq!(hit.map(|h| h.reason), Some(TriggerReason::WeakPreference));
    }

    #[test]
    fn proposer_prompt_states_two_legal_outputs() {
        let sig = signals("always use rg");
        let hit = TriggerHit {
            reason: TriggerReason::StrongPreference,
            evidence: "strong preference keyword `always `".to_string(),
        };
        let prompt = build_proposer_prompt(&sig, &hit);
        assert!(prompt.contains(PROPOSER_NOTHING_TO_SAVE));
        assert!(prompt.contains("exactly two legal outputs"));
        assert!(prompt.contains("always use rg"));
    }

    #[test]
    fn parse_nothing_to_save_exact_and_whitespace_tolerant() {
        assert_eq!(
            parse_proposer_output("NOTHING_TO_SAVE").unwrap(),
            ProposerOutput::NothingToSave
        );
        assert_eq!(
            parse_proposer_output("  NOTHING_TO_SAVE\n").unwrap(),
            ProposerOutput::NothingToSave
        );
    }

    #[test]
    fn parse_valid_proposal_roundtrips() {
        let raw = r#"{
            "signature": "tools-prefer-nextest",
            "target_kind": "tool",
            "target_ref": null,
            "content": "Prefer `cargo nextest run` over `cargo test` for workspace runs.",
            "trust_level": "L1"
        }"#;
        match parse_proposer_output(raw).unwrap() {
            ProposerOutput::Proposal(p) => {
                assert_eq!(p.signature, "tools-prefer-nextest");
                assert_eq!(p.target_kind, TargetKind::Tool);
                assert_eq!(p.trust_level, TrustLevel::L1);
            }
            other => panic!("expected proposal, got {other:?}"),
        }
    }

    #[test]
    fn parse_rejects_contract_violations() {
        // Neither legal output.
        assert!(parse_proposer_output("I think you should save this").is_err());
        // Valid JSON but not an evo.propose payload.
        assert!(parse_proposer_output(r#"{"foo": 1}"#).is_err());
        // Trust level above the heuristic ceiling (L2) is rejected.
        let over_trusted = r#"{
            "signature": "behaviour-change",
            "target_kind": "behaviour",
            "target_ref": null,
            "content": "x",
            "trust_level": "L2"
        }"#;
        assert!(parse_proposer_output(over_trusted).is_err());
        // Behaviour targets are L3-only (human gate); rejected even at L1.
        let behaviour_at_l1 = r#"{
            "signature": "behaviour-change",
            "target_kind": "behaviour",
            "target_ref": null,
            "content": "x",
            "trust_level": "L1"
        }"#;
        assert!(parse_proposer_output(behaviour_at_l1).is_err());
    }

    #[test]
    fn append_proposal_writes_valid_jsonl() {
        let dir = std::env::temp_dir().join(format!("evo-trigger-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);

        let propose = EvolutionPropose {
            signature: "memory-prefer-british-english".to_string(),
            target_kind: TargetKind::Memory,
            target_ref: None,
            content: "Docs and comments use British English.".to_string(),
            trust_level: TrustLevel::L1,
        };

        let path = append_proposal(&dir, &propose).unwrap();
        append_proposal(&dir, &propose).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2);
        for line in lines {
            let event: EngineEvent = serde_json::from_str(line).unwrap();
            assert_eq!(event.message_type(), "evo.propose");
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn config_toml_roundtrip_with_defaults() {
        let toml_str = r#"
enabled = true
intensity = "aggressive"
"#;
        let config: EvolutionConfig = toml::from_str(toml_str).unwrap();
        assert!(config.enabled);
        assert_eq!(config.intensity, EvolutionIntensity::Aggressive);
        // Unspecified fields take the AutoClaw-ported defaults.
        assert_eq!(config.cooldown_turns, 5);
        assert_eq!(config.tool_error_threshold, 2);
        assert_eq!(config.tool_call_threshold, 8);

        let default_config: EvolutionConfig = toml::from_str("").unwrap();
        assert!(!default_config.enabled);
        assert_eq!(default_config.intensity, EvolutionIntensity::Off);
    }
}
