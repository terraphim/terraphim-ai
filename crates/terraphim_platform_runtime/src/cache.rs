//! Cache-hint vocabulary for the Terraphim platform runtime.

/// Choice of caching strategy for a build or execution.
///
/// This type replaces the earlier `BuildCacheHint` concept from the platform
/// design document. The rename makes it clear that caching applies to more
/// than just build steps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CacheKind {
    /// Let the platform pick the best cache backend.
    Auto,
    /// Use the Terraphim Kache caching layer.
    Kache,
    /// Use `sccache` directly.
    Sccache,
    /// Disable caching for this execution.
    None,
}
