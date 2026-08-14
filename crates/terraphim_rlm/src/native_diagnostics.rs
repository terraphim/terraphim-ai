//! Typed native diagnostic evidence, probes, and deterministic diagnoses.

use crate::error::RlmError;
use crate::executor::ExecutionResult;
#[cfg(feature = "docker-backend")]
use crate::executor::ProbeExecutionLimits;
#[cfg(feature = "docker-backend")]
use crate::executor::StrictDockerDiagnosticsSandbox;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use sha2::{Digest, Sha256};
use std::fmt;

const MAX_DIAGNOSIS_SUMMARY_BYTES: usize = 160;
/// Hard ceiling for a fully validated native failure evidence token.
///
/// The ceiling is enforced after syntactic validation and normalization over
/// raw semantic bytes: trimmed owner, trimmed repo, commit SHA, optional
/// normalized failing step, redacted log tail, plus fixed-width run id, job id,
/// verdict discriminant, and optional-step presence marker.
pub const MAX_NATIVE_FAILURE_EVIDENCE_BYTES: usize = 64 * 1024;
const MAX_OUTPUT_BYTES: usize = 1024 * 1024;
const MAX_EVIDENCE_OWNER_BYTES: usize = 100;
const MAX_EVIDENCE_REPO_BYTES: usize = 100;
const MAX_STEP_NAME_BYTES: usize = 80;
const U64_ACCOUNTING_BYTES: usize = 8;
const DISCRIMINANT_ACCOUNTING_BYTES: usize = 1;

/// Closed set of public executable native diagnostic probes.
#[derive(Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum Probe {
    /// `cargo metadata --no-deps --format-version 1`.
    CargoMetadataNoDeps,
    /// `git diff --check`.
    GitDiffCheck,
}

impl Probe {
    /// Stable safe identity for logs, digests, and diagnoses.
    pub fn identity(self) -> &'static str {
        match self {
            Self::CargoMetadataNoDeps => "cargo-metadata-no-deps",
            Self::GitDiffCheck => "git-diff-check",
        }
    }
}

impl fmt::Debug for Probe {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CargoMetadataNoDeps => formatter.write_str("CargoMetadataNoDeps"),
            Self::GitDiffCheck => formatter.write_str("GitDiffCheck"),
        }
    }
}

/// Result returned by a typed diagnostic probe.
#[derive(Clone, Eq, PartialEq, Serialize)]
pub struct ProbeResult {
    probe: Probe,
    exit_code: i32,
    stdout: String,
    stderr: String,
    truncated: bool,
    timed_out: bool,
}

impl ProbeResult {
    /// Construct status-only probe result data with no captured output.
    #[cfg(test)]
    pub(crate) fn status(probe: Probe, exit_code: i32, truncated: bool, timed_out: bool) -> Self {
        Self {
            probe,
            exit_code,
            stdout: String::new(),
            stderr: String::new(),
            truncated,
            timed_out,
        }
    }

    pub(crate) fn from_execution(probe: Probe, result: ExecutionResult) -> Self {
        let (stdout, stdout_truncated) = bounded_output(result.stdout);
        let (stderr, stderr_truncated) = bounded_output(result.stderr);
        Self {
            probe,
            exit_code: result.exit_code,
            stdout,
            stderr,
            truncated: result.output_truncated || stdout_truncated || stderr_truncated,
            timed_out: result.timed_out,
        }
    }

    /// Probe identity.
    pub fn probe(&self) -> &Probe {
        &self.probe
    }

    /// Process exit code.
    pub fn exit_code(&self) -> i32 {
        self.exit_code
    }

    /// Bounded stdout captured by the executor.
    pub fn stdout(&self) -> &str {
        &self.stdout
    }

    /// Bounded stderr captured by the executor.
    pub fn stderr(&self) -> &str {
        &self.stderr
    }

    /// Whether output was truncated by the executor or result boundary.
    pub fn truncated(&self) -> bool {
        self.truncated
    }

    /// Whether execution timed out.
    pub fn timed_out(&self) -> bool {
        self.timed_out
    }
}

impl fmt::Debug for ProbeResult {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProbeResult")
            .field("probe", &self.probe)
            .field("exit_code", &self.exit_code)
            .field("stdout", &"<redacted>")
            .field("stderr", &"<redacted>")
            .field("truncated", &self.truncated)
            .field("timed_out", &self.timed_out)
            .finish()
    }
}

