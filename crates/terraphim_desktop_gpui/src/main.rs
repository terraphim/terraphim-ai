use gpui::*;

mod actions;
mod app;
mod autocomplete;
mod models;
mod search_service;
mod state;
mod theme;
mod views;

use app::TerraphimApp;

fn main() {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("Starting Terraphim Desktop GPUI");

    // Initialize GPUI app
    App::new().run(|cx: &mut AppContext| {
        // Register app-wide actions
        actions::register_app_actions(cx);

        // Configure theme
        theme::configure_theme(cx);

        // Open main window
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
            },
            |cx| cx.new_view(|cx| TerraphimApp::new(cx)),
        )
        .expect("Failed to open main window");

        log::info!("Terraphim Desktop window opened successfully");
    });
}
