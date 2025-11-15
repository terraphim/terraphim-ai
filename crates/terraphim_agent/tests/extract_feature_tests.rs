/// Extract clean output without log messages
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
                && !line.trim().is_empty()
        })
        .collect::<Vec<&str>>()
        .join("\n")
}

/// Check if output contains successful extract results
fn has_successful_extract_results(output: &str) -> bool {
    output.contains("Found") && output.contains("paragraph") && output.contains("Match")
}

/// Extract found term names from output
fn extract_found_terms(output: &str) -> Vec<String> {
    let mut terms = Vec::new();
    for line in output.lines() {
        if line.contains("(term: '") && line.contains("')") {
            if let Some(start) = line.find("(term: '") {
                let term_start = start + "(term: '".len();
                if let Some(end) = line[term_start..].find("')") {
                    let term = &line[term_start..term_start + end];
                    if !terms.contains(&term.to_string()) {
                        terms.push(term.to_string());
                    }
                }
            }
        }
    }
    terms
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_clean_output() {
        let raw_output = r#"INFO: Starting process
DEBUG: Loading configuration
Found 3 paragraphs
Match: test term
WARN: Some warning
"#;
        let clean = extract_clean_output(raw_output);
        assert_eq!(clean, "Found 3 paragraphs\nMatch: test term");
    }

    #[test]
    fn test_has_successful_extract_results() {
        let success_output = "Found 5 paragraphs\nMatch: important term";
        let failure_output = "Error: No results found";

        assert!(has_successful_extract_results(success_output));
        assert!(!has_successful_extract_results(failure_output));
    }

    #[test]
    fn test_extract_found_terms() {
        let output = r#"Processing document...
Found term (term: 'rust programming') at position 10
Another match (term: 'async await') found
The same (term: 'rust programming') appears again
"#;
        let terms = extract_found_terms(output);
        assert_eq!(terms.len(), 2);
        assert!(terms.contains(&"rust programming".to_string()));
        assert!(terms.contains(&"async await".to_string()));
    }

    #[test]
    fn test_extract_found_terms_empty() {
        let output = "No terms found in this output";
        let terms = extract_found_terms(output);
        assert!(terms.is_empty());
    }
}
