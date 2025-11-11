// Code editing functionality for terraphim_automata
//
// This module provides multiple strategies for applying code edits, designed to work
// with LLM output that may not perfectly match existing code. Strategies are tried
// in order from most precise to most lenient.

use crate::{Result, TerraphimAutomataError};
use aho_corasick::{AhoCorasick, MatchKind};
use serde::{Deserialize, Serialize};
#[cfg(feature = "typescript")]
use tsify::Tsify;

/// Result of an edit operation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(Tsify))]
#[cfg_attr(feature = "typescript", tsify(into_wasm_abi, from_wasm_abi))]
pub struct EditResult {
    pub success: bool,
    pub strategy_used: String,
    pub original_content: String,
    pub modified_content: String,
    pub similarity_score: f64,
}

/// Strategies for applying edits
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(Tsify))]
#[cfg_attr(feature = "typescript", tsify(into_wasm_abi, from_wasm_abi))]
pub enum EditStrategy {
    /// Exact string matching using Aho-Corasick (fastest, most precise)
    Exact,
    /// Whitespace-flexible matching (ignores indentation differences)
    WhitespaceFlexible,
    /// Block anchor matching (matches first and last lines, validates middle)
    BlockAnchor,
    /// Fuzzy matching using Levenshtein distance
    Fuzzy,
}

/// Apply a search/replace edit using multiple fallback strategies
///
/// Tries strategies in order: Exact → WhitespaceFlexible → BlockAnchor → Fuzzy
/// Returns the first successful match or an error if all strategies fail.
pub fn apply_edit(content: &str, search: &str, replace: &str) -> Result<EditResult> {
    // Try each strategy in order
    let strategies = [
        EditStrategy::Exact,
        EditStrategy::WhitespaceFlexible,
        EditStrategy::BlockAnchor,
        EditStrategy::Fuzzy,
    ];

    for strategy in strategies {
        match apply_edit_with_strategy(content, search, replace, strategy) {
            Ok(result) if result.success => {
                return Ok(result);
            }
            _ => continue,
        }
    }

    Err(TerraphimAutomataError::Dict(format!(
        "Failed to apply edit: all strategies failed to match search block"
    )))
}

/// Apply edit using a specific strategy
pub fn apply_edit_with_strategy(
    content: &str,
    search: &str,
    replace: &str,
    strategy: EditStrategy,
) -> Result<EditResult> {
    match strategy {
        EditStrategy::Exact => apply_edit_exact(content, search, replace),
        EditStrategy::WhitespaceFlexible => {
            apply_edit_whitespace_flexible(content, search, replace)
        }
        EditStrategy::BlockAnchor => apply_edit_block_anchor(content, search, replace, 0.3),
        EditStrategy::Fuzzy => apply_edit_fuzzy(content, search, replace, 0.8),
    }
}

/// Strategy 1: Exact match using Aho-Corasick (fastest - nanoseconds)
fn apply_edit_exact(content: &str, search: &str, replace: &str) -> Result<EditResult> {
    // Use Aho-Corasick for exact matching (fastest)
    let ac = AhoCorasick::builder()
        .match_kind(MatchKind::LeftmostLongest)
        .build(&[search])?;

    if ac.is_match(content) {
        // Replace first occurrence
        let modified = content.replacen(search, replace, 1);

        Ok(EditResult {
            success: true,
            strategy_used: "exact".to_string(),
            original_content: content.to_string(),
            modified_content: modified,
            similarity_score: 1.0,
        })
    } else {
        Ok(EditResult {
            success: false,
            strategy_used: "exact".to_string(),
            original_content: content.to_string(),
            modified_content: content.to_string(),
            similarity_score: 0.0,
        })
    }
}

