#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod cmd;
use terraphim_config::{ConfigBuilder, ConfigId};
use terraphim_persistence::Persistable;

use std::error::Error;
use std::path::Path;
use std::sync::Arc;
use tauri::{
    CustomMenuItem, GlobalShortcutManager, Manager, RunEvent, SystemTray, SystemTrayEvent,
    SystemTrayMenu, SystemTrayMenuItem, WindowBuilder,
};
use tokio::sync::Mutex;

use rmcp::service::ServiceExt;
use terraphim_config::ConfigState;
use terraphim_mcp_server::McpService;
use terraphim_settings::DeviceSettings;
use tokio::io::{stdin, stdout};
use tracing_subscriber::prelude::*;

/// Initialize user data folder with bundled docs/src content if empty
async fn initialize_user_data_folder(
    app_handle: &tauri::AppHandle,
    device_settings: &DeviceSettings,
) -> Result<(), Box<dyn Error>> {
    let data_path = Path::new(&device_settings.default_data_path);

    // Check if data folder exists and has content
    let should_initialize = if !data_path.exists() {
        log::info!("User data folder does not exist, creating: {:?}", data_path);
        std::fs::create_dir_all(data_path)?;
        true
    } else {
        // Check if folder is empty or missing key directories
        let kg_path = data_path.join("kg");
        let has_kg = kg_path.exists() && kg_path.read_dir()?.next().is_some();
        let has_docs = data_path.read_dir()?.any(|entry| {
            if let Ok(entry) = entry {
                let path = entry.path();
                path.is_file() && path.extension().map_or(false, |ext| ext == "md")
            } else {
                false
            }
        });

        if !has_kg || !has_docs {
            log::info!(
                "User data folder missing content, will initialize: kg={}, docs={}",
                has_kg,
                has_docs
            );
            true
        } else {
            log::info!("User data folder already initialized");
            false
        }
    };

    if should_initialize {
        // Get the bundled docs/src content
        let resource_dir = app_handle
            .path_resolver()
            .resource_dir()
            .ok_or("Failed to get resource directory")?;
        let bundled_docs_src = resource_dir.join("docs").join("src");

        if bundled_docs_src.exists() {
            log::info!(
                "Copying bundled content from {:?} to {:?}",
                bundled_docs_src,
                data_path
            );
            copy_dir_all(&bundled_docs_src, data_path)?;
            log::info!("Successfully initialized user data folder");
        } else {
            log::warn!("Bundled docs/src not found at {:?}", bundled_docs_src);
        }
    }

    Ok(())
}

/// Recursively copy a directory and all its contents
fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    if !dst.exists() {
        std::fs::create_dir_all(dst)?;
    }

    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}

fn build_tray_menu(config: &terraphim_config::Config) -> SystemTrayMenu {
    let roles = &config.roles;
    let selected_role = &config.selected_role;

    let mut menu = SystemTrayMenu::new()
        .add_item(CustomMenuItem::new("toggle", "Show/Hide"))
        .add_native_item(SystemTrayMenuItem::Separator);

    for (role_name, _role) in roles {
        // Use a unique id for each role menu item
        let item_id = format!("change_role_{}", role_name);
        let mut menu_item = CustomMenuItem::new(item_id, role_name.to_string());
        if role_name == selected_role {
            menu_item.selected = true;
        }
        menu = menu.add_item(menu_item);
    }

    let quit = CustomMenuItem::new("quit".to_string(), "Quit");

    menu.add_native_item(SystemTrayMenuItem::Separator)
        .add_item(quit)
}

