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

pub fn register_app_actions(cx: &mut AppContext) {
    log::info!("Registering app-wide actions and key bindings");

    // Navigation shortcuts
    cx.bind_keys([
        KeyBinding::new("cmd-1", NavigateToSearch, None),
        KeyBinding::new("cmd-2", NavigateToChat, None),
        KeyBinding::new("cmd-3", NavigateToEditor, None),
    ]);

    // Utility shortcuts
    cx.bind_keys([
        KeyBinding::new("cmd-shift-t", ToggleTheme, None),
        KeyBinding::new("cmd-,", OpenSettings, None),
        KeyBinding::new("cmd-n", NewConversation, Some("Chat")),
        KeyBinding::new("cmd-k", GlobalSearch, None),
    ]);

    log::info!("Registered {} global key bindings", 7);
}
