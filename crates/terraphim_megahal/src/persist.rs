//! Brain persistence via `terraphim_persistence` (feature `persistence`).
//!
//! Stores [`MegaHalState`] through the `Persistable` trait, so brains land
//! on every configured device profile (memory, sqlite, redb, ...) with the
//! usual load-fallback chain. The plain `MHRS1` JSON file functions
//! ([`MegaHal::save`] / [`MegaHal::load`]) remain the dependency-free CLI
//! fallback.
//!
//! Wasm note (Phase 4): `terraphim_persistence` profiles are backed by
//! OpenDAL operators; in-browser, the applicable profile is a
//! localStorage/OPFS shim. The memory profile compiles on wasm32; browser
//! wiring is deferred to the Phase 4 demo.

use async_trait::async_trait;

use terraphim_persistence::{Persistable, Result as PersistResult};

use crate::{BRAIN_VERSION, MegaHalState};

/// A persistable Megahal brain: wraps [`MegaHalState`] under a storage key
/// (normalised by the persistence stack; the version tag is prepended so
/// future formats can coexist).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PersistedBrain {
    key: String,
    pub state: MegaHalState,
}

impl PersistedBrain {
    /// Wrap a brain state under `key`.
    pub fn from_state(key: impl Into<String>, state: MegaHalState) -> Self {
        Self {
            key: key.into(),
            state,
        }
    }
}

#[async_trait]
impl Persistable for PersistedBrain {
    fn new(key: String) -> Self {
        Self {
            key,
            state: MegaHalState {
                learning: true,
                dictionary: Vec::new(),
                brain: Vec::new(),
                seed: Default::default(),
                fore: Default::default(),
                back: Default::default(),
                case: Default::default(),
                punc: Default::default(),
            },
        }
    }

    fn get_key(&self) -> String {
        self.normalize_key(&format!("{BRAIN_VERSION}:{}", self.key))
    }

    async fn save_to_one(&self, profile_name: &str) -> PersistResult<()> {
        self.save_to_profile(profile_name).await
    }

    async fn save(&self) -> PersistResult<()> {
        self.save_to_all().await
    }

    async fn load(&mut self) -> PersistResult<Self>
    where
        Self: Sized,
    {
        let (ops, fastest_op) = self.load_config().await?;
        let key = self.get_key();
        // Try the fastest operator, then the remaining profiles in speed
        // order (load_config hands them pre-sorted; ops is a HashMap clone,
        // so attempt every profile and let the first success win).
        if let Ok(loaded) = self.load_from_operator(&key, &fastest_op).await {
            return Ok(loaded);
        }
        for (_name, (op, _latency)) in ops.iter() {
            if let Ok(loaded) = self.load_from_operator(&key, op).await {
                return Ok(loaded);
            }
        }
        Err(terraphim_persistence::Error::Profile(format!(
            "no brain stored under {key}"
        )))
    }
}
