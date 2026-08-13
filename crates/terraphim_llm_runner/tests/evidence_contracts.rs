use terraphim_llm_runner::{
    NativeFailureEvidence, NativeFailureEvidenceError, NativeFailureEvidenceInput, NativeVerdict,
};

fn valid_input() -> NativeFailureEvidenceInput {
    NativeFailureEvidenceInput {
        owner: "terraphim".to_string(),
        repo: "terraphim-ai".to_string(),
        commit_sha: "0123456789abcdef0123456789abcdef01234567".to_string(),
        run_id: 1,
        job_id: 2,
        failing_step: Some("cargo test".to_string()),
        verdict: NativeVerdict::Failure,
        redacted_log_tail: "test failure without credentials".to_string(),
        max_evidence_bytes: 1024,
    }
}

#[test]
fn accepts_valid_failure_evidence() {
    let evidence = NativeFailureEvidence::validate(valid_input()).expect("valid evidence");

    assert_eq!(evidence.owner(), "terraphim");
    assert_eq!(evidence.repo(), "terraphim-ai");
    assert_eq!(
        evidence.commit_sha(),
        "0123456789abcdef0123456789abcdef01234567"
    );
    assert_eq!(evidence.run_id(), 1);
    assert_eq!(evidence.job_id(), 2);
    assert_eq!(evidence.failing_step(), Some("cargo test"));
    assert_eq!(evidence.verdict(), &NativeVerdict::Failure);
    assert_eq!(
        evidence.redacted_log_tail(),
        "test failure without credentials"
    );
    assert_eq!(evidence.evidence_digest().len(), 64);
}

#[test]
fn rejects_invalid_commit_sha_forms() {
    for sha in [
        "0123456789ABCDEF0123456789abcdef01234567",
        "0123456789abcdef0123456789abcdef0123456",
        "0123456789abcdef0123456789abcdef0123456g",
    ] {
        let mut input = valid_input();
        input.commit_sha = sha.to_string();

        let error = NativeFailureEvidence::validate(input).expect_err("invalid sha rejected");

        assert_eq!(error, NativeFailureEvidenceError::InvalidCommitSha);
        assert!(!format!("{error}").contains(sha));
        assert!(!format!("{error:?}").contains(sha));
    }
}

#[test]
fn rejects_non_failure_verdicts() {
    for verdict in [
        NativeVerdict::Success,
        NativeVerdict::Pending,
        NativeVerdict::Other("cancelled".to_string()),
    ] {
        let mut input = valid_input();
        input.verdict = verdict;

        let error =
            NativeFailureEvidence::validate(input).expect_err("non-failure verdict rejected");

        assert_eq!(error, NativeFailureEvidenceError::NonFailureVerdict);
        assert!(!format!("{error}").contains("cancelled"));
        assert!(!format!("{error:?}").contains("cancelled"));
    }
}

#[test]
fn trims_and_accepts_safe_identity_strings() {
    let mut input = valid_input();
    input.owner = " terraphim ".to_string();
    input.repo = "\tterraphim-ai\n".to_string();

    let evidence = NativeFailureEvidence::validate(input).expect("trimmed identities accepted");

    assert_eq!(evidence.owner(), "terraphim");
    assert_eq!(evidence.repo(), "terraphim-ai");
}

#[test]
fn rejects_invalid_identity_strings() {
    for (field, value, expected) in [
        ("owner", "", NativeFailureEvidenceError::InvalidOwner),
        ("owner", "   ", NativeFailureEvidenceError::InvalidOwner),
        (
            "owner",
            "../terraphim",
            NativeFailureEvidenceError::InvalidOwner,
        ),
        (
            "owner",
            "terra/phim",
            NativeFailureEvidenceError::InvalidOwner,
        ),
        (
            "owner",
            "terra\\phim",
            NativeFailureEvidenceError::InvalidOwner,
        ),
        (
            "owner",
            "terra\nphim",
            NativeFailureEvidenceError::InvalidOwner,
        ),
        (
            "owner",
            "terra phim",
            NativeFailureEvidenceError::InvalidOwner,
        ),
        ("repo", "", NativeFailureEvidenceError::InvalidRepo),
        ("repo", "   ", NativeFailureEvidenceError::InvalidRepo),
        (
            "repo",
            "../terraphim-ai",
            NativeFailureEvidenceError::InvalidRepo,
        ),
        (
            "repo",
            "terraphim/ai",
            NativeFailureEvidenceError::InvalidRepo,
        ),
        (
            "repo",
            "terraphim\\ai",
            NativeFailureEvidenceError::InvalidRepo,
        ),
        (
            "repo",
            "terraphim\nai",
            NativeFailureEvidenceError::InvalidRepo,
        ),
        (
            "repo",
            "terraphim ai",
            NativeFailureEvidenceError::InvalidRepo,
        ),
    ] {
        let mut input = valid_input();
        match field {
            "owner" => input.owner = value.to_string(),
            "repo" => input.repo = value.to_string(),
            _ => unreachable!("test field is known"),
        }

        let error = NativeFailureEvidence::validate(input).expect_err("identity rejected");

        assert_eq!(error, expected);
        if !value.is_empty() {
            assert!(!format!("{error}").contains(value));
            assert!(!format!("{error:?}").contains(value));
        }
    }
}

