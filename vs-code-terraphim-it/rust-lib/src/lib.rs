use wasm_bindgen::prelude::*;
use terraphim_automata::{load_thesaurus_from_json_and_replace, LinkType};

#[wasm_bindgen]
pub async fn replace_links(content: &str, thesaurus: &str) -> String {
    let replaced = load_thesaurus_from_json_and_replace(thesaurus, content, LinkType::MarkdownLinks).await;
    let result = match replaced {
        Ok(replaced) => replaced,
        Err(e) => {
            println!("Error replacing links: {}", e);
            Vec::new()
        }
    };
    String::from_utf8(result)
    .map_err(|non_utf8| String::from_utf8_lossy(non_utf8.as_bytes()).into_owned())
    .unwrap()
}
