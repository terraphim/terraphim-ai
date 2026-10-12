//! Existing Atomic-resource link replacement, shared with Terraphim automata.
use terraphim_automata::{load_thesaurus_from_json_and_replace, LinkType};
use wasm_bindgen::prelude::*;

fn replace_text(content: &str, thesaurus: &str) -> Result<String, String> {
    let bytes = load_thesaurus_from_json_and_replace(thesaurus, content, LinkType::MarkdownLinks)
        .map_err(|error| error.to_string())?;
    String::from_utf8(bytes).map_err(|error| error.to_string())
}

/// Replace known terms with Markdown links using the supplied Terraphim thesaurus.
///
/// # Errors
/// Returns a JavaScript error for an invalid thesaurus or invalid UTF-8 output.
/// Errors never become empty replacement text.
#[wasm_bindgen]
pub fn replace_links(content: &str, thesaurus: &str) -> Result<String, JsValue> {
    replace_text(content, thesaurus).map_err(|error| JsValue::from_str(&error))
}

#[cfg(test)]
mod tests {
    use super::replace_text;

    #[test]
    fn links_synonyms_with_the_existing_automata_crate() {
        let thesaurus = r#"{"name":"Engineering","data":{
            "risk":{"id":1,"nterm":"risk","url":"https://example.com/risk"},
            "concern":{"id":1,"nterm":"risk","url":"https://example.com/risk"}
        }}"#;
        assert_eq!(
            replace_text("A concern and risk. 😀", thesaurus).unwrap(),
            "A [risk](https://example.com/risk) and [risk](https://example.com/risk). 😀"
        );
    }

    #[test]
    fn invalid_thesaurus_is_an_error_and_unknown_text_is_preserved() {
        assert!(replace_text("Do not erase this text", "{invalid}").is_err());
        assert_eq!(
            replace_text("No matched term. 😀", r#"{"name":"Empty","data":{}}"#).unwrap(),
            "No matched term. 😀"
        );
    }
}
