#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod cmd;
use terraphim_config::{ConfigBuilder, ConfigId};
use terraphim_persistence::Persistable;

use std::error::Error;
use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::{
    CustomMenuItem, GlobalShortcutManager, Manager, RunEvent, SystemTray, SystemTrayEvent,
    SystemTrayMenu, WindowBuilder,
};

use terraphim_config::ConfigState;
use terraphim_settings::DeviceSettings;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let device_settings = match DeviceSettings::load_from_env_and_file(None) {
        Ok(settings) => settings,
        Err(e) => {
            log::error!("Failed to load device settings: {:?}", e);
            panic!("Failed to load device settings: {:?}", e);
        }
    };
    let device_settings_read=device_settings.clone();
    let device_settings = Arc::new(Mutex::new(device_settings));
    
    log::info!("Device settings: {:?}", device_settings.lock().await);

    let mut config = match ConfigBuilder::new_with_id(ConfigId::Desktop).build() {
        Ok(mut config) => match config.load().await {
            Ok(config) => config,
            Err(e) => {
                log::info!("Failed to load config: {:?}", e);
                let config = ConfigBuilder::new().build_default_desktop().build().unwrap();
                config
            },
        },
        Err(e) => panic!("Failed to build config: {:?}", e),
    };
    let config_state = ConfigState::new(&mut config).await?;
    let current_config = config_state.config.lock().await;
    let global_shortcut = current_config.global_shortcut.clone();
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
        .manage(device_settings.clone())
        .invoke_handler(tauri::generate_handler![
            cmd::search,
            cmd::get_config,
            cmd::update_config,
            cmd::publish_thesaurus,
            cmd::create_document,
            cmd::get_document,
            cmd::save_initial_settings,
            cmd::close_splashscreen
       ])
        .setup(move |app| {
            let settings = device_settings_read.clone();
            println!("Settings: {:?}", settings);
            let handle = app.handle();
            let main_window = app.get_window("main").unwrap(); 
            if !settings.initialized {           
            tauri::async_runtime::spawn(async move {
                let splashscreen_window = WindowBuilder::new(&handle, "splashscreen", tauri::WindowUrl::App("../dist/splashscreen.html".into()))
                .title("Splashscreen")
                .resizable(true)
                .decorations(true)
                .always_on_top(false)
                .inner_size(800.0, 600.0).build().unwrap();
                splashscreen_window.show().unwrap();

                // Hide the main window initially
                main_window.hide().unwrap();
            });
            } else {
                // Show the main window if device_settings.initialized is true
                main_window.show().unwrap();
            }
       
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