use log;
use tauri::menu::{CheckMenuItemBuilder, Menu, MenuItemBuilder, PredefinedMenuItem};
use tauri::{AppHandle, Manager, State, WebviewWindow, Wry};
use terraphim_config::{Config, ConfigState};

pub type AppRuntime = Wry;

pub fn primary_window(handle: &AppHandle<AppRuntime>) -> Option<WebviewWindow<AppRuntime>> {
    const LABELS: &[&str] = &["main", ""];
    for label in LABELS {
        if let Some(window) = handle.get_webview_window(label) {
            return Some(window);
        }
    }

    handle.webview_windows().values().next().cloned()
}

fn tray_toggle_label<M: Manager<AppRuntime>>(manager: &M) -> String {
    let handle = manager.app_handle();
    primary_window(handle)
        .and_then(|window| match window.is_visible() {
            Ok(true) => Some("Hide"),
            Ok(false) => Some("Show"),
            Err(_) => None,
        })
        .unwrap_or("Show/Hide")
        .to_string()
}

fn build_tray_menu<M: Manager<AppRuntime>>(
    manager: &M,
    config: &Config,
) -> tauri::Result<Menu<AppRuntime>> {
    let menu = Menu::new(manager)?;
    let toggle_label = tray_toggle_label(manager);
    let toggle_item = MenuItemBuilder::with_id("toggle", toggle_label).build(manager)?;
    menu.append(&toggle_item)?;
    menu.append(&PredefinedMenuItem::separator(manager)?)?;

    for (role_name, _) in &config.roles {
        let item_id = format!("change_role_{role_name}");
        let role_label = role_name.to_string();
        let role_item = CheckMenuItemBuilder::with_id(&item_id, role_label)
            .checked(role_name == &config.selected_role)
            .build(manager)?;
        menu.append(&role_item)?;
    }

    menu.append(&PredefinedMenuItem::separator(manager)?)?;
    let quit_item = MenuItemBuilder::with_id("quit", "Quit").build(manager)?;
    menu.append(&quit_item)?;

    Ok(menu)
}

pub fn apply_tray_menu(handle: &AppHandle<AppRuntime>, config: &Config) {
    if let Some(tray) = handle.tray_by_id("main") {
        match build_tray_menu(handle, config) {
            Ok(menu) => {
                if let Err(err) = tray.set_menu(Some(menu)) {
                    log::error!("Failed to apply tray menu: {}", err);
                }
            }
            Err(err) => {
                log::error!("Failed to build tray menu: {}", err);
            }
        }
    } else {
        log::warn!("Tray icon with id 'main' not found while applying menu");
    }
}

pub async fn refresh_tray_menu_from_state(handle: &AppHandle<AppRuntime>) {
    let config_state: State<'_, ConfigState> = handle.state();
    let config_snapshot = {
        let guard = config_state.config.lock().await;
        guard.clone()
    };
    apply_tray_menu(handle, &config_snapshot);
}

pub fn toggle_main_window(handle: &AppHandle<AppRuntime>) {
    if let Some(window) = primary_window(handle) {
        match window.is_visible() {
            Ok(true) => {
                if let Err(err) = window.hide() {
                    log::error!("Failed to hide main window: {}", err);
                }
            }
            Ok(false) => {
                if let Err(err) = window.show() {
                    log::error!("Failed to show main window: {}", err);
                }
            }
            Err(err) => {
                log::error!("Unable to determine window visibility: {:?}", err);
            }
        }
    } else {
        log::error!("No windows available to toggle");
    }
}
