use tray_icon::{
    menu::{Menu, MenuItem, MenuEvent, PredefinedMenuItem},
    TrayIconBuilder, TrayIcon, Icon,
};
use terraphim_config::Config;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct SystemTrayManager {
    tray_icon: Option<TrayIcon>,
    menu: Menu,
}

impl SystemTrayManager {
    pub fn new() -> anyhow::Result<Self> {
        let menu = Menu::new();
        Ok(Self {
            tray_icon: None,
            menu,
        })
    }

    pub fn build_menu(&mut self, config: &Config) -> anyhow::Result<()> {
        self.menu = Menu::new();

        // Toggle window item
        let toggle_item = MenuItem::new("Show/Hide", true, Some("toggle".into()));
        self.menu.append(&toggle_item)?;
        self.menu.append(&PredefinedMenuItem::separator())?;

        // Role items
        for (role_name, _) in &config.roles {
            let item_id = format!("role_{}", role_name.original);
            let mut item = MenuItem::new(&role_name.original, true, Some(item_id.clone().into()));

            // Mark selected role
            if role_name == &config.selected_role {
                item.set_enabled(true);
                // Note: tray-icon doesn't have set_checked, we can use different text or icon
            }

            self.menu.append(&item)?;
        }

        self.menu.append(&PredefinedMenuItem::separator())?;

        // Quit item
        let quit_item = MenuItem::new("Quit", true, Some("quit".into()));
        self.menu.append(&quit_item)?;

        Ok(())
    }

    pub fn create_tray_icon(&mut self, icon_path: &str) -> anyhow::Result<()> {
        // Load icon
        let icon_image = image::open(icon_path)
            .map_err(|e| anyhow::anyhow!("Failed to load icon: {}", e))?;
        let icon_rgba = icon_image.to_rgba8();
        let (width, height) = icon_rgba.dimensions();

        let icon = Icon::from_rgba(icon_rgba.into_raw(), width, height)?;

        // Build tray icon
        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(self.menu.clone()))
            .with_tooltip("Terraphim AI")
            .with_icon(icon)
            .build()?;

        self.tray_icon = Some(tray);
        Ok(())
    }

    pub fn update_menu(&mut self, config: &Config) -> anyhow::Result<()> {
        self.build_menu(config)?;
        if let Some(tray) = &self.tray_icon {
            tray.set_menu(Some(Box::new(self.menu.clone())))?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum TrayEvent {
    Toggle,
    RoleChanged(String),
    Quit,
}

pub fn handle_menu_event(event: MenuEvent) -> Option<TrayEvent> {
    if let Some(id) = event.id.0.as_ref() {
        match id.as_str() {
            "quit" => Some(TrayEvent::Quit),
            "toggle" | "show_hide" => Some(TrayEvent::Toggle),
            id if id.starts_with("role_") => {
                let role_name = id.strip_prefix("role_").unwrap().to_string();
                Some(TrayEvent::RoleChanged(role_name))
            }
            _ => None,
        }
    } else {
        None
    }
}
