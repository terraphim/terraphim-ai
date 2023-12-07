// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use serde::{Serialize,Deserialize};
use tauri::command;
use crate::settings::CONFIG;
use std::error::Error;
use terraphim_grep::scan_path;
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct RequestBody {
  id: i32,
  name: String,
}

#[command]
pub fn log_operation(event: String, payload: Option<String>) {
  println!("{} {:?}", event, payload);
}

#[command]
pub fn perform_request(endpoint: String, body: RequestBody) -> String {
  println!("{} {:?}", endpoint, body);
  "message response".into()
}

#[command]
pub fn search(needle: String, skip:Option<u8>,limit:Option<u8>,role: Option<String>) -> String {
  let role=role.as_deref().unwrap_or("Engineer");
  println!("{} {}", needle, role);
  for each_plugin in &CONFIG.role.get(role).unwrap().plugins {
    println!("{:?}", each_plugin);
    match each_plugin.name.as_str() {
        "terraphim-grep" => {
          println!("Terraphim Grep called with {needle} {:?}", each_plugin.hackstack);
        scan_path(&needle, each_plugin.hackstack.clone(),None);
        }
        "rustb" => {
            println!("{:?}", each_plugin.hackstack);
        }
        _ => {
            println!("{:?}", each_plugin.hackstack);
        }
    }
}
  println!("{:?}", CONFIG.role.len());
  "search response".into()
}

#[command]
pub fn get_config() -> String {
  println!("{:?}", CONFIG.role);
  serde_json::to_string(&CONFIG.role).unwrap()
}