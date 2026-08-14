use terraphim_llm_runner::{
    NativeFailureEvidence, NativeFailureEvidenceInput, NativeVerdict, Probe, ProbeExecutionLimits,
};

fn evidence() -> NativeFailureEvidence {
    NativeFailureEvidence::validate(NativeFailureEvidenceInput {
        owner: "terraphim".to_string(),
        repo: "terraphim-ai".to_string(),
        commit_sha: "0123456789abcdef0123456789abcdef01234567".to_string(),
        run_id: 1,
        job_id: 2,
        failing_step: Some("cargo test".to_string()),
        verdict: NativeVerdict::Failure,
        redacted_log_tail: "compile failed".to_string(),
        max_evidence_bytes: 1024,
    })
    .expect("valid evidence")
}

#[test]
fn probe_variants_are_closed_to_read_only_executables() {
    assert_eq!(
        Probe::CargoMetadataNoDeps.identity(),
        "cargo-metadata-no-deps"
    );
    assert_eq!(Probe::GitDiffCheck.identity(), "git-diff-check");
}

#[test]
fn probe_execution_limits_are_positive_and_bounded() {
    let limits = ProbeExecutionLimits::new(10_000, 32_768).expect("valid limits");

    assert_eq!(limits.timeout_ms(), 10_000);
    assert_eq!(limits.max_output_bytes(), 32_768);
    assert!(ProbeExecutionLimits::new(0, 32_768).is_err());
    assert!(ProbeExecutionLimits::new(10_000, 0).is_err());
    assert!(ProbeExecutionLimits::new(ProbeExecutionLimits::MAX_TIMEOUT_MS + 1, 32_768).is_err());
    assert!(ProbeExecutionLimits::new(10_000, ProbeExecutionLimits::MAX_OUTPUT_BYTES + 1).is_err());
}

#[test]
fn evidence_helper_requires_validated_evidence_type() {
    let _ = evidence();
}
