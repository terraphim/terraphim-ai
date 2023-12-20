#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

mod cmd;
#[cfg(target_os = "linux")]
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};
use tauri::{
  api::dialog::ask, http::ResponseBuilder, CustomMenuItem, GlobalShortcutManager, Manager,
  RunEvent, SystemTray, SystemTrayEvent, SystemTrayMenu, SystemTrayMenuItem,WindowBuilder, WindowUrl,
  
};

use std::{error::Error, time::Duration};

use std::collections::HashMap;
extern crate config;
extern crate serde;

#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate lazy_static;

mod settings;
use crate::settings::CONFIG;




fn main() {
  println!("{:?}", CONFIG.global_shortcut);
  let context = tauri::generate_context!();

  let quit = CustomMenuItem::new("quit".to_string(), "Quit");
  let tray_menu = SystemTrayMenu::new()
    .add_item(CustomMenuItem::new("toggle", "Show/Hide"))
    .add_item(quit);
    // .add_item(show);
    let system_tray = SystemTray::new()
    .with_menu(tray_menu);

    let mut app = tauri::Builder::default()
      .system_tray(system_tray)
      .on_system_tray_event(|app, event| match event {
        SystemTrayEvent::MenuItemClick { id, .. } => {
          let item_handle = app.tray_handle().get_item(&id);
          match id.as_str() {
            "quit" => {
              std::process::exit(0);
            }
            "toggle" => {
              let window = app.get_window("main").unwrap();
              let new_title = if window.is_visible().unwrap() {
                window.hide().unwrap();
                "Show"
              } else {
                window.show().unwrap();
                "Hide"
              };
              item_handle.set_title(new_title).unwrap();
            }
            _ => {}
          }
        }
        _ => {}
      })
      .invoke_handler(tauri::generate_handler![
        cmd::log_operation,
        cmd::perform_request,
        cmd::search,
        cmd::get_config
      ])
      .build(context)
      .expect("error while running tauri application");


  app.run(move|app_handle, e| match e {
    // Application is ready (triggered only once)
    RunEvent::Ready => {
      let app_handle = app_handle.clone();
      let window = app_handle.get_window("main").unwrap();
      window.hide().unwrap();
      app_handle
        .global_shortcut_manager()
        .register(&CONFIG.global_shortcut, move || {

          if window.is_visible().unwrap() {
            window.hide().unwrap();
          } else {
            window.show().unwrap();
          }
        })
        .unwrap();
    }

  //   // Triggered when a window is trying to close
  //   tauri::RunEvent::WindowEvent {
  //     label,
  //     event: win_event,
  //     ..
  // } => match win_event {
  //     tauri::WindowEvent::CloseRequested { api, .. } => {
  //         let win = app.get_window(label.as_str()).unwrap();
  //         win.hide().unwrap();
  //         api.prevent_close();
  //     }
  //     _ => {}
  // },

    // Keep the event loop running even if all windows are closed
    // This allow us to catch system tray events when there is no window
    RunEvent::ExitRequested { api, .. } => {
      api.prevent_exit();
    }
    _ => {}
  });
  }

