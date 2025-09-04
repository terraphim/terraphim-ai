mod utils;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

use serde::{Serialize, Deserialize};

use web_sys::console;

// will be called when the wasm module is loaded
// https://rustwasm.github.io/docs/wasm-bindgen/reference/attributes/on-rust-exports/start.html
#[wasm_bindgen(start)]
pub fn main() {
    console::log_1(&"[from wasm] Inited.".into());
}

#[wasm_bindgen]
pub fn print() {
    console::log_1(&"[from wasm] Hello World!".into());
}

#[wasm_bindgen]
pub fn print_with_value(value: &str) {
    // with 2-args log function
    console::log_2(&"[from wasm] Hello".into(), &value.into());
}


#[derive(Serialize, Deserialize)]
pub struct ReplacerConfig {
    patterns: Vec<String>,
    replace_with: Vec<String>,
    rdr: String,
}



use gloo_utils::format::JsValueSerdeExt;


use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response};

/// A struct to hold some data from the github Branch API.
///
/// Note how we don't have to define every member -- serde will ignore extra
/// data when deserializing
#[derive(Debug, Serialize, Deserialize)]
pub struct Dictionary {
    pub term: String,
    pub id: String,
    pub parent: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KG{
  pub nodes: Vec<Dictionary>,
}


// #[wasm_bindgen]
// pub async fn load_automata_url(from_url: String) -> Result<JsValue, JsValue> {
//     let mut opts = RequestInit::new();
//     opts.method("GET");
//     opts.mode(RequestMode::Cors);

//     let request = Request::new_with_str_and_init(&from_url, &opts)?;

//     // request
//     //     .headers()
//     //     .set("Accept", "application/vnd.github.v3+json")?;

//     let window = web_sys::window().unwrap();
//     let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;

//     // `resp_value` is a `Response` object.
//     assert!(resp_value.is_instance_of::<Response>());
//     let resp: Response = resp_value.dyn_into().unwrap();



//     // // Convert this other `Promise` into a rust `Future`.
//     let json = JsFuture::from(resp.json()?).await?;

//     // Use serde to parse the JSON into a struct.
//     let knowledge_graph: KG = json.into_serde().unwrap();

//     // Send the `Branch` struct back to JS as an `Object`.
//     Ok(JsValue::from_serde(&json).unwrap())
// }

use aho_corasick::{AhoCorasick, MatchKind};

// #[wasm_bindgen]
// pub fn find_matched(patterns: Vec<&str>, haystack: &str) -> Vec<&str> {
//   let mut matches = Vec::new();
//   let ac = AhoCorasick::builder()
//     .match_kind(MatchKind::LeftmostLongest)
//     .build(patterns.clone())
//     .unwrap();
//   let text = &haystack.as_str();
//   for mat in ac.find_iter(text) {
//     let term = patterns[mat.pattern()].to_string();
//     matches.push(term);
//   }
//   matches
// }


#[wasm_bindgen]
pub fn replace_all_stream(val: JsValue) -> String {
  let replacer_config: ReplacerConfig = val.into_serde().unwrap();
  let mut wtr = vec![];
  let replace_with = &replacer_config.replace_with;
  let ac = AhoCorasick::new(&replacer_config.patterns).unwrap();
  let rdr = &replacer_config.rdr;
  ac.try_stream_replace_all(rdr.as_bytes(), &mut wtr, replace_with)
    .unwrap();
  String::from_utf8(wtr)
    .map_err(|non_utf8| String::from_utf8_lossy(non_utf8.as_bytes()).into_owned())
    .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    fn test_replace_all_stream() {
        let replacer_config = ReplacerConfig {
            patterns: vec!["foo".to_string(), "bar".to_string()],
            replace_with: vec!["baz".to_string()],
            rdr: "foo bar foo bar".to_string(),
        };
        let expected_output = "baz baz baz baz".to_string();
        let actual_output = replace_all_stream(JsValue::from_serde(&replacer_config).unwrap());
        assert_eq!(actual_output, expected_output);
    }
}