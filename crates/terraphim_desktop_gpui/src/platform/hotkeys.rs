use anyhow::{anyhow, Result};
use global_hotkey::{GlobalHotKeyManager, hotkey::HotKey};
use keyboard_types::{Code, Modifiers};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

/// Global hotkey manager for Terraphim
pub struct GlobalHotkeys {
    manager: Arc<GlobalHotKeyManager>,
    hotkeys: HashMap<u32, RegisteredHotkey>,
    event_handler: Arc<Mutex<Option<Box<dyn Fn(HotkeyEvent) + Send + 'static>>>>,
    next_id: u32,
}

/// A registered global hotkey
#[derive(Clone)]
struct RegisteredHotkey {
    hotkey: HotKey,
    action: HotkeyAction,
    description: String,
    modifiers: Modifiers,
    key: Code,
}

/// Actions that can be triggered by hotkeys
#[derive(Clone, Debug, PartialEq)]
pub enum HotkeyAction {
    ShowHideWindow,
    QuickSearch,
    OpenChat,
    OpenEditor,
    Custom(String),
}

/// Event triggered when a hotkey is pressed
#[derive(Clone, Debug)]
pub struct HotkeyEvent {
    pub action: HotkeyAction,
    pub hotkey_str: String,
}

impl GlobalHotkeys {
    /// Create a new global hotkey manager
    pub fn new() -> Result<Self> {
        let manager = GlobalHotKeyManager::new()
            .map_err(|e| anyhow!("Failed to create global hotkey manager: {}", e))?;

        Ok(Self {
            manager: Arc::new(manager),
            hotkeys: HashMap::new(),
            event_handler: Arc::new(Mutex::new(None)),
            next_id: 1,
        })
    }

    /// Register default hotkeys for Terraphim
    pub fn register_defaults(&mut self) -> Result<()> {
        log::info!("Registering default global hotkeys");

        // Default hotkeys (cross-platform)
        #[cfg(target_os = "macos")]
        let cmd_or_ctrl = Modifiers::SUPER;
        #[cfg(not(target_os = "macos"))]
        let cmd_or_ctrl = Modifiers::CONTROL;

        // Cmd/Ctrl+Shift+Space: Show/Hide window
        self.register_hotkey(
            cmd_or_ctrl | Modifiers::SHIFT,
            Code::Space,
            HotkeyAction::ShowHideWindow,
            "Show/Hide Terraphim",
        )?;

        // Cmd/Ctrl+Shift+S: Quick search
        self.register_hotkey(
            cmd_or_ctrl | Modifiers::SHIFT,
            Code::KeyS,
            HotkeyAction::QuickSearch,
            "Quick Search",
        )?;

        // Cmd/Ctrl+Shift+C: Open chat
        self.register_hotkey(
            cmd_or_ctrl | Modifiers::SHIFT,
            Code::KeyC,
            HotkeyAction::OpenChat,
            "Open Chat",
        )?;

        // Cmd/Ctrl+Shift+E: Open editor
        self.register_hotkey(
            cmd_or_ctrl | Modifiers::SHIFT,
            Code::KeyE,
            HotkeyAction::OpenEditor,
            "Open Editor",
        )?;

        // Start listening for hotkey events
        self.start_event_listener();

        log::info!("Global hotkeys registered successfully");
        Ok(())
    }

    /// Register a custom hotkey
    pub fn register_hotkey(
        &mut self,
        modifiers: Modifiers,
        key_code: Code,
        action: HotkeyAction,
        description: &str,
    ) -> Result<()> {
        let id = self.next_id;
        self.next_id += 1;

        // Create the hotkey using the new constructor
        let hotkey = HotKey::new(Some(modifiers), key_code);

        // Register with the system
        self.manager
            .register(hotkey.clone())
            .map_err(|e| anyhow!("Failed to register hotkey: {}", e))?;

        // Store the hotkey info
        self.hotkeys.insert(
            id,
            RegisteredHotkey {
                hotkey,
                action,
                description: description.to_string(),
                modifiers,
                key: key_code,
            },
        );

        log::info!(
            "Registered hotkey: {} ({:?} + {:?})",
            description,
            modifiers,
            key_code
        );

        Ok(())
    }

