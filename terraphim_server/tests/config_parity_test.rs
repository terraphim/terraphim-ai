//! Desktop/server configuration parity regression test (WIG-2).
//!
//! Guards that the twin default configuration files shipped for the desktop
//! frontend and the server stay in sync on every field covered by the shared
//! `terraphim_settings::DeviceSettings` contract. A silent drift in device
//! settings (hostname, api endpoint, or storage profiles) breaks the unified
//! client experience: the desktop client would target the wrong endpoint or a
//! storage backend the server does not expose.
//!
//! ## Scope notes (prevents re-work)
//!
//! * The Tauri Rust backend was extracted to the standalone
//!   `terraphim-ai-desktop` repo (commit `e3e6a2633`). In this repo `desktop/`
//!   is frontend-only and the shared config *types* are consumed from the
//!   `terraphim` registry (`terraphim_config` / `terraphim_settings`). A
//!   struct-level desktop-vs-server Rust comparison is therefore impossible
//!   here; we instead compare the concrete default TOML instances that both
//!   clients must agree on.
//! * `DeviceSettings` models exactly these fields: `server_hostname`,
//!   `api_endpoint`, `initialized`, `profiles`, plus optional `role_config` /
//!   `default_role`. The `[roles.*]` tables are loaded separately from a
//!   `role_config` JSON file and are intentionally *not* part of the device
//!   contract, so role divergence between the twin files is out of scope and
//!   is asserted only as a documented expectation below.

use serde_json::Value;
use std::path::PathBuf;

/// Read a repo-relative twin default config file, resolving the path from
/// `CARGO_MANIFEST_DIR` so the test is independent of the working directory.
fn load_twin(rel: &str) -> String {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for component in rel.split('/') {
        path.push(component);
    }
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read twin config {rel}: {e}"))
}

/// Parse a TOML document into a JSON value. Using `serde_json::Value` lets us
/// compare the deserialised structure without depending on `PartialEq` being
/// derived on the registry `DeviceSettings` type (it is not).
fn parse_toml_to_json(name: &str, contents: &str) -> Value {
    let toml_value: toml::Value =
        toml::from_str(contents).unwrap_or_else(|e| panic!("failed to parse {name} as TOML: {e}"));
    serde_json::to_value(&toml_value)
        .unwrap_or_else(|e| panic!("failed to re-encode {name} to JSON: {e}"))
}

/// The twin default files under comparison.
struct Twins {
    desktop: Value,
    server: Value,
}

impl Twins {
    fn load() -> Self {
        let desktop_text = load_twin("../desktop/default/settings_default_desktop.toml");
        let server_text = load_twin("default/settings_default_server.toml");
        Self {
            desktop: parse_toml_to_json("settings_default_desktop.toml", &desktop_text),
            server: parse_toml_to_json("settings_default_server.toml", &server_text),
        }
    }
}

/// Both files must expose the device-contract scalars and they must be
/// byte-for-byte identical. Any drift here is a parity regression.
#[test]
fn device_scalars_are_identical() {
    let twins = Twins::load();
    for key in ["server_hostname", "api_endpoint", "initialized"] {
        let d = &twins.desktop[key];
        let s = &twins.server[key];
        assert_eq!(
            d, s,
            "device-contract scalar `{key}` diverges between desktop and server defaults"
        );
    }
}

/// The set of storage backends (`[profiles.*]`) and their parameters must be
/// identical twins. A desktop client advertising a profile the server lacks
/// (or vice-versa) breaks the unified storage contract.
///
/// This also satisfies the "serde round-trip serialisation is identical"
/// acceptance criterion: we compare the re-serialised JSON form of the
/// deserialised `profiles` table.
#[test]
fn storage_profiles_round_trip_identically() {
    let twins = Twins::load();
    let desktop_profiles = &twins.desktop["profiles"];
    let server_profiles = &twins.server["profiles"];

    // Deterministic canonical serialisation for the comparison.
    let desktop_json = serde_json::to_string_pretty(desktop_profiles).unwrap();
    let server_json = serde_json::to_string_pretty(server_profiles).unwrap();

    assert_eq!(
        desktop_profiles, server_profiles,
        "`profiles` table diverges between desktop and server defaults"
    );
    assert_eq!(
        desktop_json, server_json,
        "`profiles` serde round-trip serialisation differs (non-deterministic encoding?)"
    );
}

/// The profile *names* must agree exactly. Catches a profile being added to
/// one twin but not the other even before the parameter-level comparison runs.
#[test]
fn storage_profile_names_agree() {
    let twins = Twins::load();
    let mut desktop_names: Vec<String> = twins.desktop["profiles"]
        .as_object()
        .expect("profiles is a table")
        .keys()
        .cloned()
        .collect();
    let mut server_names: Vec<String> = twins.server["profiles"]
        .as_object()
        .expect("profiles is a table")
        .keys()
        .cloned()
        .collect();
    desktop_names.sort();
    server_names.sort();
    assert_eq!(
        desktop_names, server_names,
        "storage profile names diverge between desktop and server defaults"
    );
}

/// Reproducibility: parsing the same input twice must yield identical output.
/// Same inputs -> identical outputs is the core twin-fidelity invariant.
#[test]
fn parsing_is_reproducible() {
    let desktop_text = load_twin("../desktop/default/settings_default_desktop.toml");
    let server_text = load_twin("default/settings_default_server.toml");

    let d1 = parse_toml_to_json("desktop#1", &desktop_text);
    let d2 = parse_toml_to_json("desktop#2", &desktop_text);
    let s1 = parse_toml_to_json("server#1", &server_text);
    let s2 = parse_toml_to_json("server#2", &server_text);

    assert_eq!(d1, d2, "desktop default parsing is non-deterministic");
    assert_eq!(s1, s2, "server default parsing is non-deterministic");
}

/// Document the *intentional* divergence of the `[roles]` tables.
///
/// Roles are loaded from a separate `role_config` JSON file (see
/// `DeviceSettings::role_config`) and are therefore outside the device-config
/// contract compared here. We assert that the roles *are* present on both sides
/// (both twins declare roles) but deliberately do not require them to match —
/// the desktop ships a richer default role set. If a future change makes roles
/// part of the shared contract, this test should be tightened accordingly.
#[test]
fn roles_divergence_is_documented_and_intentional() {
    let twins = Twins::load();
    let desktop_roles = twins
        .desktop
        .get("roles")
        .and_then(Value::as_object)
        .expect("desktop defaults declare a [roles] table");
    let server_roles = twins
        .server
        .get("roles")
        .and_then(Value::as_object)
        .expect("server defaults declare a [roles] table");

    // Contract: both twins must declare at least the baseline Default role.
    assert!(
        desktop_roles.contains_key("Default"),
        "desktop defaults must declare the Default role"
    );
    assert!(
        server_roles.contains_key("Default"),
        "server defaults must declare the Default role"
    );

    // Document (but do not fail on) the richer desktop role set. This is the
    // known, accepted drift surface — recorded so a future tightening is a
    // conscious decision rather than an accident.
    let desktop_only: Vec<&String> = desktop_roles
        .keys()
        .filter(|k| !server_roles.contains_key(*k))
        .collect();
    eprintln!(
        "roles present on desktop but not server (intentional, out of device-config contract): {desktop_only:?}"
    );
}
