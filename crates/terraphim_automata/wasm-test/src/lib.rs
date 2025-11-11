use wasm_bindgen::prelude::*;
use terraphim_automata::{
    autocomplete::{autocomplete_search, build_autocomplete_index},
    load_thesaurus_from_json,
};

// Initialize panic hook for better error messages in the browser
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
    wasm_logger::init(wasm_logger::Config::default());
    log::info!("Terraphim Automata WASM module initialized");
}

/// Build an autocomplete index from a JSON thesaurus string
///
/// # Arguments
/// * `json_str` - JSON string containing thesaurus data
///
/// # Returns
/// A serialized autocomplete index or error message
#[wasm_bindgen]
pub fn build_index_from_json(json_str: &str) -> Result<Vec<u8>, JsValue> {
    let thesaurus = load_thesaurus_from_json(json_str)
        .map_err(|e| JsValue::from_str(&format!("Failed to load thesaurus: {}", e)))?;

    let index = build_autocomplete_index(thesaurus, None)
        .map_err(|e| JsValue::from_str(&format!("Failed to build index: {}", e)))?;

    let serialized = terraphim_automata::serialize_autocomplete_index(&index)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize index: {}", e)))?;

    Ok(serialized)
}

/// Search the autocomplete index with a query
///
/// # Arguments
/// * `index_bytes` - Serialized autocomplete index
/// * `query` - Search query string
/// * `max_results` - Maximum number of results to return
///
/// # Returns
/// JSON array of autocomplete results
#[wasm_bindgen]
pub fn autocomplete(index_bytes: &[u8], query: &str, max_results: usize) -> Result<String, JsValue> {
    let index = terraphim_automata::deserialize_autocomplete_index(index_bytes)
        .map_err(|e| JsValue::from_str(&format!("Failed to deserialize index: {}", e)))?;

    let results = autocomplete_search(&index, query, Some(max_results))
        .map_err(|e| JsValue::from_str(&format!("Failed to search: {}", e)))?;

    // Convert results to a simple JSON format
    let json_results: Vec<_> = results
        .iter()
        .map(|r| {
            serde_json::json!({
                "term": r.term,
                "normalized_term": r.normalized_term.to_string(),
                "id": r.id,
                "url": r.url,
                "score": r.score
            })
        })
        .collect();

    serde_json::to_string(&json_results)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize results: {}", e)))
}

/// Get version information
#[wasm_bindgen]
pub fn version() -> String {
    format!("terraphim_automata v{}", env!("CARGO_PKG_VERSION"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_init() {
        init();
        log::info!("Test initialization successful");
    }

    #[wasm_bindgen_test]
    fn test_version() {
        let ver = version();
        assert!(ver.contains("terraphim_automata"));
    }

    #[wasm_bindgen_test]
    fn test_build_and_search() {
        let json_str = r#"
        {
            "name": "Test",
            "data": {
                "foo": {
                    "id": 1,
                    "nterm": "foo",
                    "url": "https://example.com/foo"
                },
                "foobar": {
                    "id": 2,
                    "nterm": "foobar",
                    "url": "https://example.com/foobar"
                }
            }
        }"#;

        let index = build_index_from_json(json_str).expect("Failed to build index");
        let results = autocomplete(&index, "foo", 10).expect("Failed to search");

        assert!(results.contains("foo"));
        log::info!("Test build and search successful");
    }
}
