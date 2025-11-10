//! Theme and styling management for Terraphim AI
//!
//! This module provides theme management capabilities including
//! role-based color schemes and visual customization.

use eframe::egui;
use tracing::{debug, info};

use crate::state::{AppState, Theme, UIState};

/// Manages application themes and styling
pub struct ThemeManager {
    /// Current theme
    theme: Theme,

    /// Theme colors
    colors: ThemeColors,

    /// Typography settings
    typography: ThemeTypography,
}

/// Theme color palette
#[derive(Debug, Clone)]
pub struct ThemeColors {
    /// Primary/accent color
    pub accent: egui::Color32,

    /// Secondary color
    pub secondary: egui::Color32,

    /// Background color
    pub background: egui::Color32,

    /// Panel background
    pub panel: egui::Color32,

    /// Text color
    pub text: egui::Color32,

    /// Muted text color
    pub text_muted: egui::Color32,

    /// Border color
    pub border: egui::Color32,

    /// Success color
    pub success: egui::Color32,

    /// Warning color
    pub warning: egui::Color32,

    /// Error color
    pub error: egui::Color32,
}

/// Typography settings
#[derive(Debug, Clone)]
pub struct ThemeTypography {
    /// Heading font size
    pub heading_size: f32,

    /// Body font size
    pub body_size: f32,

    /// Small text font size
    pub small_size: f32,

    /// Code font size
    pub code_size: f32,
}

impl ThemeManager {
    /// Create a new ThemeManager
    pub fn new(state: &AppState) -> Self {
        let ui_state = state.get_ui_state();
        let theme = ui_state.theme.clone();
        drop(ui_state);

        let colors = Self::get_colors_for_theme(&theme);
        let typography = Self::get_typography_for_theme(&theme);

        info!("Initialized theme manager with theme: {:?}", theme);

        Self {
            theme,
            colors,
            typography,
        }
    }

    /// Get colors for a specific theme
    fn get_colors_for_theme(theme: &Theme) -> ThemeColors {
        match theme {
            Theme::Light => ThemeColors {
                accent: egui::Color32::from_rgb(66, 135, 245),   // Blue
                secondary: egui::Color32::from_rgb(45, 150, 90), // Green
                background: egui::Color32::from_rgb(250, 250, 250),
                panel: egui::Color32::from_rgb(255, 255, 255),
                text: egui::Color32::from_rgb(30, 30, 30),
                text_muted: egui::Color32::from_rgb(120, 120, 120),
                border: egui::Color32::from_rgb(220, 220, 220),
                success: egui::Color32::from_rgb(45, 150, 90),
                warning: egui::Color32::from_rgb(255, 193, 7),
                error: egui::Color32::from_rgb(220, 53, 69),
            },
            Theme::Dark => ThemeColors {
                accent: egui::Color32::from_rgb(66, 135, 245),   // Blue
                secondary: egui::Color32::from_rgb(45, 150, 90), // Green
                background: egui::Color32::from_rgb(18, 18, 18),
                panel: egui::Color32::from_rgb(25, 25, 25),
                text: egui::Color32::from_rgb(235, 235, 235),
                text_muted: egui::Color32::from_rgb(150, 150, 150),
                border: egui::Color32::from_rgb(50, 50, 50),
                success: egui::Color32::from_rgb(45, 150, 90),
                warning: egui::Color32::from_rgb(255, 193, 7),
                error: egui::Color32::from_rgb(220, 53, 69),
            },
            Theme::Nord => ThemeColors {
                // Nord theme colors (https://www.nordtheme.com/)
                accent: egui::Color32::from_rgb(163, 190, 140), // Green
                secondary: egui::Color32::from_rgb(180, 142, 173), // Purple
                background: egui::Color32::from_rgb(46, 52, 64), // Dark gray
                panel: egui::Color32::from_rgb(59, 66, 82),     // Medium gray
                text: egui::Color32::from_rgb(216, 222, 233),   // Light gray
                text_muted: egui::Color32::from_rgb(143, 153, 174), // Muted
                border: egui::Color32::from_rgb(67, 76, 94),    // Borders
                success: egui::Color32::from_rgb(163, 190, 140),
                warning: egui::Color32::from_rgb(235, 203, 139), // Yellow
                error: egui::Color32::from_rgb(191, 97, 106),    // Red
            },
            Theme::Custom => ThemeColors {
                // Default to dark theme for custom
                accent: egui::Color32::from_rgb(163, 190, 140),
                secondary: egui::Color32::from_rgb(180, 142, 173),
                background: egui::Color32::from_rgb(46, 52, 64),
                panel: egui::Color32::from_rgb(59, 66, 82),
                text: egui::Color32::from_rgb(216, 222, 233),
                text_muted: egui::Color32::from_rgb(143, 153, 174),
                border: egui::Color32::from_rgb(67, 76, 94),
                success: egui::Color32::from_rgb(163, 190, 140),
                warning: egui::Color32::from_rgb(235, 203, 139),
                error: egui::Color32::from_rgb(191, 97, 106),
            },
        }
    }

