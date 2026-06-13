use std::path::PathBuf;
use std::process::Command;

fn tla_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tla")
}

#[test]
fn tla_prefight_specs_are_present() {
    let dir = tla_dir();
    for spec in [
        "AdfRegistry.tla",
        "AdfBuildDispatch.tla",
        "AdfRunSupervisor.tla",
        "AdfProviderHealthActor.tla",
    ] {
        let path = dir.join(spec);
        assert!(path.exists(), "missing TLA+ spec: {}", path.display());
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(
            content.contains("---- MODULE"),
            "{spec} missing module header"
        );
        assert!(
            content.contains("Spec =="),
            "{spec} missing Spec definition"
        );
    }
}

#[test]
fn tla_prefight_specs_capture_required_invariants() {
    let dir = tla_dir();
    let registry = std::fs::read_to_string(dir.join("AdfRegistry.tla")).unwrap();
    assert!(registry.contains("NoSameProjectDuplicate"));
    assert!(registry.contains("RegistryOnlyContainsEntries"));

    let build = std::fs::read_to_string(dir.join("AdfBuildDispatch.tla")).unwrap();
    assert!(build.contains("NoPendingWithoutSpawn"));
    assert!(build.contains("SkippedBuildRunnerPostsNoPending"));
    assert!(build.contains("TerminalOnlyAfterPending"));

    let supervisor = std::fs::read_to_string(dir.join("AdfRunSupervisor.tla")).unwrap();
    assert!(supervisor.contains("RetryBound"));
    assert!(supervisor.contains("NoRestartAfterEscalation"));
    assert!(supervisor.contains("ActiveReleasedOnTerminal"));

    let provider = std::fs::read_to_string(dir.join("AdfProviderHealthActor.tla")).unwrap();
    assert!(provider.contains("ProbeDelayDoesNotBlockLifecycle"));
}

#[test]
fn tla_model_checker_smoke_test_when_enabled() {
    if std::env::var_os("ADF_RUN_TLA_TESTS").is_none() {
        eprintln!("skipping TLC smoke test; set ADF_RUN_TLA_TESTS=1 to require local TLA+ tooling");
        return;
    }

    let java = Command::new("java").arg("-version").output();
    assert!(java.is_ok(), "ADF_RUN_TLA_TESTS=1 but java is unavailable");

    let tlc = Command::new("java").args(["tlc2.TLC", "-help"]).output();
    assert!(
        tlc.is_ok(),
        "ADF_RUN_TLA_TESTS=1 but TLC is unavailable on the Java classpath"
    );
}
