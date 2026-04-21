//! Field-mapping tests for the four [`OrchestratorEvent`] variants added in
//! ROC v1 Step I. All tests are gated behind the `quickwit` feature so they
//! compile and run only when the feature is enabled.
//!
//! Refs: terraphim/adf-fleet#37

#[cfg(feature = "quickwit")]
mod tests {
    use terraphim_orchestrator::quickwit::{OrchestratorEvent, QuickwitFleetSink, QuickwitSink};

    #[test]
    fn pr_reviewed_event_fields() {
        let event = OrchestratorEvent::PrReviewed {
            pr_number: 42,
            project: "odilo".to_string(),
            head_sha: "abc123def456".to_string(),
            reviewer_login: "pr-reviewer".to_string(),
            confidence: 4,
            p0_count: 0,
            p1_count: 1,
            verdict: "CONDITIONAL".to_string(),
        };
        let v = serde_json::to_value(&event).expect("serialisation must succeed");
        assert_eq!(v["event_kind"], "pr_reviewed");
        assert_eq!(v["pr_number"], 42u64);
        assert_eq!(v["project"], "odilo");
        assert_eq!(v["head_sha"], "abc123def456");
        assert_eq!(v["reviewer_login"], "pr-reviewer");
        assert_eq!(v["confidence"], 4u8);
        assert_eq!(v["p0_count"], 0u32);
        assert_eq!(v["p1_count"], 1u32);
        assert_eq!(v["verdict"], "CONDITIONAL");
    }

    #[test]
    fn pr_auto_merged_event_fields() {
        let event = OrchestratorEvent::PrAutoMerged {
            pr_number: 10,
            project: "terraphim".to_string(),
            merge_sha: "deadbeef1234".to_string(),
            title: "Fix: something important".to_string(),
        };
        let v = serde_json::to_value(&event).expect("serialisation must succeed");
        assert_eq!(v["event_kind"], "pr_auto_merged");
        assert_eq!(v["pr_number"], 10u64);
        assert_eq!(v["project"], "terraphim");
        assert_eq!(v["merge_sha"], "deadbeef1234");
        assert_eq!(v["title"], "Fix: something important");
    }

    #[test]
    fn pr_auto_merged_verified_event_fields() {
        let event = OrchestratorEvent::PrAutoMergedVerified {
            pr_number: 5,
            project: "project-x".to_string(),
            merge_sha: "cafebabe5678".to_string(),
            wall_time_secs: 42.5,
        };
        let v = serde_json::to_value(&event).expect("serialisation must succeed");
        assert_eq!(v["event_kind"], "pr_auto_merged_verified");
        assert_eq!(v["pr_number"], 5u64);
        assert_eq!(v["project"], "project-x");
        assert_eq!(v["merge_sha"], "cafebabe5678");
        assert!((v["wall_time_secs"].as_f64().unwrap() - 42.5).abs() < f64::EPSILON);
    }

    #[test]
    fn pr_auto_reverted_event_fields() {
        let event = OrchestratorEvent::PrAutoReverted {
            pr_number: 7,
            project: "fleet".to_string(),
            merge_sha: "merge001".to_string(),
            revert_sha: "revert002".to_string(),
            reason: "TestFailure".to_string(),
            stderr_tail_bytes: 2048,
        };
        let v = serde_json::to_value(&event).expect("serialisation must succeed");
        assert_eq!(v["event_kind"], "pr_auto_reverted");
        assert_eq!(v["pr_number"], 7u64);
        assert_eq!(v["merge_sha"], "merge001");
        assert_eq!(v["revert_sha"], "revert002");
        assert_eq!(v["reason"], "TestFailure");
        assert_eq!(v["stderr_tail_bytes"], 2048u32);
    }

    /// Emit to an unreachable endpoint: the channel send succeeds immediately
    /// and the background HTTP flush fails silently. Business logic must not
    /// be blocked or panic.
    #[tokio::test]
    async fn event_emit_tolerates_quickwit_down() {
        let sink = QuickwitSink::new(
            "http://127.0.0.1:1".to_string(),
            "test-index".to_string(),
            10,
            3600,
        );
        let fleet = QuickwitFleetSink::single(sink);
        let event = OrchestratorEvent::PrAutoMerged {
            pr_number: 1,
            project: "__global__".to_string(),
            merge_sha: "sha1".to_string(),
            title: "test PR".to_string(),
        };
        // Must complete without panic or block — channel send is non-blocking.
        fleet.emit_event("__global__", event).await;
    }
}
