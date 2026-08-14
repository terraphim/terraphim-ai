use terraphim_llm_runner::{
    Diagnosis, DiagnosisKind, NativeFailureEvidence, NativeFailureEvidenceInput, NativeVerdict,
    RemediationSuggestion, StepName,
};

fn evidence_with_step(step: Option<&str>, log: &str) -> NativeFailureEvidence {
    NativeFailureEvidence::validate(NativeFailureEvidenceInput {
        owner: "terraphim".to_string(),
        repo: "terraphim-ai".to_string(),
        commit_sha: "0123456789abcdef0123456789abcdef01234567".to_string(),
        run_id: 7,
        job_id: 8,
        failing_step: step.map(str::to_string),
        verdict: NativeVerdict::Failure,
        redacted_log_tail: log.to_string(),
        max_evidence_bytes: 4096,
    })
    .expect("valid evidence")
}

#[test]
fn diagnosis_detects_compile_test_clippy_and_diff_rules_from_validated_evidence() {
    let compile = Diagnosis::from_evidence_and_probes(
        &evidence_with_step(Some("cargo build"), "compiler error"),
        &[],
    );
    assert_eq!(compile.kind(), &DiagnosisKind::CompileFailure);

    let test = Diagnosis::from_evidence_and_probes(
        &evidence_with_step(Some("cargo test"), "test error"),
        &[],
    );
    assert_eq!(test.kind(), &DiagnosisKind::TestFailure);

    let clippy = Diagnosis::from_evidence_and_probes(
        &evidence_with_step(Some("cargo clippy"), "lint error"),
        &[],
    );
    assert_eq!(clippy.kind(), &DiagnosisKind::ClippyFailure);

    let diff = Diagnosis::from_evidence_and_probes(
        &evidence_with_step(Some("git diff --check"), "diff error"),
        &[],
    );
    assert_eq!(diff.kind(), &DiagnosisKind::CheckoutOrDiffIssue);
}

#[test]
fn diagnosis_falls_back_to_evidence_only_unknown() {
    let diagnosis = Diagnosis::from_evidence_and_probes(
        &evidence_with_step(Some("unknown native step"), "failure without probe match"),
        &[],
    );

    assert_eq!(diagnosis.kind(), &DiagnosisKind::EvidenceOnlyUnknown);
    assert_eq!(
        diagnosis.evidence_digest(),
        "2c16cd3a8997b2de84fb41ae1feeae91c46a66bb44c7bec34b8e2e209422e487"
    );
}

#[test]
fn diagnosis_digest_is_stable_and_summary_is_bounded() {
    let evidence = evidence_with_step(
        Some("cargo test"),
        "0123456789abcdefghijklmnopqrstuvwxyz repeated diagnostic text",
    );
    let first = Diagnosis::from_evidence_and_probes(&evidence, &[]);
    let second = Diagnosis::from_evidence_and_probes(&evidence, &[]);

    assert_eq!(first.diagnosis_digest(), second.diagnosis_digest());
    assert!(first.summary().len() <= 160);
    assert!(first.summary().is_char_boundary(first.summary().len()));
    assert!(!first.summary().contains("native stdout content"));

    let debug = format!("{first:?}");
    assert!(debug.contains("summary: \"<redacted>\""));
    assert!(!debug.contains("repeated diagnostic text"));
    assert!(!debug.contains("native stderr content"));
}

#[test]
fn remediation_suggestions_are_inert_serializable_data() {
    let inspect = RemediationSuggestion::inspect_step("cargo test").expect("valid step");
    let expected = vec![
        RemediationSuggestion::ReRunNativeJob,
        inspect.clone(),
        RemediationSuggestion::FixCompileErrors,
        RemediationSuggestion::FixFormatting,
    ];

    let encoded = serde_json::to_string(&expected).expect("serialize suggestions");
    let decoded: Vec<RemediationSuggestion> =
        serde_json::from_str(&encoded).expect("deserialize suggestions");

    assert_eq!(decoded, expected);
    assert_eq!(
        inspect,
        RemediationSuggestion::InspectStep {
            name: StepName::parse("cargo test").expect("valid step")
        }
    );

    let debug = format!("{inspect:?}");
    assert!(debug.contains("name: \"<redacted>\""));
    assert!(!debug.contains("cargo test"));
    assert!(!format!("{:?}", StepName::parse("cargo test").unwrap()).contains("cargo test"));
}

#[test]
fn remediation_step_name_is_validated_and_bounded() {
    for value in [
        "",
        " ",
        "line\nbreak",
        "tab\tbreak",
        "authorization: bearer secret",
        "gitea_token=secret",
        "openai_api_key=secret",
        "op://vault/item",
        "x".repeat(81).as_str(),
    ] {
        let error = RemediationSuggestion::inspect_step(value).expect_err("invalid step rejected");

        assert_eq!(
            error.to_string(),
            "step name must be a bounded printable diagnostic step"
        );
        if !value.is_empty() {
            assert!(!format!("{error:?}").contains(value));
        }
    }
}

#[test]
fn remediation_deserialization_rejects_invalid_step_names() {
    let encoded =
        r#"[{"InspectStep":{"name":"cargo test; ok"}},{"InspectStep":{"name":"bad\nstep"}}]"#;

    let error =
        serde_json::from_str::<Vec<RemediationSuggestion>>(encoded).expect_err("invalid step");

    assert!(
        error
            .to_string()
            .contains("bounded printable diagnostic step")
    );
}
