#![cfg(not(feature = "code-search"))]

use std::fs;
use std::process::Command;

#[test]
fn cli_without_code_search_reports_explicit_error() {
    let dir = tempfile::tempdir().expect("temp dir");
    let file = dir.path().join("README.md");
    fs::write(&file, "# Fixture\n\nrelease guardian sentinel\n").expect("write fixture");

    let thesaurus = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../terraphim_server/fixtures/thesaurus_Default.json"
    );

    let output = Command::new(env!("CARGO_BIN_EXE_terraphim-grep"))
        .args([
            "--json",
            "--thesaurus",
            thesaurus,
            "--paths",
            dir.path().to_str().expect("utf8 temp path"),
            "release guardian sentinel",
        ])
        .current_dir(dir.path())
        .output()
        .expect("run terraphim-grep test binary");

    assert!(
        !output.status.success(),
        "no-code-search build should fail explicitly instead of returning empty successful JSON"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("without the `code-search` feature"),
        "expected explicit code-search error, got stderr:\n{stderr}\nstdout:\n{}",
        String::from_utf8_lossy(&output.stdout)
    );
}
