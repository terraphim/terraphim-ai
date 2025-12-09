use std::path::PathBuf;
use terraphim_automata::{builder::Logseq, ThesaurusBuilder};

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

/// Build a thesaurus from the existing KG markdown files in docs/src/kg/
async fn build_test_thesaurus() -> Result<terraphim_types::Thesaurus, Box<dyn std::error::Error>> {
    // Use CARGO_MANIFEST_DIR to find workspace root
    // CARGO_MANIFEST_DIR points to crates/terraphim_agent
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let manifest_path = PathBuf::from(manifest_dir);

    // Go up two levels: crates/terraphim_agent -> crates -> workspace_root
    let workspace_root = manifest_path
        .parent()
        .and_then(|p| p.parent())
        .ok_or("Cannot find workspace root from CARGO_MANIFEST_DIR")?;

    let kg_path = workspace_root.join("docs/src/kg");

    if !kg_path.exists() {
        return Err(format!(
            "KG path does not exist: {:?}\nworkspace_root: {:?}\nmanifest_dir: {:?}",
            kg_path, workspace_root, manifest_path
        )
        .into());
    }

    let logseq_builder = Logseq::default();
    let thesaurus = logseq_builder
        .build("test_role".to_string(), kg_path)
        .await?;

    Ok(thesaurus)
}

/// Perform replacement using the KG thesaurus
async fn replace_with_kg(
    text: &str,
    link_type: terraphim_automata::LinkType,
) -> Result<String, Box<dyn std::error::Error>> {
    let thesaurus = build_test_thesaurus().await?;
    let result = terraphim_automata::replace_matches(text, thesaurus, link_type)?;
    Ok(String::from_utf8(result)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    #[tokio::test]
    async fn test_replace_npm_to_bun() {
        let result = replace_with_kg("npm", terraphim_automata::LinkType::PlainText)
            .await
            .expect("Failed to perform replacement");

        assert!(
            result.contains("bun"),
            "Expected 'bun' in output, got: {}",
            result
        );
    }

    #[tokio::test]
    async fn test_replace_yarn_to_bun() {
        let result = replace_with_kg("yarn", terraphim_automata::LinkType::PlainText)
            .await
            .expect("Failed to perform replacement");

        assert!(
            result.contains("bun"),
            "Expected 'bun' in output, got: {}",
            result
        );
    }

    #[tokio::test]
    async fn test_replace_pnpm_install_to_bun() {
        let result = replace_with_kg("pnpm install", terraphim_automata::LinkType::PlainText)
            .await
            .expect("Failed to perform replacement");

        assert!(
            result.contains("bun_install"),
            "Expected 'bun_install' in output, got: {}",
            result
        );
    }

    #[tokio::test]
    async fn test_replace_yarn_install_to_bun() {
        let result = replace_with_kg("yarn install", terraphim_automata::LinkType::PlainText)
            .await
            .expect("Failed to perform replacement");

        assert!(
            result.contains("bun_install"),
            "Expected 'bun_install' in output, got: {}",
            result
        );
    }

    #[tokio::test]
    async fn test_replace_with_markdown_format() {
        let result = replace_with_kg("npm", terraphim_automata::LinkType::MarkdownLinks)
            .await
            .expect("Failed to perform replacement");

        assert!(
            result.contains("[bun]"),
            "Expected '[bun]' in markdown output, got: {}",
            result
        );
    }

    #[test]
    fn test_replace_help_output() {
        let output = Command::new("cargo")
            .args([
                "run",
                "--quiet",
                "-p",
                "terraphim_agent",
                "--bin",
                "terraphim-agent",
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
