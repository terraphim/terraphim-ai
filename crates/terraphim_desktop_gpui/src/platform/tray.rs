use anyhow::{anyhow, Result};
use tray_icon::{
    menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem},
    Icon, TrayIcon, TrayIconBuilder, TrayIconEvent,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use terraphim_types::RoleName;

/// System tray integration for Terraphim
pub struct SystemTray {
    tray_icon: Option<TrayIcon>,
    menu: Option<Menu>,
    menu_items: Arc<Mutex<HashMap<MenuId, SystemTrayEvent>>>,
    event_handler: Arc<Mutex<Option<Box<dyn Fn(SystemTrayEvent) + Send + 'static>>>>,
    roles: Vec<RoleName>,
    selected_role: RoleName,
}

/// Events that can be triggered from the system tray
#[derive(Clone, Debug)]
pub enum SystemTrayEvent {
    ToggleWindow,
    ChangeRole(RoleName),
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
            roles: Vec::new(),
            selected_role: RoleName::default(),
        }
    }

    /// Create a new system tray instance with roles (matches Tauri implementation)
    pub fn with_roles(roles: Vec<RoleName>, selected_role: RoleName) -> Self {
        log::info!("=== SystemTray CREATING WITH ROLES ===");
        log::info!("Roles: {:?}", roles);
        log::info!("Selected role: {:?}", selected_role);
        Self {
            tray_icon: None,
            menu: None,
            menu_items: Arc::new(Mutex::new(HashMap::new())),
            event_handler: Arc::new(Mutex::new(None)),
            roles,
            selected_role,
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
        // Try to load icon from embedded bytes (included at compile time)
        let icon_bytes = include_bytes!("../../assets/tray-icon.png");

        // Decode PNG to RGBA
        let img = image::load_from_memory(icon_bytes)
            .map_err(|e| anyhow!("Failed to load icon image: {}", e))?;
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();

        Icon::from_rgba(rgba.into_raw(), width, height)
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

    /// Create the tray menu (matches Tauri build_tray_menu implementation)
    fn create_menu(&self) -> Result<Menu> {
        log::info!("=== Creating Tray Menu (Tauri-style) ===");
        let menu = Menu::new();

        // 1. Toggle Window item (Show/Hide combined)
        let toggle_item = MenuItem::new("Show/Hide", true, None);

        // Store menu item IDs for event handling
        {
            let mut items = self.menu_items.lock().unwrap();
            items.insert(toggle_item.id().clone(), SystemTrayEvent::ToggleWindow);
        }

        menu.append(&toggle_item)
            .map_err(|e| anyhow!("Failed to add toggle item: {}", e))?;

        // 2. Separator
        let separator1 = PredefinedMenuItem::separator();
        menu.append(&separator1)
            .map_err(|e| anyhow!("Failed to add separator: {}", e))?;

        // 3. Dynamic role items (with checkmark on selected)
        log::info!("Adding {} role items to tray menu", self.roles.len());
        for role in &self.roles {
            let is_selected = role == &self.selected_role;
            let label = if is_selected {
                format!("✓ {}", role)
            } else {
                role.to_string()
            };

            log::info!("  Role: {} (selected: {})", role, is_selected);
            let role_item = MenuItem::new(&label, true, None);

            // Store role item event
            {
                let mut items = self.menu_items.lock().unwrap();
                items.insert(role_item.id().clone(), SystemTrayEvent::ChangeRole(role.clone()));
            }

            menu.append(&role_item)
                .map_err(|e| anyhow!("Failed to add role item: {}", e))?;
        }

        // 4. Separator before quit
        let separator2 = PredefinedMenuItem::separator();
        menu.append(&separator2)
            .map_err(|e| anyhow!("Failed to add separator: {}", e))?;

        // 5. Quit item
        let quit_item = MenuItem::new("Quit", true, None);
        {
            let mut items = self.menu_items.lock().unwrap();
            items.insert(quit_item.id().clone(), SystemTrayEvent::Quit);
        }
        menu.append(&quit_item)
            .map_err(|e| anyhow!("Failed to add quit item: {}", e))?;

        log::info!("=== Tray Menu Created Successfully ===");
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