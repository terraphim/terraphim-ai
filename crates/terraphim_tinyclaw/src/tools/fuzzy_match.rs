//! Fuzzy matching for file operations.
//!
//! Port of Hermes `tools/fuzzy_match.py`: a multi-strategy find/replace chain
//! that tolerates whitespace, indentation, and escaping differences common in
//! LLM-generated edits. Strategy order matches the Hermes implementation.

/// A match-finding strategy: `(content, pattern) -> byte ranges`.
pub type MatchFinder = fn(&str, &str) -> Vec<(usize, usize)>;

/// Find and replace text using a chain of increasingly fuzzy strategies.
///
/// Returns `(new_content, match_count, error)`:
/// - success: `(modified, count, None)`
/// - failure: `(original, 0, Some(reason))`
pub fn fuzzy_find_and_replace(
    content: &str,
    old_string: &str,
    new_string: &str,
    replace_all: bool,
) -> (String, usize, Option<String>) {
    if old_string.is_empty() {
        return (
            content.to_string(),
            0,
            Some("old_string cannot be empty".into()),
        );
    }
    if old_string == new_string {
        return (
            content.to_string(),
            0,
            Some("old_string and new_string are identical".into()),
        );
    }

    let strategies: Vec<MatchFinder> = vec![
        strategy_exact,
        strategy_line_trimmed,
        strategy_whitespace_normalized,
        strategy_indentation_flexible,
        strategy_escape_normalized,
        strategy_trimmed_boundary,
        strategy_block_anchor,
        strategy_context_aware,
    ];

    for strategy in strategies {
        let matches = strategy(content, old_string);
        if !matches.is_empty() {
            if matches.len() > 1 && !replace_all {
                return (
                    content.to_string(),
                    0,
                    Some(format!(
                        "Found {} matches for old_string. Provide more context to make it \
                         unique, or use replace_all=true.",
                        matches.len()
                    )),
                );
            }
            let new_content = apply_replacements(content, &matches, new_string);
            return (new_content, matches.len(), None);
        }
    }

    (
        content.to_string(),
        0,
        Some("Could not find a match for old_string in the file".into()),
    )
}

/// Apply replacements at the given byte ranges, from end to start.
fn apply_replacements(content: &str, matches: &[(usize, usize)], new_string: &str) -> String {
    let mut sorted: Vec<(usize, usize)> = matches.to_vec();
    sorted.sort_by_key(|(start, _)| *start);
    sorted.reverse();

    let mut result = content.to_string();
    for (start, end) in sorted {
        result.replace_range(start..end, new_string);
    }
    result
}