    /// Get colors for a specific role
    pub fn get_colors_for_role(&self, role_name: &str) -> ThemeColors {
        // Role-specific color schemes
        match role_name {
            "Rust Engineer" | "rust_engineer" => {
                // Orange/red accents for Rust engineer
                match self.theme {
                    Theme::Dark => ThemeColors {
                        accent: egui::Color32::from_rgb(220, 100, 60), // Orange-red
                        secondary: egui::Color32::from_rgb(203, 75, 22), // Dark orange
                        background: egui::Color32::from_rgb(14, 16, 19), // Very dark
                        panel: egui::Color32::from_rgb(20, 22, 25),    // Dark panel
                        text: egui::Color32::from_rgb(235, 235, 235),
                        text_muted: egui::Color32::from_rgb(150, 150, 150),
                        border: egui::Color32::from_rgb(50, 40, 30),
                        success: egui::Color32::from_rgb(45, 150, 90),
                        warning: egui::Color32::from_rgb(255, 193, 7),
                        error: egui::Color32::from_rgb(220, 60, 60),
                    },
                    _ => ThemeColors {
                        accent: egui::Color32::from_rgb(220, 100, 60), // Orange-red
                        secondary: egui::Color32::from_rgb(255, 140, 66), // Lighter orange
                        background: egui::Color32::from_rgb(250, 248, 247),
                        panel: egui::Color32::from_rgb(255, 255, 255),
                        text: egui::Color32::from_rgb(40, 40, 40),
                        text_muted: egui::Color32::from_rgb(120, 120, 120),
                        border: egui::Color32::from_rgb(220, 180, 160),
                        success: egui::Color32::from_rgb(45, 150, 90),
                        warning: egui::Color32::from_rgb(255, 193, 7),
                        error: egui::Color32::from_rgb(220, 60, 60),
                    },
                }
            }
            "Terraphim Engineer" | "terraphim_engineer" => {
                // Purple/green accents for Terraphim engineer
                match self.theme {
                    Theme::Dark => ThemeColors {
                        accent: egui::Color32::from_rgb(148, 163, 184), // Slate
                        secondary: egui::Color32::from_rgb(94, 234, 212), // Teal
                        background: egui::Color32::from_rgb(15, 23, 42), // Dark slate
                        panel: egui::Color32::from_rgb(30, 41, 59),     // Panel slate
                        text: egui::Color32::from_rgb(248, 250, 252),
                        text_muted: egui::Color32::from_rgb(148, 163, 184),
                        border: egui::Color32::from_rgb(51, 65, 85),
                        success: egui::Color32::from_rgb(94, 234, 212),
                        warning: egui::Color32::from_rgb(251, 191, 36), // Amber
                        error: egui::Color32::from_rgb(244, 63, 94),    // Rose
                    },
                    _ => ThemeColors {
                        accent: egui::Color32::from_rgb(139, 92, 246), // Purple
                        secondary: egui::Color32::from_rgb(45, 212, 191), // Teal
                        background: egui::Color32::from_rgb(248, 250, 252),
                        panel: egui::Color32::from_rgb(255, 255, 255),
                        text: egui::Color32::from_rgb(15, 23, 42),
                        text_muted: egui::Color32::from_rgb(100, 116, 139),
                        border: egui::Color32::from_rgb(203, 213, 225),
                        success: egui::Color32::from_rgb(45, 212, 191),
                        warning: egui::Color32::from_rgb(251, 191, 36),
                        error: egui::Color32::from_rgb(244, 63, 94),
                    },
                }
            }
            _ => {
                // Default role: Blue gradient
                self.colors.clone()
            }
        }
    }

