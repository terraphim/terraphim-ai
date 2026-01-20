use gpui::*;

actions!(
    app,
    [
        NavigateToSearch,
        NavigateToChat,
        NavigateToEditor,
        ToggleTheme,
        OpenSettings,
        NewConversation,
        GlobalSearch,
    ]
);

pub fn register_app_actions(_cx: &mut impl AppContext) {
    log::info!("Registering app-wide actions and key bindings");

    // App-level keybindings are disabled pending GPUI 0.2.2 API research
    // The system uses global hotkeys via platform/hotkeys.rs instead
    // which provide the same functionality:
    // - Shift+Super+Space: Show/Hide window
    // - Shift+Super+KeyS: Quick search
    // - Shift+Super+KeyC: Open chat
    // - Shift+Super+KeyE: Open editor

    // TODO: Research proper GPUI 0.2.2 keybinding API for app-level shortcuts
    // Navigation shortcuts
    // cx.bind_keys([
    //     KeyBinding::new("cmd-1", NavigateToSearch, None),
    //     KeyBinding::new("cmd-2", NavigateToChat, None),
    //     KeyBinding::new("cmd-3", NavigateToEditor, None),
    // ]);

    // Utility shortcuts
    // cx.bind_keys([
    //     KeyBinding::new("cmd-shift-t", ToggleTheme, None),
    //     KeyBinding::new("cmd-,", OpenSettings, None),
    //     KeyBinding::new("cmd-n", NewConversation, Some("Chat")),
    //     KeyBinding::new("cmd-k", GlobalSearch, None),
    // ]);

    log::info!("Keybindings: Using global hotkeys (platform/hotkeys.rs)");
}
