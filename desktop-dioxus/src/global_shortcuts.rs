use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager,
};
use std::sync::Arc;

pub struct ShortcutManager {
    manager: GlobalHotKeyManager,
    toggle_hotkey: Option<HotKey>,
}

impl ShortcutManager {
    pub fn new() -> anyhow::Result<Self> {
        let manager = GlobalHotKeyManager::new()?;
        Ok(Self {
            manager,
            toggle_hotkey: None,
        })
    }

    /// Register the toggle window shortcut (Ctrl+Shift+Space by default)
    pub fn register_toggle_shortcut(&mut self) -> anyhow::Result<()> {
        // Create hotkey: Ctrl+Shift+Space
        let hotkey = HotKey::new(
            Some(Modifiers::CONTROL | Modifiers::SHIFT),
            Code::Space,
        );

        self.manager.register(hotkey)?;
        self.toggle_hotkey = Some(hotkey);

        tracing::info!("Registered global shortcut: Ctrl+Shift+Space");
        Ok(())
    }

    /// Register a custom shortcut from string (e.g., "Ctrl+Shift+T")
    pub fn register_custom_shortcut(
        &mut self,
        shortcut_str: &str,
    ) -> anyhow::Result<()> {
        // Parse shortcut string
        let parts: Vec<&str> = shortcut_str.split('+').collect();

        let mut modifiers = Modifiers::empty();
        let mut key_code = None;

        for part in parts {
            match part.trim().to_lowercase().as_str() {
                "ctrl" | "control" => modifiers |= Modifiers::CONTROL,
                "shift" => modifiers |= Modifiers::SHIFT,
                "alt" => modifiers |= Modifiers::ALT,
                "super" | "meta" | "cmd" => modifiers |= Modifiers::SUPER,
                key => {
                    // Parse key code
                    key_code = Some(Self::parse_key_code(key)?);
                }
            }
        }

        if let Some(code) = key_code {
            let hotkey = HotKey::new(Some(modifiers), code);
            self.manager.register(hotkey)?;
            self.toggle_hotkey = Some(hotkey);
            tracing::info!("Registered custom shortcut: {}", shortcut_str);
            Ok(())
        } else {
            Err(anyhow::anyhow!("No key code found in shortcut string"))
        }
    }

    fn parse_key_code(key: &str) -> anyhow::Result<Code> {
        match key.to_lowercase().as_str() {
            "space" => Ok(Code::Space),
            "enter" | "return" => Ok(Code::Enter),
            "escape" | "esc" => Ok(Code::Escape),
            "tab" => Ok(Code::Tab),
            "backspace" => Ok(Code::Backspace),
            "delete" | "del" => Ok(Code::Delete),
            "a" => Ok(Code::KeyA),
            "b" => Ok(Code::KeyB),
            "c" => Ok(Code::KeyC),
            "d" => Ok(Code::KeyD),
            "e" => Ok(Code::KeyE),
            "f" => Ok(Code::KeyF),
            "g" => Ok(Code::KeyG),
            "h" => Ok(Code::KeyH),
            "i" => Ok(Code::KeyI),
            "j" => Ok(Code::KeyJ),
            "k" => Ok(Code::KeyK),
            "l" => Ok(Code::KeyL),
            "m" => Ok(Code::KeyM),
            "n" => Ok(Code::KeyN),
            "o" => Ok(Code::KeyO),
            "p" => Ok(Code::KeyP),
            "q" => Ok(Code::KeyQ),
            "r" => Ok(Code::KeyR),
            "s" => Ok(Code::KeyS),
            "t" => Ok(Code::KeyT),
            "u" => Ok(Code::KeyU),
            "v" => Ok(Code::KeyV),
            "w" => Ok(Code::KeyW),
            "x" => Ok(Code::KeyX),
            "y" => Ok(Code::KeyY),
            "z" => Ok(Code::KeyZ),
            "f1" => Ok(Code::F1),
            "f2" => Ok(Code::F2),
            "f3" => Ok(Code::F3),
            "f4" => Ok(Code::F4),
            "f5" => Ok(Code::F5),
            "f6" => Ok(Code::F6),
            "f7" => Ok(Code::F7),
            "f8" => Ok(Code::F8),
            "f9" => Ok(Code::F9),
            "f10" => Ok(Code::F10),
            "f11" => Ok(Code::F11),
            "f12" => Ok(Code::F12),
            _ => Err(anyhow::anyhow!("Unknown key code: {}", key)),
        }
    }

    pub fn unregister_all(&mut self) -> anyhow::Result<()> {
        if let Some(hotkey) = self.toggle_hotkey {
            self.manager.unregister(hotkey)?;
            self.toggle_hotkey = None;
        }
        Ok(())
    }
}

/// Listen for global shortcut events
pub async fn listen_for_shortcuts<F>(mut callback: F)
where
    F: FnMut() + Send + 'static,
{
    let receiver = GlobalHotKeyEvent::receiver();

    loop {
        if let Ok(_event) = receiver.recv() {
            tracing::info!("Global shortcut triggered");
            callback();
        }
    }
}
