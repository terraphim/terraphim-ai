//! Native LLM runner library contracts.
//!
//! This crate re-exports the typed native diagnostics boundary owned by
//! `terraphim_rlm`. Validation lives in RLM beside the strict Docker sandbox so
//! companion code cannot drift into a second implementation.
//!
//! ```compile_fail,E0599
//! use terraphim_llm_runner::{Probe, ProbeResult};
//! let _ = ProbeResult::status(Probe::CargoMetadataNoDeps, 0, false, false);
//! ```
//!
//! ```compile_fail
//! use terraphim_llm_runner::{
//!     Probe, ProbeExecutionLimits, StrictDockerDiagnosticsSandbox,
//!     ValidatedNativeFailureEvidence,
//! };
//! async fn f(sandbox: &StrictDockerDiagnosticsSandbox, evidence: &ValidatedNativeFailureEvidence) {
//!     let _ = sandbox
//!         .execute_probe(evidence, Probe::CargoMetadataNoDeps, ProbeExecutionLimits::default())
//!         .await;
//! }
//! ```
//!
//! ```compile_fail,E0599
//! use terraphim_llm_runner::StrictDockerDiagnosticsSandbox;
//! async fn f(sandbox: &StrictDockerDiagnosticsSandbox) {
//!     let _ = sandbox.cleanup().await;
//! }
//! ```
//!
//! ```compile_fail,E0599
//! use terraphim_llm_runner::Probe;
//! let _ = Probe::Shell;
//! ```
//!
//! ```compile_fail,E0599
//! use terraphim_llm_runner::Probe;
//! let _ = Probe::from_command(["cargo", "test"]);
//! ```
//!
//! ```compile_fail,E0277
//! use terraphim_llm_runner::ProbeResult;
//! fn require_deser<'de, T: serde::Deserialize<'de>>() {}
//! require_deser::<ProbeResult>();
//! ```
//!
//! ```compile_fail,E0277
//! use terraphim_llm_runner::Diagnosis;
//! fn require_deser<'de, T: serde::Deserialize<'de>>() {}
//! require_deser::<Diagnosis>();
//! ```

use std::path::Path;

use terraphim_rlm::executor::strict_docker_diagnostics_sandbox as rlm_strict_docker_diagnostics_sandbox;
pub use terraphim_rlm::executor::{
    ProbeExecutionLimits, ProbeExecutionLimitsError, StrictDockerDiagnosticsSandbox,
    StrictDockerSandboxError,
};
pub use terraphim_rlm::native_diagnostics::{
    Diagnosis, DiagnosisKind, MAX_NATIVE_FAILURE_EVIDENCE_BYTES, NativeFailureEvidence,
    NativeFailureEvidenceError, NativeFailureEvidenceInput, NativeVerdict, Probe, ProbeResult,
    RemediationSuggestion, StepName, StepNameError, ValidatedNativeFailureEvidence, execute_probe,
};

/// Construct the strict Docker-only diagnostics sandbox.
///
/// This companion factory delegates to RLM's opaque strict Docker constructor.
/// It does not expose the strict Docker profile, raw `HostConfig`, or inner
/// Docker executor.
///
/// # Errors
///
/// Returns a sanitized fieldless error when checkout validation, Docker
/// construction, local fixed-image inspection, or direct-argv tool preflight
/// fails.
pub async fn strict_docker_diagnostics_sandbox(
    checkout_path: impl AsRef<Path>,
) -> Result<StrictDockerDiagnosticsSandbox, StrictDockerSandboxError> {
    rlm_strict_docker_diagnostics_sandbox(checkout_path).await
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
            StrictDockerSandboxError::DiagnosticsImageUnavailable,
            StrictDockerSandboxError::DiagnosticsToolsUnavailable,
            StrictDockerSandboxError::NonDockerBackend,
        ] {
            assert!(!format!("{error:?}").contains(sensitive));
            assert!(!error.to_string().contains(sensitive));
            assert!(error.source().is_none());
        }
    }
}
