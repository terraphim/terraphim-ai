#![deny(clippy::all)]

#[macro_use]
extern crate napi_derive;

use terraphim_automata::{load_thesaurus_from_json_and_replace, LinkType};
use terraphim_persistence::Persistable;
use terraphim_config::ConfigState;
use terraphim_config::{ConfigBuilder,ConfigId};
use terraphim_service::TerraphimService;
use terraphim_settings::DeviceSettings;
use anyhow::Context;

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

#[napi]
pub async fn get_config() -> String {
  let device_settings =
  DeviceSettings::load_from_env_and_file(None).context("Failed to load settings").unwrap();
  println!("Device settings: {:?}", device_settings);

  // TODO: refactor 
    let mut config = match ConfigBuilder::new_with_id(ConfigId::Desktop).build() {
      Ok(mut config) => match config.load().await {
          Ok(config) => config,
          Err(e) => {
              println!("Failed to load config: {:?}", e);
              let config = ConfigBuilder::new().build_default_desktop().build().unwrap();
              config
          },
      },
      Err(e) => panic!("Failed to build config: {:?}", e),
  };
  let config_state = ConfigState::new(&mut config).await.unwrap();
  let terraphim_service = TerraphimService::new(config_state);
  let config = terraphim_service.fetch_config().await;
  serde_json::to_string(&config).unwrap()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn async_sum_test() {
    let result = sum(1, 2);
    assert_eq!(result, 3);
  }
  #[tokio::test]
  async fn async_get_config_test() {
    let config = get_config().await;
    println!("Config: {}", config);
    assert_eq!(config, "{}");
  }
} 