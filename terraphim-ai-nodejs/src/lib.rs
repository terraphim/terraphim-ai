#![deny(clippy::all)]

#[macro_use]
extern crate napi_derive;

use terraphim_automata::{load_thesaurus_from_json_and_replace, LinkType};


#[napi]
pub fn sum(a: i32, b: i32) -> i32 {
  a + b
}

#[napi]
pub async fn replace_links(content: String, thesaurus: String) -> String {
  let replaced = load_thesaurus_from_json_and_replace(&thesaurus, &content, LinkType::MarkdownLinks).await;
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