/// Strategy 2: Whitespace-flexible matching
///
/// Normalizes whitespace for comparison but preserves original indentation
fn apply_edit_whitespace_flexible(
    content: &str,
    search: &str,
    replace: &str,
) -> Result<EditResult> {
    let content_lines: Vec<&str> = content.lines().collect();
    let search_lines: Vec<&str> = search.lines().collect();

    if search_lines.is_empty() {
        return Ok(EditResult {
            success: false,
            strategy_used: "whitespace-flexible".to_string(),
            original_content: content.to_string(),
            modified_content: content.to_string(),
            similarity_score: 0.0,
        });
    }

    // Try to find search block with flexible whitespace
    for i in 0..=content_lines.len().saturating_sub(search_lines.len()) {
        if lines_match_flexible(&content_lines[i..i + search_lines.len()], &search_lines) {
            // Found match - preserve original indentation
            let indentation = get_indentation(content_lines[i]);
            let replace_lines: Vec<&str> = replace.lines().collect();
            let indented_replace = apply_indentation(&replace_lines, &indentation);

            let mut new_lines = content_lines[..i].to_vec();
            new_lines.extend(indented_replace.iter().map(|s| s.as_str()));
            new_lines.extend(&content_lines[i + search_lines.len()..]);

            let modified = new_lines.join("\n");

            return Ok(EditResult {
                success: true,
                strategy_used: "whitespace-flexible".to_string(),
                original_content: content.to_string(),
                modified_content: modified,
                similarity_score: 0.95,
            });
        }
    }

    Ok(EditResult {
        success: false,
        strategy_used: "whitespace-flexible".to_string(),
        original_content: content.to_string(),
        modified_content: content.to_string(),
        similarity_score: 0.0,
    })
}

/// Strategy 3: Block anchor matching
///
/// Matches using first and last lines as anchors, validates middle content
/// with Levenshtein distance.
///
/// # Arguments
/// * `threshold` - Minimum similarity (0.0-1.0). Lower threshold = more lenient.
///                 - Single candidate: 0.0 (very lenient)
///                 - Multiple candidates: 0.3 (stricter)
pub fn apply_edit_block_anchor(
    content: &str,
    search: &str,
    replace: &str,
    threshold: f64,
) -> Result<EditResult> {
    let content_lines: Vec<&str> = content.lines().collect();
    let search_lines: Vec<&str> = search.lines().collect();

    if search_lines.len() < 2 {
        // Need at least 2 lines for anchor matching
        return Ok(EditResult {
            success: false,
            strategy_used: "block-anchor".to_string(),
            original_content: content.to_string(),
            modified_content: content.to_string(),
            similarity_score: 0.0,
        });
    }

    let first_line = search_lines[0].trim();
    let last_line = search_lines[search_lines.len() - 1].trim();

    // Find all positions where first line matches
    let mut candidates = Vec::new();

    for i in 0..content_lines.len() {
        if content_lines[i].trim() == first_line {
            // Check if last line matches at expected position
            let expected_last = i + search_lines.len() - 1;
            if expected_last < content_lines.len()
                && content_lines[expected_last].trim() == last_line
            {
                // Calculate similarity of the entire block
                let block = content_lines[i..=expected_last].join("\n");
                let similarity = levenshtein_similarity(&block, search);

                if similarity >= threshold {
                    candidates.push((i, expected_last, similarity));
                }
            }
        }
    }

    if candidates.is_empty() {
        return Ok(EditResult {
            success: false,
            strategy_used: "block-anchor".to_string(),
            original_content: content.to_string(),
            modified_content: content.to_string(),
            similarity_score: 0.0,
        });
    }

    // Use best match if multiple candidates
    let (start, end, similarity) = if candidates.len() == 1 {
        candidates[0]
    } else {
        *candidates
            .iter()
            .max_by(|a, b| a.2.partial_cmp(&b.2).unwrap())
            .unwrap()
    };

    // Apply replacement
    let indentation = get_indentation(content_lines[start]);
    let replace_lines: Vec<&str> = replace.lines().collect();
    let indented_replace = apply_indentation(&replace_lines, &indentation);

    let mut new_lines = content_lines[..start].to_vec();
    new_lines.extend(indented_replace.iter().map(|s| s.as_str()));
    new_lines.extend(&content_lines[end + 1..]);

    let modified = new_lines.join("\n");

    Ok(EditResult {
        success: true,
        strategy_used: "block-anchor".to_string(),
        original_content: content.to_string(),
        modified_content: modified,
        similarity_score: similarity,
    })
}

