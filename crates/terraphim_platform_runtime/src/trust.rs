//! Trust-tier vocabulary for the Terraphim platform runtime.

/// Discrete trust level assigned to a workspace, command, or artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum TrustTier {
    /// Untrusted: requires the strongest isolation.
    L0,
    /// Low trust: default for fresh external inputs.
    L1,
    /// Medium trust: validated by tooling or prior review.
    L2,
    /// High trust: golden or explicitly allowlisted.
    L3,
}

impl TrustTier {
    /// Increase trust by one tier, saturating at [`TrustTier::L3`].
    pub fn promote(self) -> Self {
        match self {
            Self::L0 => Self::L1,
            Self::L1 => Self::L2,
            Self::L2 | Self::L3 => Self::L3,
        }
    }

    /// Decrease trust by one tier, saturating at [`TrustTier::L0`].
    pub fn demote(self) -> Self {
        match self {
            Self::L0 | Self::L1 => Self::L0,
            Self::L2 => Self::L1,
            Self::L3 => Self::L2,
        }
    }
}
