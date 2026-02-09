use ::terraphim_automata::autocomplete::{
    AutocompleteConfig, AutocompleteIndex, AutocompleteResult, autocomplete_search,
    build_autocomplete_index, deserialize_autocomplete_index, fuzzy_autocomplete_search,
    fuzzy_autocomplete_search_levenshtein, serialize_autocomplete_index,
};
use ::terraphim_automata::matcher::{
    LinkType, Matched, extract_paragraphs_from_automata, find_matches,
};
use ::terraphim_automata::{load_thesaurus_from_json, load_thesaurus_from_json_and_replace};
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;

/// Python wrapper for AutocompleteIndex
#[pyclass(name = "AutocompleteIndex")]
pub struct PyAutocompleteIndex {
    inner: AutocompleteIndex,
}

/// Python wrapper for AutocompleteResult
#[pyclass(name = "AutocompleteResult")]
#[derive(Clone)]
pub struct PyAutocompleteResult {
    #[pyo3(get)]
    pub term: String,
    #[pyo3(get)]
    pub normalized_term: String,
    #[pyo3(get)]
    pub id: u64,
    #[pyo3(get)]
    pub url: Option<String>,
    #[pyo3(get)]
    pub score: f64,
}

impl From<AutocompleteResult> for PyAutocompleteResult {
    fn from(result: AutocompleteResult) -> Self {
        PyAutocompleteResult {
            term: result.term,
            normalized_term: result.normalized_term.to_string(),
            id: result.id,
            url: result.url,
            score: result.score,
        }
    }
}