/// Strategy 4: Fuzzy matching using Levenshtein distance
///
/// # Arguments
/// * `threshold` - Minimum similarity (0.0-1.0), typically 0.8
pub fn apply_edit_fuzzy(
    content: &str,
    search: &str,
    replace: &str,
    threshold: f64,
) -> Result<EditResult> {
    let content_lines: Vec<&str> = content.lines().collect();
    let search_lines: Vec<&str> = search.lines().collect();

    if search_lines.is_empty() {
        return Ok(EditResult {
            success: false,
            strategy_used: "fuzzy".to_string(),
            original_content: content.to_string(),
            modified_content: content.to_string(),
            similarity_score: 0.0,
        });
    }

    let mut best_match: Option<(usize, f64)> = None;

    // Sliding window to find best match
    for i in 0..=content_lines.len().saturating_sub(search_lines.len()) {
        let block = content_lines[i..i + search_lines.len()].join("\n");
        let similarity = levenshtein_similarity(&block, search);

        if similarity >= threshold {
            if let Some((_, best_sim)) = best_match {
                if similarity > best_sim {
                    best_match = Some((i, similarity));
                }
            } else {
                best_match = Some((i, similarity));
            }
        }
    }

    if let Some((start, similarity)) = best_match {
        let indentation = get_indentation(content_lines[start]);
        let replace_lines: Vec<&str> = replace.lines().collect();
        let indented_replace = apply_indentation(&replace_lines, &indentation);

        let mut new_lines = content_lines[..start].to_vec();
        new_lines.extend(indented_replace.iter().map(|s| s.as_str()));
        new_lines.extend(&content_lines[start + search_lines.len()..]);

        let modified = new_lines.join("\n");

        Ok(EditResult {
            success: true,
            strategy_used: "fuzzy".to_string(),
            original_content: content.to_string(),
            modified_content: modified,
            similarity_score: similarity,
        })
    } else {
        Ok(EditResult {
            success: false,
            strategy_used: "fuzzy".to_string(),
            original_content: content.to_string(),
            modified_content: content.to_string(),
            similarity_score: 0.0,
        })
    }
}

// Helper functions

/// Check if lines match with flexible whitespace
fn lines_match_flexible(content_lines: &[&str], search_lines: &[&str]) -> bool {
    if content_lines.len() != search_lines.len() {
        return false;
    }

    content_lines
        .iter()
        .zip(search_lines.iter())
        .all(|(c, s)| c.trim() == s.trim())
}

/// Get the indentation (leading whitespace) from a line
fn get_indentation(line: &str) -> String {
    line.chars().take_while(|c| c.is_whitespace()).collect()
}

/// Apply indentation to multiple lines
fn apply_indentation(lines: &[&str], indentation: &str) -> Vec<String> {
    lines
        .iter()
        .map(|line| {
            if line.trim().is_empty() {
                String::new()
            } else {
                format!("{}{}", indentation, line.trim_start())
            }
        })
        .collect()
}

/// Calculate Levenshtein similarity (0.0 to 1.0)
///
/// Uses Levenshtein distance normalized by the maximum length.
/// Returns 1.0 for identical strings, 0.0 for completely different.
pub fn levenshtein_similarity(s1: &str, s2: &str) -> f64 {
    let distance = levenshtein_distance(s1, s2);
    let max_len = s1.len().max(s2.len());

    if max_len == 0 {
        return 1.0;
    }

    1.0 - (distance as f64 / max_len as f64)
}

