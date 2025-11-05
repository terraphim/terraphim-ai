// Integration tests for file editing MCP tools
//
// Tests the new code editing capabilities added in Phase 1 of the
// Code Assistant project (#271)

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to create a test file and return its path
fn create_test_file(dir: &TempDir, name: &str, content: &str) -> PathBuf {
    let file_path = dir.path().join(name);
    fs::write(&file_path, content).unwrap();
    file_path
}

/// Helper to read file content
fn read_file(path: &PathBuf) -> String {
    fs::read_to_string(path).unwrap()
}

#[tokio::test]
async fn test_edit_file_search_replace_exact_match() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = create_test_file(
        &temp_dir,
        "test.rs",
        r#"fn main() {
    println!("Hello");
}
"#,
    );

    // Note: This test requires a full ConfigState setup
    // For now, we test the edit strategy directly via terraphim_automata
    use terraphim_automata::apply_edit;

    let content = read_file(&test_file);
    let search = r#"    println!("Hello");"#;
    let replace = r#"    println!("Hello, World!");"#;

    let result = apply_edit(&content, search, replace).unwrap();

    assert!(result.success);
    assert_eq!(result.strategy_used, "exact");
    assert!(result.modified_content.contains("Hello, World!"));
}

#[tokio::test]
async fn test_edit_file_whitespace_flexible() {
    use terraphim_automata::apply_edit;

    // Content with specific indentation
    let content = r#"fn main() {
    println!("Hello");
}
"#;

    // Search without indentation - should use whitespace-flexible or exact
    let search = r#"println!("Hello");"#;
    let replace = r#"println!("Goodbye");"#;

    let result = apply_edit(content, search, replace).unwrap();

    assert!(result.success);
    // Strategy could be exact or whitespace-flexible, both work
    assert!(result.modified_content.contains("Goodbye"));
    // Verify indentation is preserved
    assert!(result.modified_content.contains("    println!"));
}

#[tokio::test]
async fn test_edit_file_block_anchor() {
    use terraphim_automata::apply_edit_block_anchor;

    let content = r#"fn main() {
    let x = 1;
    let y = 2;
    let z = 3;
}
"#;

    // Search block with slightly different middle line (tests block-anchor specifically)
    let search = r#"fn main() {
    let x = 2;
    let y = 2;
    let z = 3;
}"#;
    let replace = r#"fn main() {
    let x = 10;
    let y = 20;
    let z = 30;
}"#;

    let result = apply_edit_block_anchor(content, search, replace, 0.3).unwrap();

    assert!(result.success);
    assert_eq!(result.strategy_used, "block-anchor");
    assert!(result.modified_content.contains("let x = 10"));
}

#[tokio::test]
async fn test_edit_file_fuzzy_match() {
    use terraphim_automata::{apply_edit, apply_edit_with_strategy, EditStrategy};

    let content = r#"fn greet(name: &str) {
    println!("Hello, {}!", name);
}
"#;

    // Search with typo (should match with fuzzy)
    let search = r#"fn greet(name: &str) {
    printlin!("Hello, {}!", name);
}
"#;
    let replace = r#"fn greet(name: &str) {
    println!("Hi, {}!", name);
}
"#;

    // Test with fuzzy strategy specifically
    let result = apply_edit_with_strategy(content, search, replace, EditStrategy::Fuzzy).unwrap();

    assert!(result.success);
    assert_eq!(result.strategy_used, "fuzzy");
    assert!(result.modified_content.contains("Hi,"));
    assert!(result.similarity_score >= 0.8);

    // Also test that apply_edit (multi-strategy) works
    let result2 = apply_edit(content, search, replace).unwrap();
    assert!(result2.success);
    assert!(result2.modified_content.contains("Hi,"));
}

#[tokio::test]
async fn test_edit_strategy_fallback_chain() {
    // Test that multiple strategies work for different cases

    use terraphim_automata::apply_edit;

    let content = r#"fn main() {
    let x = 1;
    let y = 2;
}
"#;

    // Test 1: Exact match succeeds immediately
    let search_exact = "    let x = 1;";
    let replace = "    let x = 10;";
    let result = apply_edit(content, search_exact, replace).unwrap();
    assert!(result.success);
    assert!(result.modified_content.contains("let x = 10"));

    // Test 2: Different search patterns all work
    let search2 = "let x = 1;"; // No indentation - should still work
    let result2 = apply_edit(content, search2, replace).unwrap();
    assert!(result2.success);

    // Test 3: Verify that edits actually modify the content correctly
    assert!(result2.modified_content.contains("let x = 10"));
    assert!(!result2.modified_content.contains("let x = 1;"));
}

#[tokio::test]
async fn test_edit_preserves_file_structure() {
    // Test that editing preserves surrounding code

    use terraphim_automata::apply_edit;

    let content = r#"// Header comment
fn function_one() {
    println!("One");
}

fn function_two() {
    println!("Two");
}

fn function_three() {
    println!("Three");
}
"#;

    let search = r#"fn function_two() {
    println!("Two");
}"#;
    let replace = r#"fn function_two() {
    println!("Updated Two");
}"#;

    let result = apply_edit(content, search, replace).unwrap();

    assert!(result.success);
    // Verify surrounding functions are unchanged
    assert!(result.modified_content.contains("function_one"));
    assert!(result.modified_content.contains("function_three"));
    assert!(result.modified_content.contains("Header comment"));
    // Verify only target function changed
    assert!(result.modified_content.contains("Updated Two"));
    assert!(!result.modified_content.contains(r#"println!("Two")"#));
}

#[tokio::test]
async fn test_edit_with_complex_indentation() {
    use terraphim_automata::apply_edit;

    let content = r#"impl MyStruct {
    fn method_one(&self) {
        if condition {
            nested_call();
        }
    }
}
"#;

    // Search without proper indentation
    let search = r#"if condition {
    nested_call();
}"#;
    let replace = r#"if condition {
    enhanced_call();
}"#;

    let result = apply_edit(content, search, replace).unwrap();

    assert!(result.success);
    assert!(result.modified_content.contains("enhanced_call"));
    // Verify the edit worked (indentation handling is strategy-dependent)
    assert!(!result.modified_content.contains("nested_call"));
}

#[tokio::test]
async fn test_levenshtein_distance_edge_cases() {
    use terraphim_automata::levenshtein_distance;

    assert_eq!(levenshtein_distance("", ""), 0);
    assert_eq!(levenshtein_distance("abc", ""), 3);
    assert_eq!(levenshtein_distance("", "abc"), 3);
    assert_eq!(levenshtein_distance("same", "same"), 0);
    assert_eq!(levenshtein_distance("abc", "def"), 3);
    assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
}

#[tokio::test]
async fn test_levenshtein_similarity_ranges() {
    use terraphim_automata::levenshtein_similarity;

    // Perfect match
    assert_eq!(levenshtein_similarity("hello", "hello"), 1.0);

    // Very similar (one character difference)
    let sim = levenshtein_similarity("hello", "helo");
    assert!((0.79..1.0).contains(&sim)); // 4/5 = 0.8

    // Somewhat similar (one character substitution)
    let sim = levenshtein_similarity("hello", "hallo");
    assert!((0.79..1.0).contains(&sim)); // 4/5 = 0.8

    // Very different
    let sim = levenshtein_similarity("hello", "world");
    assert!(sim < 0.5);
}