#[pymethods]
impl PyAutocompleteResult {
    fn __repr__(&self) -> String {
        format!(
            "AutocompleteResult(term='{}', normalized_term='{}', id={}, score={})",
            self.term, self.normalized_term, self.id, self.score
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

/// Python wrapper for Matched
#[pyclass(name = "Matched")]
#[derive(Clone)]
pub struct PyMatched {
    #[pyo3(get)]
    pub term: String,
    #[pyo3(get)]
    pub normalized_term: String,
    #[pyo3(get)]
    pub id: u64,
    #[pyo3(get)]
    pub url: Option<String>,
    #[pyo3(get)]
    pub pos: Option<(usize, usize)>,
}

impl From<Matched> for PyMatched {
    fn from(matched: Matched) -> Self {
        PyMatched {
            term: matched.term,
            normalized_term: matched.normalized_term.value.to_string(),
            id: matched.normalized_term.id,
            url: matched.normalized_term.url,
            pos: matched.pos,
        }
    }
}

#[pymethods]
impl PyMatched {
    fn __repr__(&self) -> String {
        format!(
            "Matched(term='{}', normalized_term='{}', id={}, pos={:?})",
            self.term, self.normalized_term, self.id, self.pos
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

#[pymethods]
impl PyAutocompleteIndex {
    /// Get the name of the autocomplete index
    #[getter]
    fn name(&self) -> String {
        self.inner.name().to_string()
    }

    /// Get the number of terms in the index
    fn __len__(&self) -> usize {
        self.inner.len()
    }

    /// Search for terms matching the prefix
    ///
    /// Args:
    ///     prefix: The search prefix
    ///     max_results: Maximum number of results to return (default: 10)
    ///
    /// Returns:
    ///     List of AutocompleteResult objects
    ///
    /// Note:
    ///     Case sensitivity is determined when the index is built
    #[pyo3(signature = (prefix, max_results=10))]
    fn search(&self, prefix: &str, max_results: usize) -> PyResult<Vec<PyAutocompleteResult>> {
        let results = autocomplete_search(&self.inner, prefix, Some(max_results))
            .map_err(|e| PyValueError::new_err(format!("Search error: {}", e)))?;

        Ok(results
            .into_iter()
            .map(PyAutocompleteResult::from)
            .collect())
    }

    /// Fuzzy search using Jaro-Winkler similarity
    ///
    /// Args:
    ///     query: The search query
    ///     threshold: Similarity threshold (0.0 to 1.0, default: 0.8)
    ///     max_results: Maximum number of results (default: 10)
    ///
    /// Returns:
    ///     List of AutocompleteResult objects sorted by relevance
    #[pyo3(signature = (query, threshold=0.8, max_results=10))]
    fn fuzzy_search(
        &self,
        query: &str,
        threshold: f64,
        max_results: usize,
    ) -> PyResult<Vec<PyAutocompleteResult>> {
        let results = fuzzy_autocomplete_search(&self.inner, query, threshold, Some(max_results))
            .map_err(|e| PyValueError::new_err(format!("Fuzzy search error: {}", e)))?;

        Ok(results
            .into_iter()
            .map(PyAutocompleteResult::from)
            .collect())
    }

    /// Fuzzy search using Levenshtein distance
    ///
    /// Args:
    ///     query: The search query
    ///     max_distance: Maximum edit distance (default: 2)
    ///     max_results: Maximum number of results (default: 10)
    ///
    /// Returns:
    ///     List of AutocompleteResult objects sorted by relevance
    #[pyo3(signature = (query, max_distance=2, max_results=10))]
    fn fuzzy_search_levenshtein(
        &self,
        query: &str,
        max_distance: usize,
        max_results: usize,
    ) -> PyResult<Vec<PyAutocompleteResult>> {
        let results = fuzzy_autocomplete_search_levenshtein(
            &self.inner,
            query,
            max_distance,
            Some(max_results),
        )
        .map_err(|e| PyValueError::new_err(format!("Fuzzy search error: {}", e)))?;

        Ok(results
            .into_iter()
            .map(PyAutocompleteResult::from)
            .collect())
    }

    /// Serialize the index to bytes for caching
    ///
    /// Returns:
    ///     Bytes representation of the index
    ///
    /// Example:
    ///     >>> index = build_index(thesaurus_json)
    ///     >>> data = index.serialize()
    ///     >>> # Save to file
    ///     >>> with open("index.bin", "wb") as f:
    ///     ...     f.write(data)
    fn serialize(&self) -> PyResult<Vec<u8>> {
        serialize_autocomplete_index(&self.inner)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to serialize index: {}", e)))
    }

    /// Deserialize an index from bytes
    ///
    /// Args:
    ///     data: Bytes representation of the index
    ///
    /// Returns:
    ///     AutocompleteIndex object
    ///
    /// Example:
    ///     >>> # Load from file
    ///     >>> with open("index.bin", "rb") as f:
    ///     ...     data = f.read()
    ///     >>> index = AutocompleteIndex.deserialize(data)
    #[staticmethod]
    fn deserialize(data: &[u8]) -> PyResult<PyAutocompleteIndex> {
        let index = deserialize_autocomplete_index(data)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to deserialize index: {}", e)))?;
        Ok(PyAutocompleteIndex { inner: index })
    }

    fn __repr__(&self) -> String {
        format!(
            "AutocompleteIndex(name='{}', len={})",
            self.inner.name(),
            self.inner.len()
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

/// Load thesaurus from JSON string
///
/// Args:
///     json_str: JSON string containing thesaurus data
///
/// Returns:
///     Tuple of (thesaurus_name, number_of_terms)
///
/// Example:
///     >>> json_str = '{"name": "test", "data": {"term1": {"id": 1, "nterm": "normalized", "url": "https://example.com"}}}'
///     >>> name, count = load_thesaurus(json_str)
#[pyfunction]
fn load_thesaurus(json_str: &str) -> PyResult<(String, usize)> {
    let thesaurus = load_thesaurus_from_json(json_str)
        .map_err(|e| PyValueError::new_err(format!("Failed to load thesaurus: {}", e)))?;

    let name = thesaurus.name().to_string();
    let count = thesaurus.len();

    Ok((name, count))
}

/// Build autocomplete index from thesaurus JSON
///
/// Args:
///     json_str: JSON string containing thesaurus data
///     case_sensitive: Whether the index should be case-sensitive (default: False)
///
/// Returns:
///     AutocompleteIndex object
///
/// Example:
///     >>> json_str = '{"name": "test", "data": {"term1": {"id": 1, "nterm": "normalized", "url": "https://example.com"}}}'
///     >>> index = build_index(json_str)
///     >>> results = index.search("ter")
#[pyfunction]
#[pyo3(signature = (json_str, case_sensitive=false))]
fn build_index(json_str: &str, case_sensitive: bool) -> PyResult<PyAutocompleteIndex> {
    let thesaurus = load_thesaurus_from_json(json_str)
        .map_err(|e| PyValueError::new_err(format!("Failed to load thesaurus: {}", e)))?;

    let config = AutocompleteConfig {
        max_results: 10,
        min_prefix_length: 1,
        case_sensitive,
    };

    let index = build_autocomplete_index(thesaurus, Some(config))
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to build index: {}", e)))?;

    Ok(PyAutocompleteIndex { inner: index })
}

/// Find all matches of thesaurus terms in text
///
/// Args:
///     text: The text to search in
///     json_str: JSON string containing thesaurus data
///     return_positions: Whether to return match positions (default: True)
///
/// Returns:
///     List of Matched objects
///
/// Example:
///     >>> text = "This is a test document with some terms"
///     >>> json_str = '{"name": "test", "data": {"test": {"id": 1, "nterm": "test", "url": "https://example.com"}}}'
///     >>> matches = find_all_matches(text, json_str)
#[pyfunction]
#[pyo3(signature = (text, json_str, return_positions=true))]
fn find_all_matches(
    text: &str,
    json_str: &str,
    return_positions: bool,
) -> PyResult<Vec<PyMatched>> {
    let thesaurus = load_thesaurus_from_json(json_str)
        .map_err(|e| PyValueError::new_err(format!("Failed to load thesaurus: {}", e)))?;

    let matches = find_matches(text, thesaurus, return_positions)
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to find matches: {}", e)))?;

    Ok(matches.into_iter().map(PyMatched::from).collect())
}

/// Replace all thesaurus term matches with links
///
/// Args:
///     text: The text to process
///     json_str: JSON string containing thesaurus data
///     link_type: Type of links to create ('wiki', 'html', 'markdown', 'plain')
///
/// Returns:
///     String with replaced links
///
/// Example:
///     >>> text = "This is a test"
///     >>> json_str = '{"name": "test", "data": {"test": {"id": 1, "nterm": "test", "url": "https://example.com"}}}'
///     >>> result = replace_with_links(text, json_str, "markdown")
#[pyfunction]
fn replace_with_links(text: &str, json_str: &str, link_type: &str) -> PyResult<String> {
    let link_type_enum = match link_type.to_lowercase().as_str() {
        "wiki" => LinkType::WikiLinks,
        "html" => LinkType::HTMLLinks,
        "markdown" => LinkType::MarkdownLinks,
        "plain" => LinkType::PlainText,
        _ => {
            return Err(PyValueError::new_err(format!(
                "Invalid link type '{}'. Use 'wiki', 'html', 'markdown', or 'plain'",
                link_type
            )));
        }
    };

    // Use the Rust convenience function that loads and replaces in one step
    let result = load_thesaurus_from_json_and_replace(json_str, text, link_type_enum)
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to replace matches: {}", e)))?;

    String::from_utf8(result).map_err(|e| PyRuntimeError::new_err(format!("Invalid UTF-8: {}", e)))
}

/// Extract paragraphs starting at matched terms
///
/// Args:
///     text: The text to process
///     json_str: JSON string containing thesaurus data
///     include_term: Whether to include the matched term in the paragraph (default: True)
///
/// Returns:
///     List of tuples (term, paragraph_text)
///
/// Example:
///     >>> text = "Paragraph one.\\n\\nParagraph two with term.\\n\\nParagraph three."
///     >>> json_str = '{"name": "test", "data": {"term": {"id": 1, "nterm": "term", "url": ""}}}'
///     >>> paragraphs = extract_paragraphs(text, json_str)
#[pyfunction]
#[pyo3(signature = (text, json_str, include_term=true))]
fn extract_paragraphs(
    text: &str,
    json_str: &str,
    include_term: bool,
) -> PyResult<Vec<(String, String)>> {
    let thesaurus = load_thesaurus_from_json(json_str)
        .map_err(|e| PyValueError::new_err(format!("Failed to load thesaurus: {}", e)))?;

    let paragraphs = extract_paragraphs_from_automata(text, thesaurus, include_term)
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to extract paragraphs: {}", e)))?;

    // Convert Vec<(Matched, String)> to Vec<(String, String)>
    let result = paragraphs
        .into_iter()
        .map(|(matched, paragraph)| (matched.term, paragraph))
        .collect();

    Ok(result)
}

/// Python module for Terraphim Automata
///
/// This module provides fast autocomplete and text processing capabilities
/// for knowledge graphs using finite state transducers (FST) and Aho-Corasick automata.
#[pymodule]
fn terraphim_automata(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyAutocompleteIndex>()?;
    m.add_class::<PyAutocompleteResult>()?;
    m.add_class::<PyMatched>()?;
    m.add_function(wrap_pyfunction!(load_thesaurus, m)?)?;
    m.add_function(wrap_pyfunction!(build_index, m)?)?;
    m.add_function(wrap_pyfunction!(find_all_matches, m)?)?;
    m.add_function(wrap_pyfunction!(replace_with_links, m)?)?;
    m.add_function(wrap_pyfunction!(extract_paragraphs, m)?)?;

    // Add module version
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;

    Ok(())
}
