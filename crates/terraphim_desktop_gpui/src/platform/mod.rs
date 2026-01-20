pub mod hotkeys;
pub mod tray;

pub use hotkeys::GlobalHotkeys;
pub use tray::SystemTray;

/// Wake up the application event loop.
/// This posts a synthetic event to ensure the run loop processes pending events.
/// Useful for triggering UI updates from background threads (e.g., tray menu callbacks).
#[cfg(target_os = "macos")]
pub fn wake_app_event_loop() {
    use cocoa::appkit::{NSApp, NSEventType};
    use cocoa::base::nil;
    use cocoa::foundation::NSPoint;
    use objc::{class, msg_send, sel, sel_impl};

    unsafe {
        let app: cocoa::base::id = NSApp();

        // Create a synthetic event (application-defined event)
        let event: cocoa::base::id = msg_send![
            class!(NSEvent),
            otherEventWithType: NSEventType::NSApplicationDefined
            location: NSPoint::new(0.0, 0.0)
            modifierFlags: 0u64
            timestamp: 0.0f64
            windowNumber: 0i64
            context: nil
            subtype: 0i16
            data1: 0i64
            data2: 0i64
        ];

        // Post the event to the app's event queue to wake it up
        let _: () = msg_send![app, postEvent: event atStart: false];

        log::debug!("Posted synthetic event to wake app event loop");
    }
}

#[cfg(not(target_os = "macos"))]
pub fn wake_app_event_loop() {
    // On other platforms, we don't have a good way to wake the event loop yet
    log::debug!("wake_app_event_loop not implemented for this platform");
}
