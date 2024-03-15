#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod cmd;

use std::error::Error;
use tauri::{
    CustomMenuItem, GlobalShortcutManager, Manager, RunEvent, SystemTray, SystemTrayEvent,
    SystemTrayMenu,
};

use terraphim_config::{Config, ConfigState};
use terraphim_settings::Settings;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // TODO: Use the device settings to load the config
    let _device_settings = Settings::load_from_env_and_file(None);

    let mut config = Config::new();
    let config_state = ConfigState::new(&mut config).await?;
    let current_config = config_state.config.lock().await;
    let globbal_shortcut = current_config.global_shortcut.clone();
    // drop mutex guard to avoid deadlock
    drop(current_config);

    log::debug!("{:?}", config_state.config);
    let context = tauri::generate_context!();

    let quit = CustomMenuItem::new("quit".to_string(), "Quit");
    let tray_menu = SystemTrayMenu::new()
        .add_item(CustomMenuItem::new("toggle", "Show/Hide"))
        .add_item(quit);
    // .add_item(show);
    let system_tray = SystemTray::new().with_menu(tray_menu);

    // tauri::async_runtime::spawn(async move {
    //   let mut config_state= terraphim_server::types::ConfigState::new().await.unwrap();
    //   axum_server(addr,config_state).await;
    // });
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
            cmd::my_custom_command,
            cmd::search,
            cmd::get_config,
            cmd::log_operation,
            cmd::perform_request,
        ])
        .build(context)
        .expect("error while running tauri application");

    app.run(move |app_handle, e| match e {
        // Application is ready (triggered only once)
        RunEvent::Ready => {
            let app_handle = app_handle.clone();
            let window = app_handle.get_window("main").unwrap();
            window.hide().unwrap();
            app_handle
                .global_shortcut_manager()
                .register(&globbal_shortcut, move || {
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
    Ok(())
}
