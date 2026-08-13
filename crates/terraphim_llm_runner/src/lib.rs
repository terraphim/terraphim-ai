//! Native LLM runner library contracts.
//!
//! This crate exposes validation boundaries for native runner evidence before it
//! can be consumed by higher-level diagnosis or orchestration layers.
//!
//! Owner and repository identities are trimmed, must be non-empty, and may only
//! contain ASCII letters, ASCII digits, `.`, `_`, or `-`. The identity contract
//! rejects path separators, whitespace, control characters, and `..`.

use sha2::{Digest, Sha256};
use std::fmt;
use std::path::Path;
use terraphim_rlm::executor::strict_docker_diagnostics_sandbox as rlm_strict_docker_diagnostics_sandbox;
pub use terraphim_rlm::executor::{StrictDockerDiagnosticsSandbox, StrictDockerSandboxError};

/// Native CI verdicts accepted at the evidence boundary.
#[derive(Clone, Eq, PartialEq)]
pub enum NativeVerdict {
    /// The native CI job failed.
    Failure,
    /// The native CI job succeeded.
    Success,
    /// The native CI job has not reached a terminal state.
    Pending,
    /// Any native verdict not represented by a first-class variant.
    Other(String),
}

impl fmt::Debug for NativeVerdict {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Failure => formatter.write_str("Failure"),
            Self::Success => formatter.write_str("Success"),
            Self::Pending => formatter.write_str("Pending"),
            Self::Other(_) => formatter.debug_tuple("Other").field(&"<redacted>").finish(),
        }
    }
}

/// Construct the strict Docker-only diagnostics sandbox.
///
/// This companion factory delegates to RLM's opaque strict Docker constructor.
/// It does not expose the strict Docker profile, raw `HostConfig`, or inner
/// Docker executor.
///
/// # Errors
///
/// Returns an error when the checkout profile is invalid, Docker construction
/// fails, or the constructed backend is not Docker.
pub async fn strict_docker_diagnostics_sandbox(
    checkout_path: impl AsRef<Path>,
) -> Result<StrictDockerDiagnosticsSandbox, StrictDockerSandboxError> {
    rlm_strict_docker_diagnostics_sandbox(checkout_path).await
}

/// Candidate evidence supplied to [`NativeFailureEvidence::validate`].
#[derive(Clone, Eq, PartialEq)]
pub struct NativeFailureEvidenceInput {
    /// Forge owner identity.
    pub owner: String,
    /// Forge repository identity.
    pub repo: String,
    /// Forge commit SHA-1, currently expected as 40 lowercase hexadecimal chars.
    pub commit_sha: String,
    /// Native CI run identifier.
    pub run_id: u64,
    /// Native CI job identifier.
    pub job_id: u64,
    /// Optional failing step name reported by native CI.
    pub failing_step: Option<String>,
    /// Native CI verdict.
    pub verdict: NativeVerdict,
    /// Already-redacted log tail to use as evidence.
    pub redacted_log_tail: String,
    /// Maximum allowed evidence size in bytes.
    pub max_evidence_bytes: usize,
}

impl fmt::Debug for NativeFailureEvidenceInput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NativeFailureEvidenceInput")
            .field("owner", &self.owner)
            .field("repo", &self.repo)
            .field("commit_sha", &self.commit_sha)
            .field("run_id", &self.run_id)
            .field("job_id", &self.job_id)
            .field("failing_step", &self.failing_step)
            .field("verdict", &self.verdict)
            .field("redacted_log_tail", &"<redacted>")
            .field("max_evidence_bytes", &self.max_evidence_bytes)
            .finish()
    }
}

/// Validated native CI failure evidence.
#[derive(Clone, Eq, PartialEq)]
pub struct NativeFailureEvidence {
    owner: String,
    repo: String,
    commit_sha: String,
    run_id: u64,
    job_id: u64,
    failing_step: Option<String>,
    verdict: NativeVerdict,
    redacted_log_tail: String,
    evidence_digest: String,
}

