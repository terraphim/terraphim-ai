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
                        // Try different window labels since the default might not be "main"
                        let window_labels = ["main", ""];  // Empty string is the default label for the first window
                        let mut window_found = false;
                        
                        for label in &window_labels {
                            if let Some(window) = app.get_window(label) {
                                let new_title = match window.is_visible() {
                                    Ok(true) => {
                                        let _ = window.hide();
                                        "Show"
                                    }
                                    Ok(false) => {
                                        let _ = window.show();
                                        "Hide"
                                    }
                                    Err(e) => {
                                        log::error!("Error checking window visibility: {:?}", e);
                                        "Show/Hide"
                                    }
                                };
                                let _ = item_handle.set_title(new_title);
                                window_found = true;
                                break;
                            }
                        }
                        
                        if !window_found {
                            log::error!("No window found with labels: {:?}", window_labels);
                            // Try to get any available window
                            let windows = app.windows();
                            if let Some((_, window)) = windows.iter().next() {
                                let new_title = match window.is_visible() {
                                    Ok(true) => {
                                        let _ = window.hide();
                                        "Show"
                                    }
                                    Ok(false) => {
                                        let _ = window.show();
                                        "Hide"
                                    }
                                    Err(e) => {
                                        log::error!("Error checking window visibility: {:?}", e);
                                        "Show/Hide"
                                    }
                                };
                                let _ = item_handle.set_title(new_title);
                            } else {
                                log::error!("No windows available at all");
                            }
                        }
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
            cmd::get_config_schema,
            cmd::publish_thesaurus,
            cmd::create_document,
            cmd::get_document,
            cmd::save_initial_settings,
            cmd::close_splashscreen,
            cmd::select_role
       ])
        .setup(move |app| {
            let settings = device_settings_read.clone();
            println!("Settings: {:?}", settings);
            let handle = app.handle();
            
            // Try to get the main window with different possible labels
            let main_window = ["main", ""].iter()
                .filter_map(|label| app.get_window(label))
                .next()
                .or_else(|| {
                    // If no window found with expected labels, get the first available window
                    app.windows().values().next().cloned()
                });

            if let Some(main_window) = main_window {
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
                        let _ = main_window.hide();
                    });
                } else {
                    // Show the main window if device_settings.initialized is true
                    let _ = main_window.show();
                }
            } else {
                log::error!("No main window found during setup");
            }
       
            Ok(())
        })
        .build(context)
        .expect("error while running tauri application");

    app.run(move |app_handle, e| match e {
        RunEvent::Ready => {
            let app_handle = app_handle.clone();
            
            // Try to get the window with different possible labels
            let window_labels = ["main", ""];
            let mut window_found = false;
            
            for label in &window_labels {
                if let Some(window) = app_handle.get_window(label) {
                    let _ = window.hide();
                    let window_clone = window.clone();
                    app_handle
                        .global_shortcut_manager()
                        .register(&global_shortcut, move || {
                            match window_clone.is_visible() {
                                Ok(true) => {
                                    let _ = window_clone.hide();
                                }
                                Ok(false) => {
                                    let _ = window_clone.show();
                                }
                                Err(e) => {
                                    log::error!("Error checking window visibility in global shortcut: {:?}", e);
                                }
                            }
                        })
                        .unwrap();
                    window_found = true;
                    break;
                }
            }
            
            if !window_found {
                log::error!("No window found for global shortcut with labels: {:?}", window_labels);
                // Try to get any available window
                let windows = app_handle.windows();
                if let Some((_, window)) = windows.iter().next() {
                    let _ = window.hide();
                    let window_clone = window.clone();
                    app_handle
                        .global_shortcut_manager()
                        .register(&global_shortcut, move || {
                            match window_clone.is_visible() {
                                Ok(true) => {
                                    let _ = window_clone.hide();
                                }
                                Ok(false) => {
                                    let _ = window_clone.show();
                                }
                                Err(e) => {
                                    log::error!("Error checking window visibility in global shortcut: {:?}", e);
                                }
                            }
                        })
                        .unwrap();
                } else {
                    log::error!("No windows available for global shortcut");
                }
            }
        }
        RunEvent::ExitRequested { api, .. } => {
            api.prevent_exit();
        }
        _ => {}
    });
    Ok(())
}