#[test]
fn rejects_non_positive_ids() {
    let mut input = valid_input();
    input.run_id = 0;
    let error = NativeFailureEvidence::validate(input).expect_err("zero run id rejected");
    assert_eq!(error, NativeFailureEvidenceError::InvalidRunId);

    let mut input = valid_input();
    input.job_id = 0;
    let error = NativeFailureEvidence::validate(input).expect_err("zero job id rejected");
    assert_eq!(error, NativeFailureEvidenceError::InvalidJobId);
}

#[test]
fn rejects_oversized_log_tail_without_truncating() {
    let mut input = valid_input();
    input.redacted_log_tail = "abcd".to_string();
    input.max_evidence_bytes = 4;
    let evidence = NativeFailureEvidence::validate(input).expect("limit is inclusive");
    assert_eq!(evidence.redacted_log_tail(), "abcd");

    let mut input = valid_input();
    input.redacted_log_tail = "abcde".to_string();
    input.max_evidence_bytes = 4;
    let error = NativeFailureEvidence::validate(input).expect_err("oversized evidence rejected");

    assert_eq!(error, NativeFailureEvidenceError::EvidenceTooLarge);
    assert!(!format!("{error}").contains("abcde"));
    assert!(!format!("{error:?}").contains("abcde"));
}

#[test]
fn rejects_common_unredacted_secret_patterns() {
    for log in [
        "Authorization: Bearer sk-live-secret",
        "authorization: token ghp_secret",
        "GITEA_TOKEN=secret-value",
        "openai_api_key = sk-secret",
        "loaded reference op://Vault/Item/password",
        "-----BEGIN PRIVATE KEY-----\nsecret\n-----END PRIVATE KEY-----",
        "-----BEGIN RSA PRIVATE KEY-----\nsecret\n-----END RSA PRIVATE KEY-----",
    ] {
        let mut input = valid_input();
        input.redacted_log_tail = log.to_string();

        let error = NativeFailureEvidence::validate(input).expect_err("secret rejected");

        assert_eq!(error, NativeFailureEvidenceError::UnredactedSecret);
        assert!(!format!("{error}").contains(log));
        assert!(!format!("{error:?}").contains(log));
    }
}

#[test]
fn accepts_already_redacted_private_key_evidence() {
    let mut input = valid_input();
    input.redacted_log_tail = "[REDACTED PRIVATE KEY]".to_string();

    let evidence = NativeFailureEvidence::validate(input).expect("redacted evidence accepted");

    assert_eq!(evidence.redacted_log_tail(), "[REDACTED PRIVATE KEY]");
}

#[test]
fn input_debug_redacts_secret_bearing_contract_fields() {
    let mut input = valid_input();
    input.redacted_log_tail = "debug-input-sentinel-secret-log".to_string();
    input.verdict = NativeVerdict::Other("debug-input-sentinel-verdict".to_string());

    let debug = format!("{input:?}");

    assert!(debug.contains("owner: \"terraphim\""));
    assert!(debug.contains("redacted_log_tail: \"<redacted>\""));
    assert!(debug.contains("Other(\"<redacted>\")"));
    assert!(!debug.contains("debug-input-sentinel-secret-log"));
    assert!(!debug.contains("debug-input-sentinel-verdict"));
}

#[test]
fn evidence_debug_redacts_secret_bearing_contract_fields() {
    let mut input = valid_input();
    input.redacted_log_tail = "debug-evidence-sentinel-secret-log".to_string();
    let evidence = NativeFailureEvidence::validate(input).expect("valid evidence");

    let debug = format!("{evidence:?}");

    assert!(debug.contains("owner: \"terraphim\""));
    assert!(debug.contains("redacted_log_tail: \"<redacted>\""));
    assert!(!debug.contains("debug-evidence-sentinel-secret-log"));
}

#[test]
fn evidence_digest_is_stable_for_identical_input() {
    let left = NativeFailureEvidence::validate(valid_input()).expect("left evidence");
    let right = NativeFailureEvidence::validate(valid_input()).expect("right evidence");

    assert_eq!(left.evidence_digest(), right.evidence_digest());
}

#[test]
fn evidence_digest_changes_when_semantic_fields_change() {
    let baseline = NativeFailureEvidence::validate(valid_input())
        .expect("baseline evidence")
        .evidence_digest()
        .to_string();

    let mut cases = Vec::new();

    let mut input = valid_input();
    input.owner = "other-owner".to_string();
    cases.push(input);

    let mut input = valid_input();
    input.repo = "other-repo".to_string();
    cases.push(input);

    let mut input = valid_input();
    input.commit_sha = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string();
    cases.push(input);

    let mut input = valid_input();
    input.run_id = 99;
    cases.push(input);

    let mut input = valid_input();
    input.job_id = 100;
    cases.push(input);

    let mut input = valid_input();
    input.failing_step = None;
    cases.push(input);

    let mut input = valid_input();
    input.failing_step = Some(String::new());
    cases.push(input);

    let mut input = valid_input();
    input.redacted_log_tail = "different failure output".to_string();
    cases.push(input);

    for input in cases {
        let changed = NativeFailureEvidence::validate(input)
            .expect("changed evidence")
            .evidence_digest()
            .to_string();
        assert_ne!(changed, baseline);
    }

    let mut none_step = valid_input();
    none_step.failing_step = None;
    let none_digest = NativeFailureEvidence::validate(none_step)
        .expect("none step evidence")
        .evidence_digest()
        .to_string();

    let mut empty_step = valid_input();
    empty_step.failing_step = Some(String::new());
    let empty_digest = NativeFailureEvidence::validate(empty_step)
        .expect("empty step evidence")
        .evidence_digest()
        .to_string();

    assert_ne!(none_digest, empty_digest);
}