/// Runs the Terraphim MCP server over stdio (blocking)
async fn run_mcp_server() -> anyhow::Result<()> {
    // Initialise logging identical to the standalone terraphim_mcp_server binary so that
    // feature-parity is preserved when using the desktop binary in server-only mode.
    let log_dir = std::env::var("TERRAPHIM_LOG_DIR").unwrap_or_else(|_| {
        // Use /tmp for logs when running as MCP server to avoid permission issues
        "/tmp/terraphim-logs".to_string()
    });
    std::fs::create_dir_all(&log_dir)?;
    let file_appender = tracing_appender::rolling::daily(&log_dir, "terraphim-mcp-server.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    let _ = tracing_log::LogTracer::init();

    let subscriber = tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_writer(non_blocking))
        .with(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::WARN.into()),
        );
    let _ = subscriber.try_init();

    tracing::info!("Starting Terraphim MCP server (embedded in desktop binary)");

    // Use desktop configuration for consistent experience between desktop and MCP server modes
    let config = terraphim_config::ConfigBuilder::new()
        .build_default_desktop()
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to build default desktop configuration: {:?}", e))?;

    let mut tmp_config = config.clone();
    let config_state = terraphim_config::ConfigState::new(&mut tmp_config).await?;
    let service = McpService::new(std::sync::Arc::new(config_state));

    // Start serving over stdio.
    let server = service.serve((stdin(), stdout())).await?;
    tracing::info!("MCP server initialised – awaiting shutdown");
    let reason = server.waiting().await?;
    tracing::info!("MCP server shut down with reason: {:?}", reason);
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Early CLI handling: if invoked with the `mcp-server` sub-command run the MCP server
    // and exit. This allows the same desktop binary to be used as a pure head-less
    // Terraphim MCP server that speaks over stdio so that external MCP clients can
    // communicate with it via the Model Context Protocol.
    if std::env::args().any(|arg| arg == "mcp-server" || arg == "--mcp-server") {
        return run_mcp_server().await.map_err(|e| e.into());
    }

    // Initialize logging only for desktop app mode (not MCP server mode)
    terraphim_service::logging::init_logging(terraphim_service::logging::detect_logging_config());

    log::info!("Starting Terraphim Desktop app...");
    log::info!("Current working directory: {:?}", std::env::current_dir()?);

    let device_settings = match DeviceSettings::load_from_env_and_file(None) {
        Ok(settings) => {
            log::info!("Successfully loaded device settings: {:?}", settings);
            settings
        }
        Err(e) => {
            log::error!("Failed to load device settings: {:?}", e);
            log::info!("Using default device settings due to load error");
            DeviceSettings::new()
        }
    };
    let device_settings_read = device_settings.clone();

    let device_settings = Arc::new(Mutex::new(device_settings));

    log::info!("Device settings: {:?}", device_settings.lock().await);

    let mut config = match ConfigBuilder::new_with_id(ConfigId::Desktop).build() {
        Ok(mut config) => match config.load().await {
            Ok(config) => config,
            Err(e) => {
                log::info!("Failed to load config: {:?}", e);
                match ConfigBuilder::new().build_default_desktop().build() {
                    Ok(config) => config,
                    Err(build_err) => {
                        log::error!("Failed to build default desktop config: {:?}", build_err);
                        return Err(format!(
                            "Configuration initialization failed: {:?}",
                            build_err
                        )
                        .into());
                    }
                }
            }
        },
        Err(e) => {
            log::error!("Failed to build config: {:?}", e);
            return Err(format!("Configuration build failed: {:?}", e).into());
        }
    };
    let config_state = ConfigState::new(&mut config).await?;

    // Initialize thesaurus for roles that need it to prevent KG processing warnings
    {
        let current_config = config_state.config.lock().await;
        for (role_name, role) in &current_config.roles {
            if role.terraphim_it && role.kg.is_some() {
                log::info!(
                    "Checking thesaurus for role '{}' with terraphim_it enabled",
                    role_name
                );

                // Try to load existing thesaurus, if it doesn't exist, it will be built when needed
                let mut thesaurus = terraphim_types::Thesaurus::new(role_name.to_string());
                match thesaurus.load().await {
                    Ok(_) => {
                        log::info!("✅ Thesaurus already exists for role '{}'", role_name);
                    }
                    Err(_) => {
                        log::info!(
                            "🔧 Thesaurus not found for role '{}' - will be built on first use",
                            role_name
                        );
                        // Don't build it synchronously as it can be slow - let it build on demand
                    }
                }
            }
        }
    }

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
                        let role_name = id_clone.strip_prefix("change_role_").unwrap().to_string();
                        log::info!("User requested to change role from tray to {}", role_name);
                        let config_state: tauri::State<ConfigState> = app_handle.state();

                        match cmd::select_role(app_handle.clone(), config_state, role_name.clone())
                            .await
                        {
                            Ok(config_response) => {
                                let new_tray_menu = build_tray_menu(&config_response.config);
                                if let Err(e) = app_handle.tray_handle().set_menu(new_tray_menu) {
                                    log::error!("Failed to set new tray menu: {}", e);
                                }
                            }
                            Err(e) => {
                                log::error!("Failed to select role from tray menu: {}", e);
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
                                            log::error!(
                                                "Error checking window visibility: {:?}",
                                                e
                                            );
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
                                            log::error!(
                                                "Error checking window visibility: {:?}",
                                                e
                                            );
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
            cmd::select_role,
            cmd::get_rolegraph,
            cmd::find_documents_for_kg_term,
            cmd::save_article_to_atomic,
            cmd::get_autocomplete_suggestions,
            // Conversation management commands
            cmd::create_conversation,
            cmd::list_conversations,
            cmd::get_conversation,
            cmd::add_message_to_conversation,
            cmd::add_context_to_conversation,
            cmd::add_search_context_to_conversation,
            cmd::delete_context,
            cmd::update_context,
            // Chat command
            cmd::chat,
            // KG search commands
            cmd::search_kg_terms,
            cmd::add_kg_term_context,
            cmd::add_kg_index_context,
            // 1Password integration commands
            cmd::onepassword_status,
            cmd::onepassword_resolve_secret,
            cmd::onepassword_process_config,
            cmd::onepassword_load_settings
        ])
        .setup(move |app| {
            let settings = device_settings_read.clone();
            let handle = app.handle();

            // Initialize user data folder with bundled content if needed
            let handle_clone = handle.clone();
            let settings_clone = settings.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = initialize_user_data_folder(&handle_clone, &settings_clone).await {
                    log::error!("Failed to initialize user data folder: {:?}", e);
                }
            });

            // Try to get the main window with different possible labels
            let main_window = ["main", ""]
                .iter()
                .filter_map(|label| app.get_window(label))
                .next()
                .or_else(|| {
                    // If no window found with expected labels, get the first available window
                    app.windows().values().next().cloned()
                });

            if let Some(main_window) = main_window {
                if !settings.initialized {
                    tauri::async_runtime::spawn(async move {
                        let splashscreen_window = WindowBuilder::new(
                            &handle,
                            "splashscreen",
                            tauri::WindowUrl::App("../dist/splashscreen.html".into()),
                        )
                        .title("Splashscreen")
                        .resizable(true)
                        .decorations(true)
                        .always_on_top(false)
                        .inner_size(800.0, 600.0)
                        .build()
                        .unwrap();
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
                        .register(&global_shortcut, move || match window_clone.is_visible() {
                            Ok(true) => {
                                let _ = window_clone.hide();
                            }
                            Ok(false) => {
                                let _ = window_clone.show();
                            }
                            Err(e) => {
                                log::error!(
                                    "Error checking window visibility in global shortcut: {:?}",
                                    e
                                );
                            }
                        })
                        .unwrap();
                    window_found = true;
                    break;
                }
            }

            if !window_found {
                log::error!(
                    "No window found for global shortcut with labels: {:?}",
                    window_labels
                );
                // Try to get any available window
                let windows = app_handle.windows();
                if let Some((_, window)) = windows.iter().next() {
                    let _ = window.hide();
                    let window_clone = window.clone();
                    app_handle
                        .global_shortcut_manager()
                        .register(&global_shortcut, move || match window_clone.is_visible() {
                            Ok(true) => {
                                let _ = window_clone.hide();
                            }
                            Ok(false) => {
                                let _ = window_clone.show();
                            }
                            Err(e) => {
                                log::error!(
                                    "Error checking window visibility in global shortcut: {:?}",
                                    e
                                );
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
