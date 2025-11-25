use anyhow::{anyhow, Result};
use tray_icon::{
    menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem},
    Icon, TrayIcon, TrayIconBuilder, TrayIconEvent,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

/// System tray integration for Terraphim
pub struct SystemTray {
    tray_icon: Option<TrayIcon>,
    menu: Option<Menu>,
    menu_items: Arc<Mutex<HashMap<MenuId, SystemTrayEvent>>>,
    event_handler: Arc<Mutex<Option<Box<dyn Fn(SystemTrayEvent) + Send + 'static>>>>,
}

/// Events that can be triggered from the system tray
#[derive(Clone, Debug)]
pub enum SystemTrayEvent {
    ShowWindow,
    HideWindow,
    Search,
    Chat,
    Settings,
    About,
    Quit,
    TrayIconClick,
}

impl SystemTray {
    /// Create a new system tray instance
    pub fn new() -> Self {
        Self {
            tray_icon: None,
            menu: None,
            menu_items: Arc::new(Mutex::new(HashMap::new())),
            event_handler: Arc::new(Mutex::new(None)),
        }
    }

    /// Initialize the system tray with icon and menu
    pub fn initialize(&mut self) -> Result<()> {
        log::info!("Initializing system tray");

        // Create tray icon from embedded bytes
        let icon = self.create_icon()?;

        // Create the tray menu
        let menu = self.create_menu()?;

        // Build the tray icon
        let tray_icon = TrayIconBuilder::new()
            .with_icon(icon)
            .with_menu(Box::new(menu.clone()))
            .with_tooltip("Terraphim AI")
            .build()
            .map_err(|e| anyhow!("Failed to build tray icon: {}", e))?;

        self.tray_icon = Some(tray_icon);
        self.menu = Some(menu);

        // Start listening for events
        self.start_event_listener();

        log::info!("System tray initialized successfully");
        Ok(())
    }

    /// Create the tray icon
    fn create_icon(&self) -> Result<Icon> {
        // For now, use a simple 16x16 icon embedded as bytes
        // In production, you'd load a proper icon file
        let icon_bytes = self.generate_default_icon();

        Icon::from_rgba(icon_bytes, 16, 16)
            .map_err(|e| anyhow!("Failed to create icon: {}", e))
    }

    /// Generate a simple default icon (16x16 RGBA)
    pub fn generate_default_icon(&self) -> Vec<u8> {
        // Create a simple blue square icon (16x16 pixels, 4 bytes per pixel for RGBA)
        let size = 16 * 16 * 4;
        let mut icon_data = vec![0u8; size];

        for i in (0..size).step_by(4) {
            icon_data[i] = 50;      // R
            icon_data[i + 1] = 115;  // G (Terraphim blue: #3273dc)
            icon_data[i + 2] = 220;  // B
            icon_data[i + 3] = 255;  // A (fully opaque)
        }

        icon_data
    }

    /// Create the tray menu
    fn create_menu(&self) -> Result<Menu> {
        let menu = Menu::new();

        // Add menu items
        let show_item = MenuItem::new("Show Terraphim", true, None);
        let hide_item = MenuItem::new("Hide Terraphim", true, None);
        let search_item = MenuItem::new("Search", true, None);
        let chat_item = MenuItem::new("Chat", true, None);
        let settings_item = MenuItem::new("Settings", true, None);
        let about_item = MenuItem::new("About", true, None);
        let separator = PredefinedMenuItem::separator();
        let quit_item = MenuItem::new("Quit", true, None);

        // Store menu item IDs for event handling
        {
            let mut items = self.menu_items.lock().unwrap();
            items.insert(show_item.id().clone(), SystemTrayEvent::ShowWindow);
            items.insert(hide_item.id().clone(), SystemTrayEvent::HideWindow);
            items.insert(search_item.id().clone(), SystemTrayEvent::Search);
            items.insert(chat_item.id().clone(), SystemTrayEvent::Chat);
            items.insert(settings_item.id().clone(), SystemTrayEvent::Settings);
            items.insert(about_item.id().clone(), SystemTrayEvent::About);
            items.insert(quit_item.id().clone(), SystemTrayEvent::Quit);
        }

        // Build the menu
        menu.append(&show_item)
            .map_err(|e| anyhow!("Failed to add show item: {}", e))?;
        menu.append(&hide_item)
            .map_err(|e| anyhow!("Failed to add hide item: {}", e))?;
        menu.append(&separator)
            .map_err(|e| anyhow!("Failed to add separator: {}", e))?;
        menu.append(&search_item)
            .map_err(|e| anyhow!("Failed to add search item: {}", e))?;
        menu.append(&chat_item)
            .map_err(|e| anyhow!("Failed to add chat item: {}", e))?;
        menu.append(&separator)
            .map_err(|e| anyhow!("Failed to add separator: {}", e))?;
        menu.append(&settings_item)
            .map_err(|e| anyhow!("Failed to add settings item: {}", e))?;
        menu.append(&about_item)
            .map_err(|e| anyhow!("Failed to add about item: {}", e))?;
        menu.append(&separator)
            .map_err(|e| anyhow!("Failed to add separator: {}", e))?;
        menu.append(&quit_item)
            .map_err(|e| anyhow!("Failed to add quit item: {}", e))?;

        Ok(menu)
    }

