use std::fs;
use std::path::Path;
use terraphim_negative_contribution::NegativeContributionScanner;
use walkdir::WalkDir;

fn scanner() -> NegativeContributionScanner {
    NegativeContributionScanner::new()
}

fn workspace_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
}

#[test]
fn test_scan_known_stub_file() {
    let fixture = include_str!("fixtures/stub_sample.rs");
    let findings = scanner().scan_file("fixtures/stub_sample.rs", fixture);

    assert_eq!(
        findings.len(),
        3,
        "Expected 3 findings (todo!(), unimplemented!(), panic!), suppressed one"
    );

    let lines: Vec<u32> = findings.iter().map(|f| f.line).collect();
    assert!(lines.contains(&2), "todo!() at line 2");
    assert!(lines.contains(&6), "unimplemented!() at line 6");
    assert!(lines.contains(&14), "panic! at line 14");

    assert!(
        !lines.contains(&10),
        "suppressed todo!() at line 10 should not appear"
    );
}

#[test]
fn test_scan_suppression_on_fixture() {
    let fixture = include_str!("fixtures/stub_sample.rs");
    let suppressed_lines: Vec<u32> = scanner()
        .scan_file("fixtures/stub_sample.rs", fixture)
        .iter()
        .map(|f| f.line)
        .collect();

    assert!(
        !suppressed_lines.contains(&10),
        "Line 10 has // terraphim: allow(stub) and must be suppressed"
    );
}

#[test]
fn test_scan_terraphim_ai_codebase() {
    let root = workspace_root();
    let scanner = scanner();
    let mut total_findings = 0;
    let mut files_with_findings: Vec<(String, usize)> = Vec::new();
    let mut files_scanned = 0;

    for entry in WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            let path = e.path();
            path.extension().is_some_and(|ext| ext == "rs")
                && !path.starts_with(root.join("target"))
                && !path.starts_with(root.join(".git"))
        })
    {
        let path = entry.path();
        let relative = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();

        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let findings = scanner.scan_file(&relative, &content);
        files_scanned += 1;

        if !findings.is_empty() {
            total_findings += findings.len();
            files_with_findings.push((relative, findings.len()));
        }
    }

    eprintln!("\n=== EDM Scanner Codebase Baseline ===");
    eprintln!("Files scanned: {files_scanned}");
    eprintln!("Total findings: {total_findings}");
    eprintln!("Files with findings: {}", files_with_findings.len());
    for (file, count) in &files_with_findings {
        eprintln!("  {file}: {count} finding(s)");
    }
    eprintln!("=====================================\n");

    assert!(files_scanned > 0, "Must scan at least one file");
}

#[test]
fn test_scan_does_not_panic_on_any_file() {
    let root = workspace_root();
    let scanner = scanner();
    let mut scanned = 0;

    for entry in WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            let path = e.path();
            path.extension().is_some_and(|ext| ext == "rs")
                && !path.starts_with(root.join("target"))
                && !path.starts_with(root.join(".git"))
        })
    {
        let path = entry.path();
        let relative = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();
        let content = fs::read_to_string(path).unwrap_or_default();

        let _ = scanner.scan_file(&relative, &content);
        scanned += 1;
    }

    assert!(
        scanned > 100,
        "Should scan at least 100 Rust files in workspace"
    );
}
