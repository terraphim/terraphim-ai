fn extract_clean_output(output: &str) -> String {
    output
        .lines()
        .filter(|line| {
            !line.contains("INFO")
                && !line.contains("WARN")
                && !line.contains("DEBUG")
                && !line.contains("OpenDal")
                && !line.contains("Creating role")
                && !line.contains("Successfully built thesaurus")
                && !line.contains("Starting summarization worker")
                && !line.contains("Failed to load config")
                && !line.contains("Failed to load thesaurus")
                && !line.contains("ERROR")
                && !line.trim().is_empty()
        })
        .collect::<Vec<&str>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    #[test]
    fn test_replace_npm_to_bun() {
        let output = Command::new("cargo")
            .args([
                "run",
                "--quiet",
                "-p",
                "terraphim_tui",
                "--bin",
                "terraphim-tui",
                "--",
                "replace",
                "npm",
            ])
            .output()
            .expect("Failed to execute command");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let clean_output = extract_clean_output(&stdout);

        assert!(
            clean_output.contains("bun"),
            "Expected 'bun' in output, got: {}",
            clean_output
        );
    }

    #[test]
    fn test_replace_yarn_to_bun() {
        let output = Command::new("cargo")
            .args([
                "run",
                "--quiet",
                "-p",
                "terraphim_tui",
                "--bin",
                "terraphim-tui",
                "--",
                "replace",
                "yarn",
            ])
            .output()
            .expect("Failed to execute command");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let clean_output = extract_clean_output(&stdout);

        assert!(
            clean_output.contains("bun"),
            "Expected 'bun' in output, got: {}",
            clean_output
        );
    }

    #[test]
    fn test_replace_pnpm_install_to_bun() {
        let output = Command::new("cargo")
            .args([
                "run",
                "--quiet",
                "-p",
                "terraphim_tui",
                "--bin",
                "terraphim-tui",
                "--",
                "replace",
                "pnpm install",
            ])
            .output()
            .expect("Failed to execute command");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let clean_output = extract_clean_output(&stdout);

        assert!(
            clean_output.contains("bun"),
            "Expected 'bun' in output, got: {}",
            clean_output
        );
    }

    #[test]
    fn test_replace_yarn_install_to_bun() {
        let output = Command::new("cargo")
            .args([
                "run",
                "--quiet",
                "-p",
                "terraphim_tui",
                "--bin",
                "terraphim-tui",
                "--",
                "replace",
                "yarn install",
            ])
            .output()
            .expect("Failed to execute command");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let clean_output = extract_clean_output(&stdout);

        assert!(
            clean_output.contains("bun"),
            "Expected 'bun' in output, got: {}",
            clean_output
        );
    }

    #[test]
    fn test_replace_with_markdown_format() {
        let output = Command::new("cargo")
            .args([
                "run",
                "--quiet",
                "-p",
                "terraphim_tui",
                "--bin",
                "terraphim-tui",
                "--",
                "replace",
                "npm",
                "--format",
                "markdown",
            ])
            .output()
            .expect("Failed to execute command");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let clean_output = extract_clean_output(&stdout);

        assert!(
            clean_output.contains("[bun]"),
            "Expected '[bun]' in markdown output, got: {}",
            clean_output
        );
    }

    #[test]
    fn test_replace_help_output() {
        let output = Command::new("cargo")
            .args([
                "run",
                "--quiet",
                "-p",
                "terraphim_tui",
                "--bin",
                "terraphim-tui",
                "--",
                "replace",
                "--help",
            ])
            .output()
            .expect("Failed to execute command");

        let stdout = String::from_utf8_lossy(&output.stdout);

        assert!(
            stdout.contains("replace") || stdout.contains("Replace"),
            "Help output should mention replace command"
        );
        assert!(
            stdout.contains("text") || stdout.contains("TEXT"),
            "Help output should mention text argument"
        );
    }

    #[test]
    fn test_extract_clean_output_helper() {
        let raw_output = r#"INFO: Starting process
DEBUG: Loading configuration
bun
WARN: Some warning
ERROR: Failed to load thesaurus
"#;
        let clean = extract_clean_output(raw_output);
        assert_eq!(clean, "bun");
    }

    #[test]
    fn test_extract_clean_output_multiline() {
        let raw_output = r#"[2025-10-06T19:35:46Z WARN  opendal::services] service=memory
bun install
[2025-10-06T19:35:46Z ERROR terraphim_service] Failed to load
"#;
        let clean = extract_clean_output(raw_output);
        assert_eq!(clean, "bun install");
    }
}
