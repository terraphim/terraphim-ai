//! Main entry point for Terraphim AI Egui application
//!
//! This application provides a native desktop UI for the Terraphim AI assistant
//! with knowledge graph search, LLM integration, and role-based context management.

use std::sync::Arc;
use tokio::runtime::Runtime;
use tracing::{error, info};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt};

use terraphim_egui::app::EguiApp;

fn main() -> Result<(), eframe::Error> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(tracing_subscriber::filter::LevelFilter::INFO)
        .init();

    info!("Starting Terraphim AI Egui application");

    // Create tokio runtime for async operations
    let rt = Arc::new(Runtime::new().expect("Failed to create tokio runtime"));
    let _ = rt; // Keep runtime alive for the duration of the app

    // Set up the native window options
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("Terraphim AI - Privacy-First Knowledge Assistant")
            .with_min_inner_size([600.0, 400.0])
            .with_decorations(true)
            .with_transparent(false)
            .with_resizable(true),
        // Enable dark mode by default
        ..Default::default()
    };

    // Run the application
    let result = eframe::run_native(
        "Terraphim AI",
        native_options,
        Box::new(|cc| {
            // Set up app initial state
            Box::new(EguiApp::new(cc))
        }),
    );

    match result {
        Ok(_) => {
            info!("Terraphim AI Egui application exited successfully");
            Ok(())
        }
        Err(e) => {
            error!("Error running Terraphim AI Egui application: {}", e);
            Err(e)
        }
    }
}
