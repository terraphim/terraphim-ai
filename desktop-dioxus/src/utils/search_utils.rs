use terraphim_types::{SearchQuery, RoleName};

pub fn parse_search_input(input: &str) -> ParsedSearch {
    ParsedSearch {
        terms: vec![input.to_string()],
        operator: None,
    }
}

pub struct ParsedSearch {
    pub terms: Vec<String>,
    pub operator: Option<String>,
}

pub fn build_search_query(parsed: &ParsedSearch, role: &RoleName) -> SearchQuery {
    SearchQuery {
        input: parsed.terms.join(" "),
        role_name: role.original.clone(),
        skip: 0,
        limit: 10,
        threshold: None,
        operator: None,
    }
}