impl fmt::Debug for NativeFailureEvidence {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NativeFailureEvidence")
            .field("owner", &self.owner)
            .field("repo", &self.repo)
            .field("commit_sha", &self.commit_sha)
            .field("run_id", &self.run_id)
            .field("job_id", &self.job_id)
            .field("failing_step", &self.failing_step)
            .field("verdict", &self.verdict)
            .field("redacted_log_tail", &"<redacted>")
            .field("evidence_digest", &self.evidence_digest)
            .finish()
    }
}

impl NativeFailureEvidence {
    /// Validate and construct native CI failure evidence.
    ///
    /// # Errors
    ///
    /// Returns an error when evidence violates the public contract.
    pub fn validate(
        mut input: NativeFailureEvidenceInput,
    ) -> Result<Self, NativeFailureEvidenceError> {
        input.owner = validate_identity(&input.owner, IdentityField::Owner)?;
        input.repo = validate_identity(&input.repo, IdentityField::Repo)?;

        if !is_lowercase_sha1(&input.commit_sha) {
            return Err(NativeFailureEvidenceError::InvalidCommitSha);
        }
        if input.run_id == 0 {
            return Err(NativeFailureEvidenceError::InvalidRunId);
        }
        if input.job_id == 0 {
            return Err(NativeFailureEvidenceError::InvalidJobId);
        }
        if input.redacted_log_tail.len() > input.max_evidence_bytes {
            return Err(NativeFailureEvidenceError::EvidenceTooLarge);
        }
        if contains_unredacted_secret(&input.redacted_log_tail) {
            return Err(NativeFailureEvidenceError::UnredactedSecret);
        }
        if input.verdict != NativeVerdict::Failure {
            return Err(NativeFailureEvidenceError::NonFailureVerdict);
        }

        let evidence_digest = compute_digest(&input);

        Ok(Self {
            owner: input.owner,
            repo: input.repo,
            commit_sha: input.commit_sha,
            run_id: input.run_id,
            job_id: input.job_id,
            failing_step: input.failing_step,
            verdict: input.verdict,
            redacted_log_tail: input.redacted_log_tail,
            evidence_digest,
        })
    }

    /// Forge owner identity.
    pub fn owner(&self) -> &str {
        &self.owner
    }

    /// Forge repository identity.
    pub fn repo(&self) -> &str {
        &self.repo
    }

    /// Forge commit SHA-1.
    pub fn commit_sha(&self) -> &str {
        &self.commit_sha
    }

    /// Native CI run identifier.
    pub fn run_id(&self) -> u64 {
        self.run_id
    }

    /// Native CI job identifier.
    pub fn job_id(&self) -> u64 {
        self.job_id
    }

    /// Optional failing step name.
    pub fn failing_step(&self) -> Option<&str> {
        self.failing_step.as_deref()
    }

    /// Immutable native verdict.
    pub fn verdict(&self) -> &NativeVerdict {
        &self.verdict
    }

    /// Redacted log tail accepted as evidence.
    pub fn redacted_log_tail(&self) -> &str {
        &self.redacted_log_tail
    }

    /// Deterministic digest over the accepted evidence fields.
    pub fn evidence_digest(&self) -> &str {
        &self.evidence_digest
    }
}

/// Safe validation errors for native failure evidence.
#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum NativeFailureEvidenceError {
    /// Owner was not a safe identity string.
    #[error("owner must be a non-empty safe identity string")]
    InvalidOwner,
    /// Repository was not a safe identity string.
    #[error("repo must be a non-empty safe identity string")]
    InvalidRepo,
    /// Commit SHA was not exactly 40 lowercase hexadecimal characters.
    #[error("commit SHA must be exactly 40 lowercase hexadecimal characters")]
    InvalidCommitSha,
    /// Native run id was not positive.
    #[error("run id must be positive")]
    InvalidRunId,
    /// Native job id was not positive.
    #[error("job id must be positive")]
    InvalidJobId,
    /// Evidence exceeded the configured byte limit.
    #[error("evidence exceeds configured byte limit")]
    EvidenceTooLarge,
    /// Evidence appears to contain an unredacted secret.
    #[error("evidence appears to contain an unredacted secret")]
    UnredactedSecret,
    /// The supplied native verdict was not a failure.
    #[error("native verdict must be failure")]
    NonFailureVerdict,
}