/// Execute one typed diagnostic probe in the strict Docker diagnostics sandbox.
///
/// # Errors
///
/// Returns an RLM execution error if the strict Docker probe fails to start or
/// complete, or if fail-closed cleanup cannot be proven.
#[cfg(feature = "docker-backend")]
pub async fn execute_probe(
    sandbox: &StrictDockerDiagnosticsSandbox,
    evidence: &ValidatedNativeFailureEvidence,
    probe: Probe,
    limits: ProbeExecutionLimits,
) -> Result<ProbeResult, RlmError> {
    sandbox.execute_probe(evidence, probe, limits).await
}

/// Deterministic native failure diagnosis categories.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub enum DiagnosisKind {
    /// Compilation or metadata failed.
    CompileFailure,
    /// Test build/execution failed.
    TestFailure,
    /// Clippy failed.
    ClippyFailure,
    /// Checkout or diff hygiene failed.
    CheckoutOrDiffIssue,
    /// A probe timed out.
    Timeout,
    /// Only validated evidence was available and no deterministic rule matched.
    EvidenceOnlyUnknown,
}

/// Deterministic zero-LLM diagnosis derived from validated evidence and probes.
#[derive(Clone, Eq, PartialEq, Serialize)]
pub struct Diagnosis {
    kind: DiagnosisKind,
    evidence_digest: String,
    diagnosis_digest: String,
    summary: String,
    remediations: Vec<RemediationSuggestion>,
}

impl Diagnosis {
    /// Derive a deterministic diagnosis without LLM/model calls.
    pub fn from_evidence_and_probes(
        evidence: &ValidatedNativeFailureEvidence,
        probes: &[ProbeResult],
    ) -> Self {
        // Validated evidence gates deterministic diagnostics. Correlating the
        // mounted checkout to owner/repo/SHA is the caller or event adapter's
        // responsibility unless a reliable local correlation signal is added.
        let kind = diagnose_kind(evidence, probes);
        let summary = bounded_summary(evidence.redacted_log_tail(), MAX_DIAGNOSIS_SUMMARY_BYTES);
        let remediations = remediations_for(&kind, evidence.failing_step());
        let diagnosis_digest = compute_diagnosis_digest(&kind, evidence.evidence_digest(), probes);

        Self {
            kind,
            evidence_digest: evidence.evidence_digest().to_string(),
            diagnosis_digest,
            summary,
            remediations,
        }
    }

    /// Diagnosis category.
    pub fn kind(&self) -> &DiagnosisKind {
        &self.kind
    }

    /// Original evidence digest.
    pub fn evidence_digest(&self) -> &str {
        &self.evidence_digest
    }

    /// Stable digest over evidence identity, diagnosis kind, and probe statuses.
    pub fn diagnosis_digest(&self) -> &str {
        &self.diagnosis_digest
    }

    /// Bounded safe summary derived from already-redacted evidence only.
    pub fn summary(&self) -> &str {
        &self.summary
    }

    /// Inert remediation suggestions.
    pub fn remediations(&self) -> &[RemediationSuggestion] {
        &self.remediations
    }
}

impl fmt::Debug for Diagnosis {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Diagnosis")
            .field("kind", &self.kind)
            .field("evidence_digest", &self.evidence_digest)
            .field("diagnosis_digest", &self.diagnosis_digest)
            .field("summary", &"<redacted>")
            .field("remediations", &self.remediations)
            .finish()
    }
}

/// Inert remediation suggestions. These variants do not execute work.
#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum RemediationSuggestion {
    /// Re-run the native CI job.
    ReRunNativeJob,
    /// Inspect a validated failing step by name.
    InspectStep {
        /// Validated bounded step name.
        name: StepName,
    },
    /// Fix compile errors.
    FixCompileErrors,
    /// Fix formatting or diff hygiene.
    FixFormatting,
}

impl RemediationSuggestion {
    /// Construct a validated inert inspect-step suggestion.
    ///
    /// # Errors
    ///
    /// Returns an error when the name is empty, oversized, or contains
    /// non-printable/control characters.
    pub fn inspect_step(name: &str) -> Result<Self, StepNameError> {
        Ok(Self::InspectStep {
            name: StepName::parse(name)?,
        })
    }
}

