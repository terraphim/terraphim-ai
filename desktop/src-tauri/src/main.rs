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
    SystemTrayMenu, SystemTrayMenuItem, SystemTraySubmenu, WindowBuilder,
};

use terraphim_config::ConfigState;
use terraphim_settings::DeviceSettings;

fn build_tray_menu(config: &terraphim_config::Config) -> SystemTrayMenu {
    let mut role_menu = SystemTrayMenu::new();
    let roles = &config.roles;
    let selected_role = &config.selected_role;

    for (role_name, _role) in roles {
        // Use a unique id for each role menu item
        let item_id = format!("change_role_{}", role_name);
        let mut menu_item = CustomMenuItem::new(item_id, role_name.to_string());
        if role_name == selected_role {
            menu_item.selected = true;
        }
        role_menu = role_menu.add_item(menu_item);
    }

    let quit = CustomMenuItem::new("quit".to_string(), "Quit");
    let change_role_submenu = SystemTraySubmenu::new("Change Role", role_menu);

    SystemTrayMenu::new()
        .add_item(CustomMenuItem::new("toggle", "Show/Hide"))
        .add_submenu(change_role_submenu)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(quit)
}

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
    let tray_menu = build_tray_menu(&current_config);
    drop(current_config);

    log::debug!("{:?}", config_state.config);
    let context = tauri::generate_context!();

    let system_tray = SystemTray::new().with_menu(tray_menu);

    let app = tauri::Builder::default()
        .system_tray(system_tray)
        .on_system_tray_event(|app, event| {
            if let SystemTrayEvent::MenuItemClick { id, .. } = event {
                let app_handle = app.clone();
                let id_clone = id.clone();

                if id.starts_with("change_role_") {
                    tauri::async_runtime::spawn(async move {
                        let role_name = id_clone.strip_prefix("change_role_").unwrap();
                        log::info!("User requested to change role to {}", role_name);
                        let config_state: tauri::State<ConfigState> = app_handle.state();

                        match cmd::select_role(config_state, role_name.to_string()).await {
                            Ok(new_config_response) => {
                                let new_tray_menu = build_tray_menu(&new_config_response.config);
                                if let Err(e) = app_handle.tray_handle().set_menu(new_tray_menu) {
                                    log::error!("Failed to set new tray menu: {}", e);
                                }
                            }
                            Err(e) => {
                                log::error!("Failed to select role: {}", e);
                            }
                        }
                    });
                } else {
                    let item_handle = app.tray_handle().get_item(&id_clone);
                    match id_clone.as_str() {
                        "quit" => {
                            std::process::exit(0);
                        }
                        "toggle" => {
                            let window_labels = ["main", ""];
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
                                if let Some((_, window)) = app.windows().iter().next() {
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
            }
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