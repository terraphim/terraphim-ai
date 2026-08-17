//! Shared `EngineEvent` vocabulary for Terraphim engine surfaces.
//!
//! This crate is the Desktop P1 freeze of the shared TACP/`EngineEvent`
//! contract. The Zed and VS Code consumers (approved R1 alignment) pin their
//! compatibility sets against the types and golden serialisation vectors
//! defined here, so wire names and payload shapes are normative and must not
//! drift.
//!
//! The evolution lifecycle family (`evo.*`) is field-for-field faithful to
//! the Terraphim Agent Communication Protocol (TACP) specification,
//! section 5.1 (`terraphim/agent-communication-protocol`, issue #28, commit
//! dc31489):
//!
//! | Wire name     | Variant             | Payload type          |
//! |---------------|---------------------|-----------------------|
//! | `evo.propose` | [`EvolutionProposed`] | [`EvolutionPropose`] |
//! | `evo.approve` | [`EvolutionApproved`] | [`EvolutionApprove`] |
//! | `evo.reject`  | [`EvolutionRejected`] | [`EvolutionReject`]  |
//! | `evo.applied` | [`EvolutionApplied`]  | [`EvolutionApplied`] |
//!
//! Normative constraints from spec 5.1 honoured by this crate:
//!
//! 1. In-turn user veto binds at proposal time (a proposer concern; the
//!    vocabulary carries no field that could defer it to disposition).
//! 2. Only `L3` proposals may modify behaviour-governing artefacts. This is
//!    checkable via [`EvolutionPropose::is_behaviour_governing`] together with
//!    [`TrustLevel`]; enforcement belongs to the proposer/gate, not the wire.
//! 3. `evo.applied` MUST NOT be emitted before a matching `evo.approve`.
//!    Enforced in the type system: [`EvolutionApplied`] has private fields and
//!    the only constructor, [`EvolutionApplied::from_approval`], requires an
//!    [`EvolutionApprove`] reference, from which the matching identity fields
//!    (`signature`, `target_kind`, `target_ref`, `trust_level`) are copied.
//!
//! `TrustLevel` is re-used from `terraphim_types::shared_learning` (the
//! shared-learning trust ladder, `L0`-`L3`); it is deliberately not
//! redeclared here so that all Terraphim crates share one type.

use serde::{Deserialize, Serialize};

pub use terraphim_types::shared_learning::TrustLevel;

/// Wire name of the `evo.propose` message type (TACP spec 5.1).
pub const EVO_PROPOSE_TYPE: &str = "evo.propose";
/// Wire name of the `evo.approve` message type (TACP spec 5.1).
pub const EVO_APPROVE_TYPE: &str = "evo.approve";
/// Wire name of the `evo.reject` message type (TACP spec 5.1).
pub const EVO_REJECT_TYPE: &str = "evo.reject";
/// Wire name of the `evo.applied` message type (TACP spec 5.1).
pub const EVO_APPLIED_TYPE: &str = "evo.applied";

/// Shared engine-event vocabulary.
///
/// Serialisation convention: internally tagged on `type`, with the variant
/// renamed to the exact dotted TACP message-type name, and snake_case payload
/// fields, matching the TACP spec 5.1 payload table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum EngineEvent {
    /// `evo.propose`: an agent proposes a durable change to memory,
    /// behaviour, a skill, or a tool preference.
    #[serde(rename = "evo.propose")]
    EvolutionProposed(EvolutionPropose),
    /// `evo.approve`: the human gate approves a proposal with a disposition.
    #[serde(rename = "evo.approve")]
    EvolutionApproved(EvolutionApprove),
    /// `evo.reject`: the human gate rejects a proposal with a disposition.
    #[serde(rename = "evo.reject")]
    EvolutionRejected(EvolutionReject),
    /// `evo.applied`: an approved change has been applied and audited.
    ///
    /// Can only be constructed from a matching [`EvolutionApprove`]; see
    /// [`EvolutionApplied::from_approval`].
    #[serde(rename = "evo.applied")]
    EvolutionApplied(EvolutionApplied),
}

impl EngineEvent {
    /// The dotted TACP message-type name for this event (spec 5.1).
    pub fn message_type(&self) -> &'static str {
        match self {
            EngineEvent::EvolutionProposed(_) => EVO_PROPOSE_TYPE,
            EngineEvent::EvolutionApproved(_) => EVO_APPROVE_TYPE,
            EngineEvent::EvolutionRejected(_) => EVO_REJECT_TYPE,
            EngineEvent::EvolutionApplied(_) => EVO_APPLIED_TYPE,
        }
    }
}

/// The kind of artefact an evolution proposal targets (spec 5.1
/// `target_kind`: `memory` | `behaviour` | `skill` | `tool`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetKind {
    /// Durable memory (notes, learnings, corrections).
    Memory,
    /// Behaviour-governing artefacts (rules, system prompts, policies).
    Behaviour,
    /// A skill definition.
    Skill,
    /// A tool preference or configuration.
    Tool,
}

/// Disposition recorded by the human gate on `evo.approve`/`evo.reject`
/// (spec 5.1: `allow_once` | `allow_always` | `reject` | `reject_always`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Disposition {
    /// Permit this change once.
    AllowOnce,
    /// Permit this and future matching changes.
    AllowAlways,
    /// Reject this change.
    Reject,
    /// Reject this and future matching changes.
    RejectAlways,
}