impl fmt::Debug for RemediationSuggestion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReRunNativeJob => formatter.write_str("ReRunNativeJob"),
            Self::InspectStep { .. } => formatter
                .debug_struct("InspectStep")
                .field("name", &"<redacted>")
                .finish(),
            Self::FixCompileErrors => formatter.write_str("FixCompileErrors"),
            Self::FixFormatting => formatter.write_str("FixFormatting"),
        }
    }
}

/// Validated bounded diagnostic step name.
#[derive(Clone, Eq, Hash, PartialEq)]
pub struct StepName(String);

impl StepName {
    /// Parse a bounded printable diagnostic step name.
    ///
    /// # Errors
    ///
    /// Returns an error when the name is empty, oversized, or contains
    /// non-printable/control characters.
    pub fn parse(value: &str) -> Result<Self, StepNameError> {
        let trimmed = value.trim();
        if valid_step_name(trimmed) {
            Ok(Self(trimmed.to_string()))
        } else {
            Err(StepNameError)
        }
    }

    /// Borrow the validated step name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for StepName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("StepName")
            .field(&"<redacted>")
            .finish()
    }
}

impl Serialize for StepName {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for StepName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(&value).map_err(de::Error::custom)
    }
}

/// Safe step-name validation error.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("step name must be a bounded printable diagnostic step")]
pub struct StepNameError;

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

/// Candidate evidence supplied to [`ValidatedNativeFailureEvidence::validate`].
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
    ///
    /// Must be positive and no larger than
    /// [`MAX_NATIVE_FAILURE_EVIDENCE_BYTES`].
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
            .field(
                "failing_step",
                &self.failing_step.as_ref().map(|_| "<redacted>"),
            )
            .field("verdict", &self.verdict)
            .field("redacted_log_tail", &"<redacted>")
            .field("max_evidence_bytes", &self.max_evidence_bytes)
            .finish()
    }
}

/// Validated native CI failure evidence identity token.
#[derive(Clone, Eq, PartialEq)]
pub struct ValidatedNativeFailureEvidence {
    owner: String,
    repo: String,
    commit_sha: String,
    run_id: u64,
    job_id: u64,
    failing_step: Option<StepName>,
    verdict: NativeVerdict,
    redacted_log_tail: String,
    evidence_digest: String,
}

/// Backwards-compatible alias for the validated evidence token.
pub type NativeFailureEvidence = ValidatedNativeFailureEvidence;

impl fmt::Debug for ValidatedNativeFailureEvidence {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ValidatedNativeFailureEvidence")
            .field("owner", &self.owner)
            .field("repo", &self.repo)
            .field("commit_sha", &self.commit_sha)
            .field("run_id", &self.run_id)
            .field("job_id", &self.job_id)
            .field(
                "failing_step",
                &self.failing_step.as_ref().map(|_| "<redacted>"),
            )
            .field("validated_failure", &true)
            .field("redacted_log_tail", &"<redacted>")
            .field("evidence_digest", &self.evidence_digest)
            .finish()
    }
}

impl ValidatedNativeFailureEvidence {
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
        if input.max_evidence_bytes == 0
            || input.max_evidence_bytes > MAX_NATIVE_FAILURE_EVIDENCE_BYTES
        {
            return Err(NativeFailureEvidenceError::InvalidEvidenceLimit);
        }
        if input.redacted_log_tail.len() > input.max_evidence_bytes {
            return Err(NativeFailureEvidenceError::EvidenceTooLarge);
        }
        if input.redacted_log_tail.len() > MAX_NATIVE_FAILURE_EVIDENCE_BYTES {
            return Err(NativeFailureEvidenceError::EvidenceTooLarge);
        }
        if input.verdict != NativeVerdict::Failure {
            return Err(NativeFailureEvidenceError::NonFailureVerdict);
        }
        let failing_step = input
            .failing_step
            .as_deref()
            .map(StepName::parse)
            .transpose()
            .map_err(|_| NativeFailureEvidenceError::InvalidFailingStep)?;
        input.failing_step = failing_step
            .as_ref()
            .map(|step_name| step_name.as_str().to_string());

        if native_failure_evidence_accounted_bytes(&input) > MAX_NATIVE_FAILURE_EVIDENCE_BYTES {
            return Err(NativeFailureEvidenceError::EvidenceTooLarge);
        }
        if contains_unredacted_secret(&input.redacted_log_tail) {
            return Err(NativeFailureEvidenceError::UnredactedSecret);
        }

        let evidence_digest = compute_digest(&input);

