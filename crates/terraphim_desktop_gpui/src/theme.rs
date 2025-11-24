use gpui::*;

/// Terraphim theme configuration
pub struct TerraphimTheme {
    pub mode: ThemeMode,
    pub colors: ThemeColors,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ThemeMode {
    Light,
    Dark,
}

#[derive(Clone, Debug)]
pub struct ThemeColors {
    // Background colors
    pub background: Hsla,
    pub surface: Hsla,
    pub surface_hover: Hsla,

    // Text colors
    pub text_primary: Hsla,
    pub text_secondary: Hsla,
    pub text_disabled: Hsla,

    // Primary colors (inspired by Bulma's blue)
    pub primary: Hsla,
    pub primary_hover: Hsla,
    pub primary_text: Hsla,

    // Semantic colors
    pub success: Hsla,
    pub warning: Hsla,
    pub danger: Hsla,
    pub info: Hsla,

    // Border colors
    pub border: Hsla,
    pub border_light: Hsla,
}

impl TerraphimTheme {
    pub fn new(_cx: &mut ModelContext<Self>) -> Self {
        Self {
            mode: ThemeMode::Light,
            colors: Self::light_colors(),
        }
    }

    pub fn toggle_mode(&mut self, cx: &mut ModelContext<Self>) {
        self.mode = match self.mode {
            ThemeMode::Light => ThemeMode::Dark,
            ThemeMode::Dark => ThemeMode::Light,
        };

        self.colors = match self.mode {
            ThemeMode::Light => Self::light_colors(),
            ThemeMode::Dark => Self::dark_colors(),
        };

        log::info!("Theme toggled to {:?}", self.mode);
        cx.notify();
    }

    fn light_colors() -> ThemeColors {
        ThemeColors {
            background: rgb(0xffffff),
            surface: rgb(0xf5f5f5),
            surface_hover: rgb(0xf0f0f0),

            text_primary: rgb(0x363636),
            text_secondary: rgb(0x7a7a7a),
            text_disabled: rgb(0xb5b5b5),

            primary: rgb(0x3273dc),
            primary_hover: rgb(0x2366d1),
            primary_text: rgb(0xffffff),

            success: rgb(0x48c774),
            warning: rgb(0xffdd57),
            danger: rgb(0xf14668),
            info: rgb(0x3298dc),

            border: rgb(0xdbdbdb),
            border_light: rgb(0xededed),
        }
    }

    fn dark_colors() -> ThemeColors {
        ThemeColors {
            background: rgb(0x1a1a1a),
            surface: rgb(0x2a2a2a),
            surface_hover: rgb(0x363636),

            text_primary: rgb(0xf5f5f5),
            text_secondary: rgb(0xb5b5b5),
            text_disabled: rgb(0x7a7a7a),

            primary: rgb(0x3273dc),
            primary_hover: rgb(0x2366d1),
            primary_text: rgb(0xffffff),

            success: rgb(0x48c774),
            warning: rgb(0xffdd57),
            danger: rgb(0xf14668),
            info: rgb(0x3298dc),

            border: rgb(0x4a4a4a),
            border_light: rgb(0x363636),
        }
    }
}

pub fn configure_theme(_cx: &mut AppContext) {
    log::info!("Theme system configured");
    // Theme configuration will be applied per-window via TerraphimTheme model
}