    /// Get typography settings for a specific theme
    fn get_typography_for_theme(theme: &Theme) -> ThemeTypography {
        match theme {
            Theme::Light | Theme::Dark | Theme::Nord | Theme::Custom => ThemeTypography {
                heading_size: 18.0,
                body_size: 14.0,
                small_size: 12.0,
                code_size: 13.0,
            },
        }
    }

    /// Apply theme to the given egui context
    pub fn apply(&self, ctx: &egui::Context) {
        // Update egui visuals
        let mut style = (*ctx.style()).clone();

        // Update colors in the style
        style.visuals = match self.theme {
            Theme::Light => egui::Visuals::light(),
            Theme::Dark | Theme::Nord | Theme::Custom => egui::Visuals::dark(),
        };

        // Customize specific color aspects
        style.visuals.widgets.active.bg_fill = self.colors.panel;
        style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgba_premultiplied(
            self.colors.panel.r(),
            self.colors.panel.g(),
            self.colors.panel.b(),
            180,
        );
        style.visuals.widgets.inactive.bg_fill = self.colors.panel;

        ctx.set_style(style);
    }

    /// Get current theme
    pub fn theme(&self) -> &Theme {
        &self.theme
    }

    /// Get accent color
    pub fn accent_color(&self) -> egui::Color32 {
        self.colors.accent
    }

    /// Get secondary color
    pub fn secondary_color(&self) -> egui::Color32 {
        self.colors.secondary
    }

    /// Get background color
    pub fn background_color(&self) -> egui::Color32 {
        self.colors.background
    }

    /// Get panel color
    pub fn panel_color(&self) -> egui::Color32 {
        self.colors.panel
    }

    /// Get text color
    pub fn text_color(&self) -> egui::Color32 {
        self.colors.text
    }

    /// Get muted text color
    pub fn text_muted_color(&self) -> egui::Color32 {
        self.colors.text_muted
    }

    /// Get border color
    pub fn border_color(&self) -> egui::Color32 {
        self.colors.border
    }

    /// Get success color
    pub fn success_color(&self) -> egui::Color32 {
        self.colors.success
    }

    /// Get warning color
    pub fn warning_color(&self) -> egui::Color32 {
        self.colors.warning
    }

    /// Get error color
    pub fn error_color(&self) -> egui::Color32 {
        self.colors.error
    }

    /// Set theme
    pub fn set_theme(&mut self, mut theme: Theme, state: &AppState) {
        debug!("Changing theme to: {:?}", theme);
        let theme_clone = theme.clone();
        self.theme = theme;
        self.colors = Self::get_colors_for_theme(&theme_clone);
        self.typography = Self::get_typography_for_theme(&theme_clone);

        // Update state
        state.update_ui_state(|ui_state| {
            ui_state.theme = theme_clone;
        });
    }

    /// Switch to next theme
    pub fn next_theme(&mut self, state: &AppState) {
        let new_theme = match self.theme {
            Theme::Light => Theme::Dark,
            Theme::Dark => Theme::Nord,
            Theme::Nord => Theme::Custom,
            Theme::Custom => Theme::Light,
        };

        self.set_theme(new_theme, state);
    }
}