        Ok(Self {
            owner: input.owner,
            repo: input.repo,
            commit_sha: input.commit_sha,
            run_id: input.run_id,
            job_id: input.job_id,
            failing_step,
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
        self.failing_step.as_ref().map(StepName::as_str)
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

    /// Marker proving this token came through the validated failure boundary.
    pub fn validated_failure_marker(&self) -> bool {
        matches!(self.verdict, NativeVerdict::Failure)
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
    /// Evidence byte limit was zero or above the hard ceiling.
    #[error("evidence byte limit must be positive and within the hard ceiling")]
    InvalidEvidenceLimit,
    /// Evidence appears to contain an unredacted secret.
    #[error("evidence appears to contain an unredacted secret")]
    UnredactedSecret,
    /// The supplied native verdict was not a failure.
    #[error("native verdict must be failure")]
    NonFailureVerdict,
    /// Optional failing step was not a safe bounded step name.
    #[error("failing step must be absent or a bounded printable diagnostic step")]
    InvalidFailingStep,
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
        && trimmed.len() <= identity_max_bytes(field)
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

fn identity_max_bytes(field: IdentityField) -> usize {
    match field {
        IdentityField::Owner => MAX_EVIDENCE_OWNER_BYTES,
        IdentityField::Repo => MAX_EVIDENCE_REPO_BYTES,
    }
}

fn native_failure_evidence_accounted_bytes(input: &NativeFailureEvidenceInput) -> usize {
    input
        .owner
        .len()
        .saturating_add(input.repo.len())
        .saturating_add(input.commit_sha.len())
        .saturating_add(U64_ACCOUNTING_BYTES)
        .saturating_add(U64_ACCOUNTING_BYTES)
        .saturating_add(DISCRIMINANT_ACCOUNTING_BYTES)
        .saturating_add(DISCRIMINANT_ACCOUNTING_BYTES)
        .saturating_add(input.failing_step.as_ref().map_or(0, String::len))
        .saturating_add(input.redacted_log_tail.len())
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

fn diagnose_kind(
    evidence: &ValidatedNativeFailureEvidence,
    probes: &[ProbeResult],
) -> DiagnosisKind {
    if probes.iter().any(ProbeResult::timed_out) {
        return DiagnosisKind::Timeout;
    }

    if has_nonzero_probe(probes, Probe::GitDiffCheck)
        || failing_step_contains(evidence, &["git diff", "diff --check"])
    {
        return DiagnosisKind::CheckoutOrDiffIssue;
    }

    if failing_step_contains(evidence, &["clippy"]) {
        return DiagnosisKind::ClippyFailure;
    }

    if failing_step_contains(evidence, &["cargo test", "test"]) {
        return DiagnosisKind::TestFailure;
    }

    if has_nonzero_probe(probes, Probe::CargoMetadataNoDeps)
        || failing_step_contains(evidence, &["cargo build", "cargo check", "compile"])
    {
        return DiagnosisKind::CompileFailure;
    }

    DiagnosisKind::EvidenceOnlyUnknown
}

fn has_nonzero_probe(probes: &[ProbeResult], probe: Probe) -> bool {
    probes
        .iter()
        .any(|result| result.exit_code() != 0 && result.probe == probe)
}

fn failing_step_contains(evidence: &ValidatedNativeFailureEvidence, needles: &[&str]) -> bool {
    let Some(step) = evidence.failing_step() else {
        return false;
    };
    let lower = step.to_ascii_lowercase();
    needles.iter().any(|needle| lower.contains(needle))
}

fn remediations_for(
    kind: &DiagnosisKind,
    failing_step: Option<&str>,
) -> Vec<RemediationSuggestion> {
    let mut remediations = Vec::new();
    remediations.push(RemediationSuggestion::ReRunNativeJob);
    if let Some(step) = failing_step.and_then(|step| RemediationSuggestion::inspect_step(step).ok())
    {
        remediations.push(step);
    }

    match kind {
        DiagnosisKind::CompileFailure
        | DiagnosisKind::TestFailure
        | DiagnosisKind::ClippyFailure => {
            remediations.push(RemediationSuggestion::FixCompileErrors);
        }
        DiagnosisKind::CheckoutOrDiffIssue => {
            remediations.push(RemediationSuggestion::FixFormatting);
        }
        DiagnosisKind::Timeout | DiagnosisKind::EvidenceOnlyUnknown => {}
    }

    remediations
}

fn compute_diagnosis_digest(
    kind: &DiagnosisKind,
    evidence_digest: &str,
    probes: &[ProbeResult],
) -> String {
    let mut canonical = Vec::new();
    push_field(&mut canonical, "kind", format!("{kind:?}").as_bytes());
    push_field(
        &mut canonical,
        "evidence_digest",
        evidence_digest.as_bytes(),
    );
    let backed = if probes.is_empty() {
        "evidence-only"
    } else {
        "probe-backed"
    };
    push_field(&mut canonical, "classification_source", backed.as_bytes());
    for probe in probes {
        push_field(&mut canonical, "probe", probe.probe.identity().as_bytes());
        push_field(
            &mut canonical,
            "exit_code",
            probe.exit_code.to_string().as_bytes(),
        );
        push_field(
            &mut canonical,
            "truncated",
            probe.truncated.to_string().as_bytes(),
        );
        push_field(
            &mut canonical,
            "timed_out",
            probe.timed_out.to_string().as_bytes(),
        );
    }

    let digest = Sha256::digest(canonical);
    format!("{digest:x}")
}

fn bounded_summary(value: &str, max_bytes: usize) -> String {
    let mut summary = String::new();
    for ch in value.chars() {
        if summary.len().saturating_add(ch.len_utf8()) > max_bytes {
            break;
        }
        summary.push(ch);
    }
    summary
}

fn valid_step_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_STEP_NAME_BYTES
        && !contains_unredacted_secret(value)
        && value
            .chars()
            .all(|ch| ch.is_ascii() && !ch.is_ascii_control())
}

fn bounded_output(value: String) -> (String, bool) {
    if value.len() <= MAX_OUTPUT_BYTES {
        return (value, false);
    }

    let mut output = String::new();
    for ch in value.chars() {
        if output.len().saturating_add(ch.len_utf8()) > MAX_OUTPUT_BYTES {
            break;
        }
        output.push(ch);
    }
    (output, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evidence_with_step(step: Option<&str>, log: &str) -> ValidatedNativeFailureEvidence {
        ValidatedNativeFailureEvidence::validate(NativeFailureEvidenceInput {
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
    fn diagnosis_digest_distinguishes_evidence_only_from_probe_backed() {
        let evidence = evidence_with_step(Some("cargo build"), "compile failure");
        let evidence_only = Diagnosis::from_evidence_and_probes(&evidence, &[]);
        let probe_backed = Diagnosis::from_evidence_and_probes(
            &evidence,
            &[ProbeResult::status(
                Probe::CargoMetadataNoDeps,
                101,
                false,
                false,
            )],
        );

        assert_eq!(evidence_only.kind(), &DiagnosisKind::CompileFailure);
        assert_eq!(probe_backed.kind(), &DiagnosisKind::CompileFailure);
        assert_ne!(
            evidence_only.diagnosis_digest(),
            probe_backed.diagnosis_digest()
        );
    }

    #[test]
    fn probe_backed_diagnosis_rules_use_internal_probe_provenance() {
        let evidence = evidence_with_step(Some("native ci"), "failure");
        let compile = Diagnosis::from_evidence_and_probes(
            &evidence,
            &[ProbeResult::status(
                Probe::CargoMetadataNoDeps,
                101,
                false,
                false,
            )],
        );
        assert_eq!(compile.kind(), &DiagnosisKind::CompileFailure);

        let diff = Diagnosis::from_evidence_and_probes(
            &evidence,
            &[ProbeResult::status(Probe::GitDiffCheck, 1, false, false)],
        );
        assert_eq!(diff.kind(), &DiagnosisKind::CheckoutOrDiffIssue);

        let timeout = Diagnosis::from_evidence_and_probes(
            &evidence,
            &[ProbeResult::status(
                Probe::CargoMetadataNoDeps,
                -1,
                false,
                true,
            )],
        );
        assert_eq!(timeout.kind(), &DiagnosisKind::Timeout);
    }

    #[test]
    fn probe_result_from_execution_bounds_output() {
        let result = ProbeResult::from_execution(
            Probe::CargoMetadataNoDeps,
            ExecutionResult {
                stdout: "a".repeat(MAX_OUTPUT_BYTES + 1),
                stderr: String::new(),
                exit_code: 0,
                execution_time_ms: 1,
                output_truncated: false,
                output_file_path: None,
                timed_out: false,
                metadata: Default::default(),
            },
        );

        assert_eq!(result.stdout().len(), MAX_OUTPUT_BYTES);
        assert!(result.truncated());
    }
}
