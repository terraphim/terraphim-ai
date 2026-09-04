//! Smoke tests for the platform-runtime vocabulary.

use std::time::Duration;

use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    Artifact, ArtifactKind, CacheKind, ExecutionError, ExecutionResult, ExecutorBackend,
    FailureEvent, FailureKind, PolicyDecision, PolicyError, Route, RunnerKind, SuggestedAction,
    TrustTier, ValidatorError, Workspace, WorkspaceMode,
};

fn round_trip<T>(value: &T) -> T
where
    T: Serialize + for<'de> Deserialize<'de> + std::fmt::Debug,
{
    let json = serde_json::to_string(value).expect("serialisation should succeed");
    serde_json::from_str(&json).expect("deserialisation should succeed")
}

#[test]
fn failure_kind_round_trips() {
    for kind in [
        FailureKind::Ubs,
        FailureKind::Rch,
        FailureKind::Kache,
        FailureKind::Runner,
        FailureKind::Workflow,
        FailureKind::Policy,
    ] {
        assert_eq!(round_trip(&kind), kind);
    }
}

#[test]
fn suggested_action_round_trips() {
    for action in [
        SuggestedAction::FixCode,
        SuggestedAction::Suppress {
            justification: "false positive".into(),
        },
        SuggestedAction::PromoteToGolden,
        SuggestedAction::Reroute,
        SuggestedAction::EscalateToLlm,
        SuggestedAction::Defer,
    ] {
        assert_eq!(round_trip(&action), action);
    }
}

#[test]
fn failure_event_round_trips() {
    let event = FailureEvent {
        kind: FailureKind::Ubs,
        signature: "ubs-null-deref".into(),
        action: SuggestedAction::FixCode,
        context: json!({"file": "src/lib.rs", "line": 42}),
        workspace: "ws-001".into(),
        timestamp: Timestamp::now(),
    };
    let back = round_trip(&event);
    assert_eq!(back.kind, event.kind);
    assert_eq!(back.signature, event.signature);
    assert_eq!(back.action, event.action);
    assert_eq!(back.context, event.context);
    assert_eq!(back.workspace, event.workspace);
    assert_eq!(back.timestamp, event.timestamp);
}

#[test]
fn route_round_trips() {
    for route in [Route::Host, Route::Rch, Route::Firecracker, Route::Ubs] {
        assert_eq!(round_trip(&route), route);
    }
}

#[test]
fn cache_kind_round_trips() {
    for kind in [CacheKind::Auto, CacheKind::Kache, CacheKind::Sccache, CacheKind::None] {
        assert_eq!(round_trip(&kind), kind);
    }
}

#[test]
fn runner_kind_round_trips() {
    for kind in [RunnerKind::Local, RunnerKind::Gitea, RunnerKind::Github] {
        assert_eq!(round_trip(&kind), kind);
    }
}

#[test]
fn trust_tier_round_trips() {
    for tier in [TrustTier::L0, TrustTier::L1, TrustTier::L2, TrustTier::L3] {
        assert_eq!(round_trip(&tier), tier);
    }
}

#[test]
fn trust_tier_promote_is_monotonic_and_saturating() {
    assert_eq!(TrustTier::L0.promote(), TrustTier::L1);
    assert_eq!(TrustTier::L1.promote(), TrustTier::L2);
    assert_eq!(TrustTier::L2.promote(), TrustTier::L3);
    assert_eq!(TrustTier::L3.promote(), TrustTier::L3);
}

#[test]
fn trust_tier_demote_is_monotonic_and_saturating() {
    assert_eq!(TrustTier::L3.demote(), TrustTier::L2);
    assert_eq!(TrustTier::L2.demote(), TrustTier::L1);
    assert_eq!(TrustTier::L1.demote(), TrustTier::L0);
    assert_eq!(TrustTier::L0.demote(), TrustTier::L0);
}

#[test]
fn workspace_round_trips() {
    let workspace = Workspace {
        root: "/tmp/project".into(),
        hash: "abc123".into(),
        mode: WorkspaceMode::RsyncMirror,
    };
    assert_eq!(round_trip(&workspace), workspace);
}

#[test]
fn artifact_round_trips() {
    let artifact = Artifact {
        path: "/tmp/project/target/debug/app".into(),
        content_hash: "deadbeef".into(),
        kind: ArtifactKind::BinaryExecutable,
    };
    assert_eq!(round_trip(&artifact), artifact);
}

#[test]
fn execution_result_round_trips() {
    let result = ExecutionResult {
        exit_code: 1,
        duration: Duration::from_millis(1234),
        stdout_tail: "ok".into(),
        stderr_tail: "error".into(),
    };
    assert_eq!(round_trip(&result), result);
}

#[test]
fn policy_decision_does_not_derive_serialize() {
    // PolicyDecision is intentionally not serialisable: it is a runtime plan,
    // not a persisted event. This test just constructs one to keep the type in
    // use.
    let decision = PolicyDecision {
        route: Route::Rch,
        trust: TrustTier::L2,
        rationale: "cargo-heavy command".into(),
    };
    assert_eq!(decision.route, Route::Rch);
    assert_eq!(decision.trust, TrustTier::L2);
}

/// A no-op backend used only to prove that [`ExecutorBackend`] is object-safe.
struct DummyBackend;

#[async_trait::async_trait]
impl ExecutorBackend for DummyBackend {
    async fn execute(
        &self,
        _command: &str,
        _workspace: &Workspace,
    ) -> Result<ExecutionResult, ExecutionError> {
        Err(ExecutionError::BackendUnavailable("test".into()))
    }
}

#[tokio::test]
async fn executor_backend_trait_object_is_safe() {
    let backend: Box<dyn ExecutorBackend> = Box::new(DummyBackend);
    let workspace = Workspace {
        root: "/tmp".into(),
        hash: "0".into(),
        mode: WorkspaceMode::HostPath,
    };
    let result = backend.execute("true", &workspace).await;
    assert!(matches!(result, Err(ExecutionError::BackendUnavailable(_))));
}

#[test]
fn error_variants_display() {
    assert!(!PolicyError::Disallowed("rm -rf /".into()).to_string().is_empty());
    assert!(!ValidatorError::Validation("bad".into()).to_string().is_empty());
    assert!(!ExecutionError::Timeout.to_string().is_empty());
}
