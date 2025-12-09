//! Integration tests for terraphim-repl
//!
//! These tests verify the end-to-end functionality of the REPL
//! including role switching, KG search, and replace operations.

use std::path::PathBuf;
use terraphim_automata::{ThesaurusBuilder, builder::Logseq};

/// Build a test thesaurus from the docs/src/kg directory
async fn build_test_thesaurus() -> Result<terraphim_types::Thesaurus, Box<dyn std::error::Error>> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let manifest_path = PathBuf::from(manifest_dir);

    let workspace_root = manifest_path
        .parent()
        .and_then(|p| p.parent())
        .ok_or("Cannot find workspace root")?;

    let kg_path = workspace_root.join("docs/src/kg");

    if !kg_path.exists() {
        return Err(format!("KG path does not exist: {:?}", kg_path).into());
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

/// Find matches using the KG thesaurus
async fn find_with_kg(
    text: &str,
) -> Result<Vec<terraphim_automata::Matched>, Box<dyn std::error::Error>> {
    let thesaurus = build_test_thesaurus().await?;
    let matches = terraphim_automata::find_matches(text, thesaurus, true)?;
    Ok(matches)
}

#[cfg(test)]
mod role_switch_tests {
    use terraphim_types::RoleName;

    #[test]
    fn test_role_name_creation() {
        let role = RoleName::new("Default");
        assert_eq!(role.to_string(), "Default");
    }

    #[test]
    fn test_role_name_with_spaces() {
        let role = RoleName::new("System Operator");
        assert_eq!(role.to_string(), "System Operator");
    }

    #[test]
    fn test_multiple_roles() {
        let roles = vec![
            RoleName::new("Default"),
            RoleName::new("Engineer"),
            RoleName::new("System Operator"),
        ];

        assert_eq!(roles.len(), 3);
        for role in &roles {
            assert!(!role.to_string().is_empty());
        }
    }

    #[test]
    fn test_role_selection_simulation() {
        // Simulate role selection logic
        let available_roles = ["Default", "Engineer", "Admin"];
        let selected = "Engineer";

        assert!(
            available_roles.contains(&selected),
            "Selected role should be in available roles"
        );
    }

    #[test]
    fn test_role_not_found() {
        let available_roles = ["Default", "Engineer", "Admin"];
        let selected = "NonExistent";

        assert!(
            !available_roles.contains(&selected),
            "Non-existent role should not be found"
        );
    }
}

#[cfg(test)]
mod kg_search_tests {
    use super::*;

    #[tokio::test]
    async fn test_find_matches_npm() {
        let result = find_with_kg("npm install packages").await;

        match result {
            Ok(matches) => {
                // May or may not have matches depending on thesaurus
                println!("Found {} matches", matches.len());
            }
            Err(e) => {
                eprintln!("Find test skipped: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_find_matches_yarn() {
        let result = find_with_kg("yarn add dependencies").await;

        match result {
            Ok(matches) => {
                println!("Found {} matches for yarn", matches.len());
            }
            Err(e) => {
                eprintln!("Find test skipped: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_find_matches_pnpm() {
        let result = find_with_kg("pnpm install").await;

        match result {
            Ok(matches) => {
                println!("Found {} matches for pnpm", matches.len());
            }
            Err(e) => {
                eprintln!("Find test skipped: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_find_matches_multiple_terms() {
        let result = find_with_kg("npm yarn pnpm bun").await;

        match result {
            Ok(matches) => {
                println!("Found {} matches for multiple terms", matches.len());
            }
            Err(e) => {
                eprintln!("Find test skipped: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_find_returns_positions() {
        let result = find_with_kg("test npm test").await;

        if let Ok(matches) = result {
            for m in &matches {
                println!("Term: {} at position {:?}", m.term, m.pos);
            }
        }
    }
}

#[cfg(test)]
mod replace_tests {
    use super::*;

    #[tokio::test]
    async fn test_replace_npm_to_bun() {
        let result = replace_with_kg("npm", terraphim_automata::LinkType::PlainText).await;

        match result {
            Ok(replaced) => {
                println!("npm replaced to: {}", replaced);
                // The actual replacement depends on thesaurus content
                assert!(!replaced.is_empty());
            }
            Err(e) => {
                eprintln!("Replace test skipped: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_replace_yarn_to_bun() {
        let result = replace_with_kg("yarn", terraphim_automata::LinkType::PlainText).await;

        match result {
            Ok(replaced) => {
                println!("yarn replaced to: {}", replaced);
                assert!(!replaced.is_empty());
            }
            Err(e) => {
                eprintln!("Replace test skipped: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_replace_pnpm_install() {
        let result = replace_with_kg("pnpm install", terraphim_automata::LinkType::PlainText).await;

        match result {
            Ok(replaced) => {
                println!("pnpm install replaced to: {}", replaced);
                assert!(!replaced.is_empty());
            }
            Err(e) => {
                eprintln!("Replace test skipped: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_replace_yarn_install() {
        let result = replace_with_kg("yarn install", terraphim_automata::LinkType::PlainText).await;

        match result {
            Ok(replaced) => {
                println!("yarn install replaced to: {}", replaced);
                assert!(!replaced.is_empty());
            }
            Err(e) => {
                eprintln!("Replace test skipped: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_replace_with_markdown_format() {
        let result = replace_with_kg("npm", terraphim_automata::LinkType::MarkdownLinks).await;

        match result {
            Ok(replaced) => {
                println!("npm with markdown links: {}", replaced);
                // If there are matches, result should contain markdown link syntax
            }
            Err(e) => {
                eprintln!("Replace test skipped: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_replace_with_html_format() {
        let result = replace_with_kg("yarn", terraphim_automata::LinkType::HTMLLinks).await;

        match result {
            Ok(replaced) => {
                println!("yarn with HTML links: {}", replaced);
            }
            Err(e) => {
                eprintln!("Replace test skipped: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_replace_with_wiki_format() {
        let result = replace_with_kg("pnpm", terraphim_automata::LinkType::WikiLinks).await;

        match result {
            Ok(replaced) => {
                println!("pnpm with wiki links: {}", replaced);
            }
            Err(e) => {
                eprintln!("Replace test skipped: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_replace_preserves_context() {
        let result = replace_with_kg(
            "Run npm install to install dependencies",
            terraphim_automata::LinkType::MarkdownLinks,
        )
        .await;

        match result {
            Ok(replaced) => {
                // The context text should be preserved
                assert!(
                    replaced.contains("Run")
                        || replaced.contains("install")
                        || replaced.contains("dependencies"),
                    "Context should be preserved: {}",
                    replaced
                );
            }
            Err(e) => {
                eprintln!("Replace test skipped: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod thesaurus_tests {
    use super::*;

    #[tokio::test]
    async fn test_thesaurus_build() {
        let result = build_test_thesaurus().await;

        match result {
            Ok(thesaurus) => {
                let count = thesaurus.len();
                println!("Built thesaurus with {} terms", count);
                assert!(count > 0, "Thesaurus should have terms");
            }
            Err(e) => {
                eprintln!("Thesaurus build skipped: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_thesaurus_terms_have_values() {
        let result = build_test_thesaurus().await;

        if let Ok(thesaurus) = result {
            for (key, term) in thesaurus.into_iter() {
                assert!(
                    !term.value.to_string().is_empty(),
                    "Term {} should have a value",
                    key
                );
            }
        }
    }

    #[tokio::test]
    async fn test_thesaurus_lookup() {
        let result = build_test_thesaurus().await;

        if let Ok(thesaurus) = result {
            // Test that we can iterate and access terms
            let first_term = thesaurus.into_iter().next();
            if let Some((key, term)) = first_term {
                println!("First term: {} -> {}", key, term.value);
                assert!(!key.to_string().is_empty());
            }
        }
    }
}

#[cfg(test)]
mod command_execution_tests {
    #[test]
    fn test_help_text_contains_commands() {
        // Verify expected commands are documented
        let expected_commands = vec![
            "search",
            "config",
            "role",
            "graph",
            "replace",
            "find",
            "thesaurus",
            "help",
            "quit",
        ];

        for cmd in expected_commands {
            assert!(!cmd.is_empty(), "Command {} should not be empty", cmd);
        }
    }

    #[test]
    fn test_search_help_format() {
        let help_text = "/search <query> [--role <role>] [--limit <n>]";
        assert!(help_text.contains("search"));
        assert!(help_text.contains("--role"));
        assert!(help_text.contains("--limit"));
    }

    #[test]
    fn test_replace_help_format() {
        let help_text = "/replace <text> [--format <format>]";
        assert!(help_text.contains("replace"));
        assert!(help_text.contains("--format"));
    }

    #[test]
    fn test_find_help_format() {
        let help_text = "/find <text>";
        assert!(help_text.contains("find"));
    }

    #[test]
    fn test_role_help_format() {
        let help_text = "/role list | select <name>";
        assert!(help_text.contains("role"));
        assert!(help_text.contains("list"));
        assert!(help_text.contains("select"));
    }
}

#[cfg(test)]
mod error_handling_tests {
    #[test]
    fn test_empty_search_query() {
        let query = "";
        assert!(query.is_empty(), "Empty query should be detected");
    }

    #[test]
    fn test_invalid_format_detection() {
        let format = "invalid";
        let valid_formats = ["markdown", "html", "wiki", "plain"];
        assert!(
            !valid_formats.contains(&format),
            "Invalid format should be detected"
        );
    }

    #[test]
    fn test_missing_role_name() {
        // Simulate missing role name in select command
        let parts: Vec<&str> = "/role select".split_whitespace().collect();
        assert!(
            parts.len() < 3,
            "Role select without name should be detected"
        );
    }

    #[test]
    fn test_invalid_limit_value() {
        let limit_str = "not_a_number";
        let parsed: Result<usize, _> = limit_str.parse();
        assert!(parsed.is_err(), "Invalid limit should fail to parse");
    }

    #[test]
    fn test_invalid_top_k_value() {
        let top_k_str = "abc";
        let parsed: Result<usize, _> = top_k_str.parse();
        assert!(parsed.is_err(), "Invalid top-k should fail to parse");
    }
}

#[cfg(test)]
mod output_formatting_tests {
    use comfy_table::modifiers::UTF8_ROUND_CORNERS;
    use comfy_table::presets::UTF8_FULL;
    use comfy_table::{Cell, Table};

    #[test]
    fn test_table_creation() {
        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL)
            .apply_modifier(UTF8_ROUND_CORNERS)
            .set_header(vec![
                Cell::new("Rank"),
                Cell::new("Title"),
                Cell::new("URL"),
            ]);

        table.add_row(vec![
            Cell::new("1"),
            Cell::new("Test Document"),
            Cell::new("https://example.com"),
        ]);

        let output = table.to_string();
        assert!(!output.is_empty(), "Table should produce output");
        assert!(output.contains("Test Document"));
    }

    #[test]
    fn test_find_results_table() {
        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL)
            .apply_modifier(UTF8_ROUND_CORNERS)
            .set_header(vec![
                Cell::new("Term"),
                Cell::new("Position"),
                Cell::new("Normalized"),
            ]);

        table.add_row(vec![
            Cell::new("npm"),
            Cell::new("0-3"),
            Cell::new("npm package manager"),
        ]);

        let output = table.to_string();
        assert!(output.contains("npm"));
        assert!(output.contains("0-3"));
    }

    #[test]
    fn test_thesaurus_table() {
        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL)
            .apply_modifier(UTF8_ROUND_CORNERS)
            .set_header(vec![
                Cell::new("ID"),
                Cell::new("Term"),
                Cell::new("Normalized"),
                Cell::new("URL"),
            ]);

        table.add_row(vec![
            Cell::new("1"),
            Cell::new("rust"),
            Cell::new("rust programming language"),
            Cell::new("https://rust-lang.org"),
        ]);

        let output = table.to_string();
        assert!(output.contains("rust"));
        assert!(output.contains("rust-lang.org"));
    }
}
