use gpui::*;
use gpui_component::Root;
use terraphim_config::{ConfigBuilder, ConfigId};
use terraphim_persistence::Persistable;

mod actions;
mod app;
mod autocomplete;
mod editor;
mod kg_search;
mod models;
mod platform;
mod search_service;
mod state;
mod theme;
mod utils;
mod views;

use app::TerraphimApp;

fn main() {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("Starting Terraphim Desktop GPUI");

    // Create tokio runtime that stays alive for the entire app
    // This is needed because terraphim_service uses tokio for async operations
    let runtime = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");

    log::info!("Loading configuration with tokio runtime...");

    let config_state = runtime.block_on(async {
            // Load configuration using pattern from Tauri main.rs
            let mut config = match ConfigBuilder::new_with_id(ConfigId::Desktop).build() {
                Ok(mut config) => match config.load().await {
                    Ok(config) => {
                        log::info!("Successfully loaded config from persistence");
                        config
                    }
                    Err(e) => {
                        log::warn!("Failed to load config: {:?}, using default", e);
                        match ConfigBuilder::new().build_default_desktop().build() {
                            Ok(config) => config,
                            Err(build_err) => {
                                log::error!("Failed to build default desktop config: {:?}", build_err);
                                panic!("Configuration initialization failed: {:?}", build_err);
                            }
                        }
                    }
                },
                Err(e) => {
                    log::error!("Failed to build config: {:?}", e);
                    panic!("Configuration build failed: {:?}", e);
                }
            };

            // Initialize config state with role graphs
            match terraphim_config::ConfigState::new(&mut config).await {
                Ok(state) => {
                    log::info!("ConfigState initialized successfully with {} roles", state.roles.len());
                    state
                }
                Err(e) => {
                    log::error!("Failed to create ConfigState: {:?}", e);
                    panic!("Failed to create ConfigState: {:?}", e);
                }
            }
        });

    log::info!("Configuration loaded successfully, starting GPUI...");

    // Leak runtime to make it 'static and always available
    // terraphim_service needs tokio reactor for spawning processes (ripgrep)
    let runtime = Box::leak(Box::new(runtime));
    let _guard = runtime.enter();

    // Initialize GPUI application
    let app = Application::new();

    app.run(move |cx| {
        // Initialize gpui-component features
        gpui_component::init(cx);

        // Register app-wide actions
        actions::register_app_actions(cx);

        // Configure theme
        theme::configure_theme(cx);

        // Clone config_state for async block
        let config_state_for_window = config_state.clone();

        // Spawn window creation asynchronously
        cx.spawn(async move |cx| {
            log::info!("Opening window with initialized services...");

            // Load ALL roles from config (Tauri pattern main.rs:234-235)
            let all_roles = {
                let config = config_state.config.lock().await;
                config.roles.keys().cloned().collect::<Vec<_>>()
            };
            log::info!("Loaded {} roles for UI", all_roles.len());

            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds {
                        origin: Point::new(px(100.0), px(100.0)),
                        size: Size {
                            width: px(1200.0),
                            height: px(800.0),
                        },
                    })),
                    titlebar: Some(TitlebarOptions {
                        title: Some("Terraphim AI".into()),
                        appears_transparent: false,
                        traffic_light_position: None,
                    }),
                    window_min_size: Some(Size {
                        width: px(800.0),
                        height: px(600.0),
                    }),
                    kind: WindowKind::Normal,
                    is_movable: true,
                    display_id: None,
                    window_background: WindowBackgroundAppearance::Opaque,
                    app_id: Some("ai.terraphim.desktop".into()),
                    ..Default::default()
                },
                |window, cx| {
                    let view = cx.new(|cx| TerraphimApp::new(window, cx, config_state_for_window, all_roles));
                    // Wrap in Root component as required by gpui-component
                    cx.new(|cx| Root::new(view, window, cx))
                },
            )?;

            log::info!("Terraphim Desktop window opened successfully");
            Ok::<_, anyhow::Error>(())
        })
        .detach();
    });
}