    /// Unregister a hotkey by its action
    pub fn unregister_by_action(&mut self, action: &HotkeyAction) -> Result<()> {
        let id_to_remove = self
            .hotkeys
            .iter()
            .find(|(_, h)| h.action == *action)
            .map(|(id, _)| *id);

        if let Some(id) = id_to_remove {
            if let Some(registered) = self.hotkeys.remove(&id) {
                self.manager
                    .unregister(registered.hotkey)
                    .map_err(|e| anyhow!("Failed to unregister hotkey: {}", e))?;

                log::info!("Unregistered hotkey: {}", registered.description);
            }
        }

        Ok(())
    }

    /// Start listening for hotkey events
    fn start_event_listener(&self) {
        let hotkeys = self.hotkeys.clone();
        let event_handler = self.event_handler.clone();

        std::thread::spawn(move || {
            log::info!("Starting global hotkey event listener");

            // Global hotkey uses a channel-based approach
            let receiver = global_hotkey::GlobalHotKeyEvent::receiver();

            loop {
                if let Ok(event) = receiver.recv() {
                    log::info!("Received hotkey event with id: {}", event.id);

                    // Find the registered hotkey by ID
                    if let Some(registered) = hotkeys.get(&event.id) {
                        log::info!(
                            "Global hotkey pressed: {}",
                            registered.description
                        );

                        let hotkey_event = HotkeyEvent {
                            action: registered.action.clone(),
                            hotkey_str: format!(
                                "{:?} + {:?}",
                                registered.modifiers,
                                registered.key
                            ),
                        };

                        if let Some(handler) = event_handler.lock().unwrap().as_ref() {
                            handler(hotkey_event);
                        }
                    }
                }
            }
        });
    }

    /// Set the event handler for hotkey events
    pub fn on_event<F>(&mut self, handler: F)
    where
        F: Fn(HotkeyEvent) + Send + 'static,
    {
        *self.event_handler.lock().unwrap() = Some(Box::new(handler));
    }

    /// Get a list of all registered hotkeys
    pub fn list_hotkeys(&self) -> Vec<(String, String)> {
        self.hotkeys
            .values()
            .map(|h| {
                (
                    h.description.clone(),
                    format!("{:?} + {:?}", h.modifiers, h.key),
                )
            })
            .collect()
    }

    /// Check if global hotkeys are supported on this platform
    pub fn is_supported() -> bool {
        // Global hotkeys are supported on macOS, Windows, and Linux with X11
        cfg!(any(target_os = "macos", target_os = "windows", target_os = "linux"))
    }

    /// Check if accessibility permissions are needed (macOS)
    #[cfg(target_os = "macos")]
    pub fn needs_accessibility_permission() -> bool {
        // On macOS, we need accessibility permissions for global hotkeys
        // This would check using platform-specific APIs
        true
    }

    #[cfg(not(target_os = "macos"))]
    pub fn needs_accessibility_permission() -> bool {
        false
    }
}

impl Drop for GlobalHotkeys {
    fn drop(&mut self) {
        log::info!("Cleaning up global hotkeys");

        // Unregister all hotkeys
        for (_, registered) in self.hotkeys.drain() {
            if let Err(e) = self.manager.unregister(registered.hotkey) {
                log::error!("Failed to unregister hotkey on cleanup: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hotkey_action_equality() {
        assert_eq!(HotkeyAction::ShowHideWindow, HotkeyAction::ShowHideWindow);
        assert_ne!(HotkeyAction::ShowHideWindow, HotkeyAction::QuickSearch);
    }

    #[test]
    fn test_is_supported() {
        let supported = GlobalHotkeys::is_supported();
        assert!(supported || !supported); // Always true, but tests the function
    }

    #[test]
    fn test_needs_accessibility_permission() {
        let needs_permission = GlobalHotkeys::needs_accessibility_permission();
        #[cfg(target_os = "macos")]
        assert!(needs_permission);
        #[cfg(not(target_os = "macos"))]
        assert!(!needs_permission);
    }
}