/// Payload of `evo.propose` (TACP spec 5.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvolutionPropose {
    /// Stable kebab-case identity of the proposal (dedup key), e.g.
    /// `tools-use-rg-for-search`.
    pub signature: String,
    /// The kind of artefact the change applies to.
    pub target_kind: TargetKind,
    /// The artefact the change applies to (file path, skill name, rule id).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_ref: Option<String>,
    /// Proposed content as a section-scoped merge, never a wholesale file
    /// replacement.
    pub content: String,
    /// Trust level of the proposal per the shared-learning trust ladder.
    pub trust_level: TrustLevel,
}

impl EvolutionPropose {
    /// True when the proposal targets a behaviour-governing artefact.
    ///
    /// Per spec 5.1 constraint 2, only `L3` proposals may modify such
    /// artefacts; `L1`/`L2` are limited to memory and preference targets.
    pub fn is_behaviour_governing(&self) -> bool {
        matches!(self.target_kind, TargetKind::Behaviour)
    }
}

/// Payload of `evo.approve` (TACP spec 5.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvolutionApprove {
    /// Stable kebab-case identity of the approved proposal.
    pub signature: String,
    /// The kind of artefact the change applies to.
    pub target_kind: TargetKind,
    /// The artefact the change applies to (file path, skill name, rule id).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_ref: Option<String>,
    /// Trust level at approval time.
    pub trust_level: TrustLevel,
    /// Disposition recorded by the human gate.
    pub disposition: Disposition,
}

/// Payload of `evo.reject` (TACP spec 5.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvolutionReject {
    /// Stable kebab-case identity of the rejected proposal.
    pub signature: String,
    /// The kind of artefact the change would have applied to.
    pub target_kind: TargetKind,
    /// The artefact the change would have applied to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_ref: Option<String>,
    /// Trust level at rejection time.
    pub trust_level: TrustLevel,
    /// Disposition recorded by the human gate.
    pub disposition: Disposition,
}

/// Payload of `evo.applied` (TACP spec 5.1).
///
/// Spec 5.1 constraint 3 (`evo.applied` MUST NOT be emitted before a
/// matching `evo.approve`) is enforced in the type system: the fields are
/// private and the only constructor is
/// [`from_approval`](EvolutionApplied::from_approval), which requires an
/// [`EvolutionApprove`] reference and copies the matching identity fields
/// from it. Direct construction is a compile error:
///
/// ```compile_fail
/// use terraphim_engine_events::EvolutionApplied;
/// // ERROR: fields of `EvolutionApplied` are private; an approval
/// // reference is required via `EvolutionApplied::from_approval`.
/// let applied = EvolutionApplied {
///     signature: "tools-use-rg-for-search".to_string(),
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvolutionApplied {
    /// Stable kebab-case identity of the applied proposal, copied from the
    /// matching `evo.approve`.
    signature: String,
    /// The kind of artefact the change applied to, copied from the approval.
    target_kind: TargetKind,
    /// The artefact the change applied to, copied from the approval.
    #[serde(skip_serializing_if = "Option::is_none")]
    target_ref: Option<String>,
    /// Trust level at application time, copied from the approval.
    trust_level: TrustLevel,
    /// Machine-checkable verification pattern for the applied change;
    /// absence is recorded as debt (spec 5.1: SHOULD).
    #[serde(skip_serializing_if = "Option::is_none")]
    verify: Option<String>,
    /// Reference to the audit record (daily note entry, evolution log,
    /// issue comment). Mandatory per spec 5.1.
    audit_ref: String,
}

impl EvolutionApplied {
    /// Construct an `evo.applied` payload from its matching approval.
    ///
    /// This is the only constructor: an approval reference is required, and
    /// the identity fields (`signature`, `target_kind`, `target_ref`,
    /// `trust_level`) are taken from it so the applied event always matches a
    /// prior `evo.approve` (spec 5.1 constraint 3).
    pub fn from_approval(
        approval: &EvolutionApprove,
        verify: Option<String>,
        audit_ref: String,
    ) -> Self {
        Self {
            signature: approval.signature.clone(),
            target_kind: approval.target_kind,
            target_ref: approval.target_ref.clone(),
            trust_level: approval.trust_level,
            verify,
            audit_ref,
        }
    }

    /// Stable kebab-case identity of the applied proposal.
    pub fn signature(&self) -> &str {
        &self.signature
    }

    /// The kind of artefact the change applied to.
    pub fn target_kind(&self) -> TargetKind {
        self.target_kind
    }

    /// The artefact the change applied to.
    pub fn target_ref(&self) -> Option<&str> {
        self.target_ref.as_deref()
    }

    /// Trust level at application time.
    pub fn trust_level(&self) -> TrustLevel {
        self.trust_level
    }

    /// Machine-checkable verification pattern, if provided.
    pub fn verify(&self) -> Option<&str> {
        self.verify.as_deref()
    }

    /// Reference to the audit record.
    pub fn audit_ref(&self) -> &str {
        &self.audit_ref
    }
}