/// Calculate Levenshtein distance between two strings
///
/// This is the minimum number of single-character edits (insertions, deletions,
/// or substitutions) required to change one string into the other.
pub fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let len1 = s1.len();
    let len2 = s2.len();

    // Early returns for empty strings
    if len1 == 0 {
        return len2;
    }
    if len2 == 0 {
        return len1;
    }

    // Create matrix for dynamic programming
    let mut matrix: Vec<Vec<usize>> = vec![vec![0; len2 + 1]; len1 + 1];

    // Initialize first column and row
    for i in 0..=len1 {
        matrix[i][0] = i;
    }
    for j in 0..=len2 {
        matrix[0][j] = j;
    }

    // Fill matrix
    for (i, c1) in s1.chars().enumerate() {
        for (j, c2) in s2.chars().enumerate() {
            let cost = if c1 == c2 { 0 } else { 1 };

            matrix[i + 1][j + 1] = (matrix[i][j + 1] + 1) // deletion
                .min(matrix[i + 1][j] + 1) // insertion
                .min(matrix[i][j] + cost); // substitution
        }
    }

    matrix[len1][len2]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_match() {
        let content = "fn main() {\n    println!(\"Hello\");\n}";
        let search = "    println!(\"Hello\");";
        let replace = "    println!(\"Hello, World!\");";

        let result = apply_edit_exact(content, search, replace).unwrap();
        assert!(result.success);
        assert_eq!(result.strategy_used, "exact");
        assert!(result.modified_content.contains("Hello, World!"));
    }

    #[test]
    fn test_whitespace_flexible() {
        let content = "fn main() {\n    println!(\"Hello\");\n}";
        let search = "println!(\"Hello\");"; // No indentation in search
        let replace = "println!(\"Hello, World!\");";

        let result = apply_edit_whitespace_flexible(content, search, replace).unwrap();
        assert!(result.success);
        assert_eq!(result.strategy_used, "whitespace-flexible");
        assert!(result.modified_content.contains("Hello, World!"));
        // Verify indentation is preserved
        assert!(result.modified_content.contains("    println!"));
    }

    #[test]
    fn test_block_anchor_match() {
        let content = r#"fn main() {
    let x = 1;
    let y = 2;
    let z = 3;
}"#;

        // Search block with slightly different middle content
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
        assert!(result.modified_content.contains("let y = 20"));
    }

    #[test]
    fn test_fuzzy_match() {
        let content = "fn main() {\n    println!(\"Hello, World!\");\n}";
        // Slightly different search (typo)
        let search = "fn main() {\n    printlin!(\"Hello, World!\");\n}";
        let replace = "fn main() {\n    println!(\"Goodbye!\");\n}";

        let result = apply_edit_fuzzy(content, search, replace, 0.8).unwrap();
        assert!(result.success);
        assert_eq!(result.strategy_used, "fuzzy");
        assert!(result.modified_content.contains("Goodbye!"));
    }

    #[test]
    fn test_apply_edit_multi_strategy() {
        let content = "fn main() {\n    println!(\"Hello\");\n}";
        let search = "println!(\"Hello\");"; // No indentation
        let replace = "println!(\"Hi\");";

        // Should succeed with whitespace-flexible strategy
        let result = apply_edit(content, search, replace).unwrap();
        assert!(result.success);
        assert!(result.modified_content.contains("Hi"));
    }

    #[test]
    fn test_levenshtein_distance() {
        assert_eq!(levenshtein_distance("", ""), 0);
        assert_eq!(levenshtein_distance("abc", "abc"), 0);
        assert_eq!(levenshtein_distance("abc", "def"), 3);
        assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
        assert_eq!(levenshtein_distance("saturday", "sunday"), 3);
    }

    #[test]
    fn test_levenshtein_similarity() {
        assert_eq!(levenshtein_similarity("abc", "abc"), 1.0);
        assert!(levenshtein_similarity("kitten", "sitting") > 0.5);
        assert!(levenshtein_similarity("abc", "def") < 0.5);
    }

    #[test]
    fn test_get_indentation() {
        assert_eq!(get_indentation("    code"), "    ");
        assert_eq!(get_indentation("\tcode"), "\t");
        assert_eq!(get_indentation("code"), "");
        assert_eq!(get_indentation("  \t  code"), "  \t  ");
    }

    #[test]
    fn test_apply_indentation() {
        let lines = vec!["line1", "line2", "line3"];
        let indented = apply_indentation(&lines, "  ");

        assert_eq!(indented, vec!["  line1", "  line2", "  line3"]);
    }
}
