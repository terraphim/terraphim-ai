//! Fleet invariant: every `*build-runner` agent in the live conf.d files must
//! have `event_only = true`. This locks in the dispatch-gate policy across the
//! fleet so a future operator who adds a new build-runner clone cannot
//! accidentally reintroduce the comment-mention dispatch bug fixed in #1104.
//!
//! Conf.d fixtures are copied from /opt/ai-dark-factory/conf.d/ at test-write
//! time. The migrate-to-confd.py pipeline is one-shot, so the tests/fixtures/
//! directory is the supported way to reference live config from a unit test
//! without coupling to absolute paths under /opt/ai-dark-factory/.

use std::path::PathBuf;
use toml::Value;

const FLEET_FILES: &[&str] = &[
    "terraphim.toml",
    "atomic-server.toml",
    "better-auth-rust.toml",
    "gitea.toml",
    "gitea-robot.toml",
];

fn fixture_path(name: &str) -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("tests");
    p.push("fixtures");
    p.push("conf.d");
    p.push(name);
    p
}

#[test]
fn test_fleet_conf_d_build_runner_event_only_invariant() {
    let mut total_build_runners = 0usize;

    for fname in FLEET_FILES {
        let path = fixture_path(fname);
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("could not read fixture {}: {}", path.display(), e));
        let value: Value = toml::from_str(&content)
            .unwrap_or_else(|e| panic!("could not parse {}: {}", path.display(), e));

        let agents = value.get("agents").and_then(Value::as_array);
        let agents = match agents {
            Some(a) => a,
            None => continue,
        };

        for agent in agents {
            let name = agent
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("<unnamed>");
            if !name.ends_with("build-runner") {
                continue;
            }

            total_build_runners += 1;
            let event_only = agent
                .get("event_only")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            assert!(
                event_only,
                "fleet invariant violated: agent {:?} in {} must have event_only = true (Refs #1104)",
                name, fname
            );
        }
    }

    assert!(
        total_build_runners >= 5,
        "expected at least 5 build-runner-pattern agents across the fleet; found {}",
        total_build_runners
    );
}

#[test]
fn test_source_template_build_runner_event_only() {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.pop();
    path.push("scripts");
    path.push("adf-setup");
    path.push("agents");
    path.push("build-runner.toml");

    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("could not read source template {}: {}", path.display(), e));
    let value: Value = toml::from_str(&content)
        .unwrap_or_else(|e| panic!("could not parse source template: {}", e));

    let agents = value
        .get("agents")
        .and_then(Value::as_array)
        .expect("source template must contain [[agents]]");

    let build_runner = agents
        .iter()
        .find(|a| a.get("name").and_then(Value::as_str) == Some("build-runner"))
        .expect("source template must contain build-runner agent");

    let event_only = build_runner
        .get("event_only")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    assert!(
        event_only,
        "source template build-runner must have event_only = true to match the fleet invariant"
    );
}