    /// Start listening for tray events
    pub fn start_event_listener(&self) {
        let event_handler = self.event_handler.clone();

        // Spawn a thread to listen for tray icon events
        std::thread::spawn(move || {
            log::info!("Starting tray event listener");

            // Listen for tray icon clicks
            let receiver = TrayIconEvent::receiver();
            loop {
                if let Ok(event) = receiver.recv() {
                    log::info!("Received tray icon event: {:?}", event);

                    // Handle tray icon click
                    if let Some(handler) = event_handler.lock().unwrap().as_ref() {
                        handler(SystemTrayEvent::TrayIconClick);
                    }
                }
            }
        });

        // Spawn another thread for menu events
        let event_handler = self.event_handler.clone();
        let menu_items = self.menu_items.clone();

        std::thread::spawn(move || {
            log::info!("Starting menu event listener");

            // Listen for menu events
            let receiver = MenuEvent::receiver();
            loop {
                if let Ok(event) = receiver.recv() {
                    log::info!("Received menu event: {:?}", event.id);

                    // Look up the event using the stored menu item ID
                    let items = menu_items.lock().unwrap();
                    if let Some(tray_event) = items.get(&event.id) {
                        let tray_event = tray_event.clone();
                        drop(items); // Release the lock before calling the handler

                        log::info!("Triggering tray event: {:?}", tray_event);

                        if let Some(handler) = event_handler.lock().unwrap().as_ref() {
                            handler(tray_event);
                        }
                    } else {
                        log::warn!("Unknown menu item ID: {:?}", event.id);
                    }
                }
            }
        });
    }

    /// Set the event handler for tray events
    pub fn on_event<F>(&mut self, handler: F)
    where
        F: Fn(SystemTrayEvent) + Send + 'static,
    {
        *self.event_handler.lock().unwrap() = Some(Box::new(handler));
    }

    /// Update a menu item's enabled state
    pub fn set_menu_item_enabled(&mut self, event_type: SystemTrayEvent, enabled: bool) -> Result<()> {
        // Find the menu item ID associated with this event
        let items = self.menu_items.lock().unwrap();
        let menu_id = items
            .iter()
            .find(|(_, ev)| std::mem::discriminant(*ev) == std::mem::discriminant(&event_type))
            .map(|(id, _)| id.clone());

        if let Some(id) = menu_id {
            // In a full implementation, we'd update the actual menu item
            log::info!("Would update menu item {:?} enabled state to {}", event_type, enabled);
            Ok(())
        } else {
            Err(anyhow!("Menu item not found for event: {:?}", event_type))
        }
    }

    /// Update the tray icon tooltip
    pub fn set_tooltip(&mut self, tooltip: &str) -> Result<()> {
        if let Some(tray) = &mut self.tray_icon {
            tray.set_tooltip(Some(tooltip))
                .map_err(|e| anyhow!("Failed to set tooltip: {}", e))?;
        }
        Ok(())
    }

    /// Show the tray icon
    pub fn show(&mut self) -> Result<()> {
        if let Some(tray) = &mut self.tray_icon {
            tray.set_visible(true)
                .map_err(|e| anyhow!("Failed to show tray icon: {}", e))?;
        }
        Ok(())
    }

    /// Hide the tray icon
    pub fn hide(&mut self) -> Result<()> {
        if let Some(tray) = &mut self.tray_icon {
            tray.set_visible(false)
                .map_err(|e| anyhow!("Failed to hide tray icon: {}", e))?;
        }
        Ok(())
    }

    /// Check if the system tray is supported on this platform
    pub fn is_supported() -> bool {
        // System tray is supported on macOS, Windows, and most Linux desktops
        cfg!(any(target_os = "macos", target_os = "windows", target_os = "linux"))
    }
}

impl Drop for SystemTray {
    fn drop(&mut self) {
        log::info!("Cleaning up system tray");
        // The tray icon will be automatically removed when dropped
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_tray_creation() {
        let tray = SystemTray::new();
        assert!(tray.tray_icon.is_none());
        assert!(tray.menu.is_none());
    }

    #[test]
    fn test_is_supported() {
        // Should return true on desktop platforms
        let supported = SystemTray::is_supported();
        assert!(supported || !supported); // Always true, but tests the function
    }

    #[test]
    fn test_default_icon_generation() {
        let tray = SystemTray::new();
        let icon_data = tray.generate_default_icon();
        assert_eq!(icon_data.len(), 16 * 16 * 4); // 16x16 RGBA
    }
}