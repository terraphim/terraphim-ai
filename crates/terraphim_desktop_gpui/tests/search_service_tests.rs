use terraphim_desktop_gpui::search_service::{LogicalOperator, SearchService};

#[test]
fn test_parse_simple_query() {
    let query = SearchService::parse_query("rust");
    assert_eq!(query.terms.len(), 1);
    assert_eq!(query.terms[0], "rust");
    assert!(query.operator.is_none());
}

#[test]
fn test_parse_and_query() {
    let query = SearchService::parse_query("rust AND tokio");
    assert_eq!(query.terms.len(), 2);
    assert_eq!(query.terms[0], "rust");
    assert_eq!(query.terms[1], "tokio");
    assert_eq!(query.operator, Some(LogicalOperator::And));
}

#[test]
fn test_parse_or_query() {
    let query = SearchService::parse_query("rust OR async");
    assert_eq!(query.terms.len(), 2);
    assert_eq!(query.operator, Some(LogicalOperator::Or));
}

#[test]
fn test_parse_case_insensitive() {
    let query = SearchService::parse_query("Rust and Tokio");
    assert_eq!(query.terms.len(), 2);
    assert_eq!(query.operator, Some(LogicalOperator::And));
}

#[test]
fn test_is_complex_query() {
    let simple = SearchService::parse_query("rust");
    assert!(!simple.is_complex());

    let complex = SearchService::parse_query("rust AND tokio");
    assert!(complex.is_complex());
}
