/// Theme color constants matching TerraphimTheme
/// 
/// These constants provide easy access to theme colors throughout the app.
/// Eventually these should be replaced with dynamic theme lookups from TerraphimTheme entity.

use gpui::*;

/// Light theme colors (default)
pub mod light {
    use super::*;
    
    pub fn background() -> Hsla { rgb(0xffffff).into() }
    pub fn surface() -> Hsla { rgb(0xf5f5f5).into() }
    pub fn surface_hover() -> Hsla { rgb(0xf0f0f0).into() }
    
    pub fn text_primary() -> Hsla { rgb(0x363636).into() }
    pub fn text_secondary() -> Hsla { rgb(0x7a7a7a).into() }
    pub fn text_disabled() -> Hsla { rgb(0xb5b5b5).into() }
    
    pub fn primary() -> Hsla { rgb(0x3273dc).into() }
    pub fn primary_hover() -> Hsla { rgb(0x2366d1).into() }
    pub fn primary_text() -> Hsla { rgb(0xffffff).into() }
    
    pub fn success() -> Hsla { rgb(0x48c774).into() }
    pub fn warning() -> Hsla { rgb(0xffdd57).into() }
    pub fn danger() -> Hsla { rgb(0xf14668).into() }
    pub fn info() -> Hsla { rgb(0x3298dc).into() }
    
    pub fn border() -> Hsla { rgb(0xdbdbdb).into() }
    pub fn border_light() -> Hsla { rgb(0xededed).into() }
    
    // Additional semantic colors for UI elements
    pub fn autocomplete_selected() -> Hsla { rgb(0xe3f2fd).into() }  // Light blue for selected autocomplete items
}

/// Dark theme colors
pub mod dark {
    use super::*;
    
    pub fn background() -> Hsla { rgb(0x1a1a1a).into() }
    pub fn surface() -> Hsla { rgb(0x2a2a2a).into() }
    pub fn surface_hover() -> Hsla { rgb(0x363636).into() }
    
    pub fn text_primary() -> Hsla { rgb(0xf5f5f5).into() }
    pub fn text_secondary() -> Hsla { rgb(0xb5b5b5).into() }
    pub fn text_disabled() -> Hsla { rgb(0x7a7a7a).into() }
    
    pub fn primary() -> Hsla { rgb(0x3273dc).into() }
    pub fn primary_hover() -> Hsla { rgb(0x2366d1).into() }
    pub fn primary_text() -> Hsla { rgb(0xffffff).into() }
    
    pub fn success() -> Hsla { rgb(0x48c774).into() }
    pub fn warning() -> Hsla { rgb(0xffdd57).into() }
    pub fn danger() -> Hsla { rgb(0xf14668).into() }
    pub fn info() -> Hsla { rgb(0x3298dc).into() }
    
    pub fn border() -> Hsla { rgb(0x4a4a4a).into() }
    pub fn border_light() -> Hsla { rgb(0x363636).into() }
    
    pub fn autocomplete_selected() -> Hsla { rgb(0x2a3a4a).into() }
}

/// Default theme colors (currently light theme)
/// 
/// Use these functions throughout the app for consistent theming.
/// TODO: Replace with dynamic theme lookup from TerraphimTheme entity.
pub use light as theme;
