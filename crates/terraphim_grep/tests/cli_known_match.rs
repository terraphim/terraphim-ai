#![cfg(feature = "code-search")]

use serde_json::Value;
use std::fs;
use std::process::Command;

fn run_grep(args: &[&str], cwd: &std::path::Path) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_terraphim-grep"))
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("run terraphim-grep test binary");

    assert!(
        output.status.success(),
        "terraphim-grep failed\nstatus: {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    serde_json::from_slice(&output.stdout).unwrap_or_else(|err| {
        panic!(
            "stdout was not valid JSON: {err}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

#[test]
fn cli_known_match_directory_returns_chunk_and_truthful_stats() {
    let dir = tempfile::tempdir().expect("temp dir");
    let file = dir.path().join("README.md");
    fs::write(&file, "# Fixture\n\nrelease guardian sentinel\n").expect("write fixture");

    let thesaurus = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../terraphim_server/fixtures/thesaurus_Default.json"
    );

    let json = run_grep(
        &[
            "--json",
            "--thesaurus",
            thesaurus,
            "--paths",
            dir.path().to_str().expect("utf8 temp path"),
            "release guardian sentinel",
        ],
        dir.path(),
    );

    let chunks = json["chunks"].as_array().expect("chunks array");
    assert!(!chunks.is_empty(), "expected at least one chunk: {json}");
    assert!(
        chunks
            .iter()
            .any(|chunk| chunk.to_string().contains("release guardian sentinel")),
        "expected sentinel in chunks: {json}"
    );
    assert_eq!(
        json["stats"]["chunks_returned"].as_u64(),
        Some(chunks.len() as u64),
        "stats.chunks_returned must match actual chunks: {json}"
    );
}