#[derive(Clone, Copy)]
enum IdentityField {
    Owner,
    Repo,
}

fn validate_identity(
    value: &str,
    field: IdentityField,
) -> Result<String, NativeFailureEvidenceError> {
    let trimmed = value.trim();
    let valid = !trimmed.is_empty()
        && !trimmed.contains("..")
        && trimmed
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'));

    if valid {
        Ok(trimmed.to_string())
    } else {
        Err(match field {
            IdentityField::Owner => NativeFailureEvidenceError::InvalidOwner,
            IdentityField::Repo => NativeFailureEvidenceError::InvalidRepo,
        })
    }
}

fn is_lowercase_sha1(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn contains_unredacted_secret(log: &str) -> bool {
    let lower = log.to_ascii_lowercase();

    (lower.contains("authorization:") && (lower.contains("bearer ") || lower.contains("token ")))
        || contains_assignment(&lower, "gitea_token")
        || contains_assignment(&lower, "openai_api_key")
        || lower.contains("op://")
        || (lower.contains("-----begin ") && lower.contains("private key-----"))
}

fn contains_assignment(log: &str, name: &str) -> bool {
    let mut remaining = log;
    while let Some(index) = remaining.find(name) {
        let after_name = &remaining[index + name.len()..];
        if after_name.trim_start().starts_with('=') {
            return true;
        }
        remaining = after_name;
    }
    false
}

fn compute_digest(input: &NativeFailureEvidenceInput) -> String {
    let mut canonical = Vec::new();
    push_field(&mut canonical, "owner", input.owner.as_bytes());
    push_field(&mut canonical, "repo", input.repo.as_bytes());
    push_field(&mut canonical, "commit_sha", input.commit_sha.as_bytes());
    push_field(
        &mut canonical,
        "run_id",
        input.run_id.to_string().as_bytes(),
    );
    push_field(
        &mut canonical,
        "job_id",
        input.job_id.to_string().as_bytes(),
    );
    match input.failing_step.as_deref() {
        Some(step) => {
            push_field(&mut canonical, "failing_step.present", b"true");
            push_field(&mut canonical, "failing_step", step.as_bytes());
        }
        None => {
            push_field(&mut canonical, "failing_step.present", b"false");
        }
    }
    push_field(&mut canonical, "verdict", b"failure");
    push_field(
        &mut canonical,
        "redacted_log_tail",
        input.redacted_log_tail.as_bytes(),
    );

    let digest = Sha256::digest(canonical);
    format!("{digest:x}")
}

fn push_field(canonical: &mut Vec<u8>, name: &str, value: &[u8]) {
    canonical.extend_from_slice(name.as_bytes());
    canonical.push(0);
    canonical.extend_from_slice(value.len().to_string().as_bytes());
    canonical.push(0);
    canonical.extend_from_slice(value);
    canonical.push(0xff);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn strict_sandbox_error_formatting_has_no_backend_source_chain() {
        let sensitive = "/checkout/path unix:///var/run/docker.sock token=secret";

        for error in [
            StrictDockerSandboxError::InvalidCheckout,
            StrictDockerSandboxError::BackendInit,
            StrictDockerSandboxError::DockerUnhealthy,
            StrictDockerSandboxError::NonDockerBackend,
        ] {
            assert!(!format!("{error:?}").contains(sensitive));
            assert!(!error.to_string().contains(sensitive));
            assert!(error.source().is_none());
        }
    }
}
