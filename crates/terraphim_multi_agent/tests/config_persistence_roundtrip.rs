//! Integration test: round-trip a `terraphim_config::Config` through the
//! memory-only persistence backend.
//!
//! This test was relocated here from `terraphim_persistence`'s inline unit
//! tests as part of the Gitea #1910 cycle break: it is the only test that
//! required `terraphim_persistence` to dev-depend on `terraphim_config`, which
//! closed a `config <-> persistence` dependency cycle. `terraphim_multi_agent`
//! already production-depends on both crates, so hosting the test here adds no
//! new edge to the dependency graph and keeps the direction acyclic.
//!
//! No mocks: it exercises the real in-memory OpenDAL operator via
//! `DeviceStorage::init_memory_only()` and the real `Persistable` round-trip.

use terraphim_config::{Config, ConfigBuilder};
use terraphim_persistence::{DeviceStorage, Persistable};

#[tokio::test]
async fn config_round_trips_through_memory_persistence() {
    // Initialise the device storage with a single in-memory backend.
    DeviceStorage::init_memory_only()
        .await
        .expect("memory-only storage should initialise");

    // Build a default config and persist it.
    let config = ConfigBuilder::new()
        .build()
        .expect("config should build with defaults");
    config.save().await.expect("config should save to memory");

    // Load it back into a fresh value keyed by the same config id.
    let mut target = config.clone();
    let loaded: Config = target.load().await.expect("config should load from memory");

    // The persisted config must survive the round-trip unchanged.
    assert_eq!(config.id, loaded.id);
    assert_eq!(config.default_role, loaded.default_role);
}