// ---------------------------------------------------------------------------
// Similarity (Levenshtein-based ratio; approximates difflib SequenceMatcher)
// ---------------------------------------------------------------------------

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut curr = vec![0usize; b.len() + 1];
    for i in 1..=a.len() {
        curr[0] = i;
        for j in 1..=b.len() {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            curr[j] = (prev[j] + 1).min(curr[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[b.len()]
}

/// Similarity ratio in `[0, 1]`. `1.0` for identical strings, `0.0` for
/// maximally different. Approximates `difflib.SequenceMatcher.ratio()`.
pub fn ratio(a: &str, b: &str) -> f64 {
    let alen = a.chars().count();
    let blen = b.chars().count();
    let max = alen.max(blen);
    if max == 0 {
        return 1.0;
    }
    let d = levenshtein(a, b);
    1.0 - (d as f64 / max as f64)
}

// ---------------------------------------------------------------------------
// Byte-range helpers for line-based strategies
// ---------------------------------------------------------------------------

/// Byte offset of each line's first character.
fn line_starts(content: &str) -> Vec<usize> {
    let mut starts = vec![0usize];
    for (i, b) in content.bytes().enumerate() {
        if b == b'\n' {
            starts.push(i + 1);
        }
    }
    starts
}

/// Byte range `[start, end)` covering `lines[i .. i+n]`, ending at the last
/// line's content (excluding its trailing newline).
fn window_range(starts: &[usize], line_lens: &[usize], i: usize, n: usize) -> (usize, usize) {
    let start = starts[i];
    let end = starts[i + n - 1] + line_lens[i + n - 1];
    (start, end)
}

/// Match a normalized line window against a normalized pattern, returning
/// original byte ranges.
fn line_window_matches(
    content: &str,
    norm: impl Fn(&str) -> String,
    pattern: &str,
) -> Vec<(usize, usize)> {
    let content_lines: Vec<&str> = content.split('\n').collect();
    let pattern_lines: Vec<&str> = pattern.split('\n').collect();
    let n = pattern_lines.len();
    if n == 0 || content_lines.len() < n {
        return Vec::new();
    }

    let starts = line_starts(content);
    let line_lens: Vec<usize> = content_lines.iter().map(|l| l.len()).collect();
    let norm_pattern: Vec<String> = pattern_lines.iter().map(|l| norm(l)).collect();

    let mut matches = Vec::new();
    for i in 0..=(content_lines.len() - n) {
        let norm_block: Vec<String> = content_lines[i..i + n].iter().map(|l| norm(l)).collect();
        if norm_block == norm_pattern {
            matches.push(window_range(&starts, &line_lens, i, n));
        }
    }
    matches
}

// ---------------------------------------------------------------------------
// Strategies
// ---------------------------------------------------------------------------

/// Strategy 1: exact string match (all occurrences, including overlaps).
fn strategy_exact(content: &str, pattern: &str) -> Vec<(usize, usize)> {
    let mut matches = Vec::new();
    let mut start = 0usize;
    while let Some(pos) = content[start..].find(pattern) {
        let abs = start + pos;
        matches.push((abs, abs + pattern.len()));
        start = abs + 1;
    }
    matches
}

/// Strategy 2: match with per-line whitespace trimming.
fn strategy_line_trimmed(content: &str, pattern: &str) -> Vec<(usize, usize)> {
    line_window_matches(content, |l| l.trim().to_string(), pattern)
}

/// Strategy 3: collapse multiple spaces/tabs to a single space.
fn strategy_whitespace_normalized(content: &str, pattern: &str) -> Vec<(usize, usize)> {
    fn normalize(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        let mut in_space = false;
        for c in s.chars() {
            if c == ' ' || c == '\t' {
                if !in_space {
                    out.push(' ');
                    in_space = true;
                }
            } else {
                out.push(c);
                in_space = false;
            }
        }
        out
    }
    line_window_matches(content, normalize, pattern)
}

/// Strategy 4: ignore leading indentation entirely.
fn strategy_indentation_flexible(content: &str, pattern: &str) -> Vec<(usize, usize)> {
    line_window_matches(content, |l| l.trim_start().to_string(), pattern)
}

/// Strategy 5: convert `\n`/`\t`/`\r` escape sequences to real characters.
fn strategy_escape_normalized(content: &str, pattern: &str) -> Vec<(usize, usize)> {
    fn unescape(s: &str) -> String {
        s.replace("\\n", "\n")
            .replace("\\t", "\t")
            .replace("\\r", "\r")
    }
    let pattern_unescaped = unescape(pattern);
    if pattern_unescaped == pattern {
        return Vec::new();
    }
    strategy_exact(content, &pattern_unescaped)
}

/// Strategy 6: trim whitespace from the first and last lines only.
fn strategy_trimmed_boundary(content: &str, pattern: &str) -> Vec<(usize, usize)> {
    let pattern_lines: Vec<&str> = pattern.split('\n').collect();
    if pattern_lines.is_empty() {
        return Vec::new();
    }
    let mut modified: Vec<String> = pattern_lines.iter().map(|l| l.to_string()).collect();
    modified[0] = modified[0].trim().to_string();
    if modified.len() > 1 {
        let last = modified.len() - 1;
        modified[last] = modified[last].trim().to_string();
    }

    let content_lines: Vec<&str> = content.split('\n').collect();
    let n = pattern_lines.len();
    if content_lines.len() < n {
        return Vec::new();
    }
    let starts = line_starts(content);
    let line_lens: Vec<usize> = content_lines.iter().map(|l| l.len()).collect();

    let mut matches = Vec::new();
    for i in 0..=(content_lines.len() - n) {
        let mut check: Vec<String> = content_lines[i..i + n]
            .iter()
            .map(|l| l.to_string())
            .collect();
        check[0] = check[0].trim().to_string();
        if check.len() > 1 {
            let last = check.len() - 1;
            check[last] = check[last].trim().to_string();
        }
        if check == modified {
            matches.push(window_range(&starts, &line_lens, i, n));
        }
    }
    matches
}

/// Strategy 7: anchor on first + last lines, accept middle at 70% similarity.
fn strategy_block_anchor(content: &str, pattern: &str) -> Vec<(usize, usize)> {
    let pattern_lines: Vec<&str> = pattern.split('\n').collect();
    if pattern_lines.len() < 2 {
        return Vec::new();
    }
    let first = pattern_lines[0].trim();
    let last = pattern_lines[pattern_lines.len() - 1].trim();

    let content_lines: Vec<&str> = content.split('\n').collect();
    let n = pattern_lines.len();
    if content_lines.len() < n {
        return Vec::new();
    }
    let starts = line_starts(content);
    let line_lens: Vec<usize> = content_lines.iter().map(|l| l.len()).collect();

    let mut matches = Vec::new();
    for i in 0..=(content_lines.len() - n) {
        if content_lines[i].trim() == first && content_lines[i + n - 1].trim() == last {
            let similarity = if n <= 2 {
                1.0
            } else {
                let content_middle = content_lines[i + 1..i + n - 1].join("\n");
                let pattern_middle = pattern_lines[1..n - 1].join("\n");
                ratio(&content_middle, &pattern_middle)
            };
            if similarity >= 0.70 {
                matches.push(window_range(&starts, &line_lens, i, n));
            }
        }
    }
    matches
}

/// Strategy 8: line-by-line similarity with 50% high-similarity threshold.
fn strategy_context_aware(content: &str, pattern: &str) -> Vec<(usize, usize)> {
    let pattern_lines: Vec<&str> = pattern.split('\n').collect();
    let content_lines: Vec<&str> = content.split('\n').collect();
    if pattern_lines.is_empty() {
        return Vec::new();
    }
    let n = pattern_lines.len();
    if content_lines.len() < n {
        return Vec::new();
    }
    let starts = line_starts(content);
    let line_lens: Vec<usize> = content_lines.iter().map(|l| l.len()).collect();

    let mut matches = Vec::new();
    for i in 0..=(content_lines.len() - n) {
        let block = &content_lines[i..i + n];
        let mut high_similarity = 0usize;
        for (p_line, c_line) in pattern_lines.iter().zip(block.iter()) {
            if ratio(p_line.trim(), c_line.trim()) >= 0.80 {
                high_similarity += 1;
            }
        }
        if (high_similarity as f64) >= (n as f64) * 0.5 {
            matches.push(window_range(&starts, &line_lens, i, n));
        }
    }
    matches
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_match_single() {
        let (out, count, err) =
            fuzzy_find_and_replace("def foo():\n    pass", "def foo():", "def bar():", false);
        assert_eq!(count, 1);
        assert!(err.is_none());
        assert!(out.contains("def bar():"));
    }

    #[test]
    fn exact_match_multi_requires_replace_all() {
        let content = "x = 1\nx = 2\n";
        let (_, count, err) = fuzzy_find_and_replace(content, "x =", "y =", false);
        assert_eq!(count, 0);
        assert!(err.unwrap().contains("2 matches"));

        let (out2, count2, _) = fuzzy_find_and_replace(content, "x =", "y =", true);
        assert_eq!(count2, 2);
        assert!(!out2.contains("x ="));
    }

    #[test]
    fn line_trimmed_matches_whitespace_diff() {
        let content = "   def foo():   \n";
        let (out, count, err) = fuzzy_find_and_replace(content, "def foo():", "def bar():", false);
        assert_eq!(count, 1);
        assert!(err.is_none());
        assert!(out.contains("def bar():"));
    }

    #[test]
    fn whitespace_normalized_matches_multiple_spaces() {
        let content = "a  +   b = c";
        let (out, count, _) = fuzzy_find_and_replace(content, "a + b = c", "x", false);
        assert_eq!(count, 1);
        assert_eq!(out, "x");
    }

    #[test]
    fn indentation_flexible_matches_indent_diff() {
        let content = "    pass\n";
        let (out, count, _) = fuzzy_find_and_replace(content, "pass", "return", false);
        assert_eq!(count, 1);
        assert!(out.contains("return"));
    }

    #[test]
    fn escape_normalized_matches_literal_backslash_n() {
        let content = "line1\nline2\n";
        // Pattern with a literal backslash-n escape.
        let (_, count, _) = fuzzy_find_and_replace(content, "line1\\nline2", "x", false);
        assert_eq!(count, 1);
    }

    #[test]
    fn no_match_returns_error() {
        let (out, count, err) = fuzzy_find_and_replace("hello", "goodbye", "x", false);
        assert_eq!(count, 0);
        assert_eq!(out, "hello");
        assert!(err.is_some());
    }

    #[test]
    fn empty_old_string_is_error() {
        let (_, _, err) = fuzzy_find_and_replace("hello", "", "x", false);
        assert_eq!(err.unwrap(), "old_string cannot be empty");
    }

    #[test]
    fn identical_strings_is_error() {
        let (_, _, err) = fuzzy_find_and_replace("hello", "hello", "hello", false);
        assert!(err.unwrap().contains("identical"));
    }

    #[test]
    fn ratio_identical_is_one() {
        assert_eq!(ratio("abc", "abc"), 1.0);
        assert_eq!(ratio("", ""), 1.0);
    }

    #[test]
    fn ratio_different_lower() {
        assert!(ratio("abc", "xyz") < 0.5);
        assert!(ratio("kitten", "sitting") > 0.5);
    }

    #[test]
    fn block_anchor_accepts_similar_middle() {
        let content = "def foo():\n    x = 1\n    return x\n";
        let pattern = "def foo():\n    x = 2\n    return x\n";
        let (out, count, _) = fuzzy_find_and_replace(content, pattern, "def bar():", false);
        assert_eq!(count, 1);
        assert!(out.contains("def bar():"));
    }
}
