#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod cmd;
mod config;
mod startup;

use std::error::Error;
use tauri::{
    CustomMenuItem, GlobalShortcutManager, Manager, RunEvent, SystemTray, SystemTrayEvent,
    SystemTrayMenu, WindowBuilder,
};

use terraphim_config::ConfigState;
use terraphim_settings::DeviceSettings;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // TODO: Use the device settings to load the config
    let _device_settings = DeviceSettings::load_from_env_and_file(None);

    let mut config = config::load_config()?;
    let config_state = ConfigState::new(&mut config).await?;
    let current_config = config_state.config.lock().await;
    let global_shortcut = current_config.global_shortcut.clone();
    // drop mutex guard to avoid deadlock
    drop(current_config);

    log::debug!("{:?}", config_state.config);
    let context = tauri::generate_context!();

    let quit = CustomMenuItem::new("quit".to_string(), "Quit");
    let tray_menu = SystemTrayMenu::new()
        .add_item(CustomMenuItem::new("toggle", "Show/Hide"))
        .add_item(quit);
    let system_tray = SystemTray::new().with_menu(tray_menu);

    let app = tauri::Builder::default()
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
        .manage(config_state.clone())
        .invoke_handler(tauri::generate_handler![
            cmd::search,
            cmd::get_config,
            cmd::update_config,
            cmd::publish_thesaurus,
            startup::save_initial_settings,
            start_main_app,
        ])
        .setup(|app| {
            let splashscreen_window = WindowBuilder::new(app, "splashscreen", tauri::WindowUrl::App("splashscreen.html".into()))
                .title("Splashscreen")
                .resizable(false)
                .decorations(false)
                .always_on_top(true)
                .inner_size(400.0, 200.0)
                .build()?;

            // Hide the main window initially
            app.get_window("main").unwrap().hide()?;

            Ok(())
        })
        .build(context)
        .expect("error while running tauri application");

    app.run(move |app_handle, e| match e {
        RunEvent::Ready => {
            let app_handle = app_handle.clone();
            let window = app_handle.get_window("main").unwrap();
            window.hide().unwrap();
            app_handle
                .global_shortcut_manager()
                .register(&global_shortcut, move || {
                    if window.is_visible().unwrap() {
                        window.hide().unwrap();
                    } else {
                        window.show().unwrap();
                    }
                })
                .unwrap();
        }
        RunEvent::ExitRequested { api, .. } => {
            api.prevent_exit();
        }
        _ => {}
    });
    Ok(())
}

#[tauri::command]
async fn start_main_app(app_handle: tauri::AppHandle) {
    // Reload config here if needed
    // let mut config = config::load_config().unwrap();
    // let config_state = ConfigState::new(&mut config).await.unwrap();
    // app_handle.manage(config_state);

    // Close splashscreen
    if let Some(splashscreen) = app_handle.get_window("splashscreen") {
        splashscreen.close().unwrap();
    }

    // Show main window
    if let Some(main_window) = app_handle.get_window("main") {
        main_window.show().unwrap();
    }
}