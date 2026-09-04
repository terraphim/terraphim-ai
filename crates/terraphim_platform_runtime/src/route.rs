//! Execution-route vocabulary for the Terraphim platform runtime.

/// Where a command or step is executed.
///
/// Cache is intentionally not a route: caching is a cross-cutting concern
/// configured through [`crate::CacheKind`], not a destination for execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Route {
    /// Directly on the runner host.
    Host,
    /// Through `rch exec --` (remote/queued cargo with sccache).
    Rch,
    /// Inside a Firecracker micro-VM.
    Firecracker,
    /// Through the Ultimate Bug Scanner pipeline.
    Ubs,
}
