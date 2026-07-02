//! Application color theme, exposed as a GPUI global.

use crate::domain::preferences::ThemeMode;
use gpui::{rgb, Global, Hsla, Window};

/// Theme colors used throughout the application.
#[derive(Clone, Debug)]
pub struct Theme {
    // Primary colors
    pub primary: Hsla,
    pub primary_hover: Hsla,
    pub primary_active: Hsla,
    pub primary_foreground: Hsla,
    
    // Secondary colors
    pub secondary: Hsla,
    pub secondary_hover: Hsla,
    pub secondary_foreground: Hsla,
    
    // Status colors
    pub success: Hsla,
    pub success_foreground: Hsla,
    pub warning: Hsla,
    pub warning_foreground: Hsla,
    pub info: Hsla,
    pub info_foreground: Hsla,
    pub error: Hsla,
    pub error_hover: Hsla,
    pub error_foreground: Hsla,
    
    // Neutral colors
    pub background: Hsla,
    pub foreground: Hsla,
    pub muted: Hsla,
    pub muted_foreground: Hsla,
    pub surface: Hsla,
    pub surface_hover: Hsla,
    
    // Border colors
    pub border: Hsla,
    pub border_focus: Hsla,
    
    // Input colors
    pub input_bg: Hsla,
    pub input_border: Hsla,
    pub input_border_focus: Hsla,
    
    // Card colors
    pub card_bg: Hsla,
    pub card_border: Hsla,
    
    // Interaction states
    pub selection: Hsla,
    pub hover: Hsla,
    pub disabled: Hsla,
    pub disabled_foreground: Hsla,
    pub focus_ring: Hsla,
    
    // Design tokens
    pub radius_sm: f32,
    pub radius_md: f32,
    pub radius_lg: f32,
}

impl Theme {
    pub fn light() -> Self {
        Self {
            // Primary
            primary: rgb(0x2a63d9).into(),
            primary_hover: rgb(0x1e50b8).into(),
            primary_active: rgb(0x1a45a0).into(),
            primary_foreground: rgb(0xffffff).into(),
            
            // Secondary
            secondary: rgb(0xf0f0f0).into(),
            secondary_hover: rgb(0xe0e0e0).into(),
            secondary_foreground: rgb(0x252525).into(),
            
            // Status
            success: rgb(0x22c55e).into(),
            success_foreground: rgb(0xffffff).into(),
            warning: rgb(0xf59e0b).into(),
            warning_foreground: rgb(0xffffff).into(),
            info: rgb(0x3b82f6).into(),
            info_foreground: rgb(0xffffff).into(),
            error: rgb(0xcc3333).into(),
            error_hover: rgb(0xb02d2d).into(),
            error_foreground: rgb(0xffffff).into(),
            
            // Neutral
            background: rgb(0xffffff).into(),
            foreground: rgb(0x252525).into(),
            muted: rgb(0x8c8c8c).into(),
            muted_foreground: rgb(0x6b6b6b).into(),
            surface: rgb(0xf4f5f5).into(),
            surface_hover: rgb(0xebebeb).into(),
            
            // Border
            border: rgb(0xd9d9d9).into(),
            border_focus: rgb(0x2a63d9).into(),
            
            // Input
            input_bg: rgb(0xffffff).into(),
            input_border: rgb(0xd9d9d9).into(),
            input_border_focus: rgb(0x2a63d9).into(),
            
            // Card
            card_bg: rgb(0xffffff).into(),
            card_border: rgb(0xe8e8e8).into(),
            
            // Interaction
            selection: rgb(0x2a63d9).into(),
            hover: rgb(0xe6e6e6).into(),
            disabled: rgb(0xd9d9d9).into(),
            disabled_foreground: rgb(0x8c8c8c).into(),
            focus_ring: rgb(0x2a63d9).into(),
            
            // Design tokens
            radius_sm: 4.0,
            radius_md: 6.0,
            radius_lg: 8.0,
        }
    }

    pub fn dark() -> Self {
        Self {
            // Primary
            primary: rgb(0x4d8cff).into(),
            primary_hover: rgb(0x3d7cef).into(),
            primary_active: rgb(0x2d6cdf).into(),
            primary_foreground: rgb(0xffffff).into(),
            
            // Secondary
            secondary: rgb(0x2a2a2a).into(),
            secondary_hover: rgb(0x353535).into(),
            secondary_foreground: rgb(0xe0e0e0).into(),
            
            // Status
            success: rgb(0x4ade80).into(),
            success_foreground: rgb(0x000000).into(),
            warning: rgb(0xfbbf24).into(),
            warning_foreground: rgb(0x000000).into(),
            info: rgb(0x60a5fa).into(),
            info_foreground: rgb(0x000000).into(),
            error: rgb(0xff6b6b).into(),
            error_hover: rgb(0xff5252).into(),
            error_foreground: rgb(0xffffff).into(),
            
            // Neutral
            background: rgb(0x181818).into(),
            foreground: rgb(0xe0e0e0).into(),
            muted: rgb(0x888888).into(),
            muted_foreground: rgb(0xa0a0a0).into(),
            surface: rgb(0x1e1e1e).into(),
            surface_hover: rgb(0x2a2a2a).into(),
            
            // Border
            border: rgb(0x3a3a3a).into(),
            border_focus: rgb(0x4d8cff).into(),
            
            // Input
            input_bg: rgb(0x252525).into(),
            input_border: rgb(0x3a3a3a).into(),
            input_border_focus: rgb(0x4d8cff).into(),
            
            // Card
            card_bg: rgb(0x202020).into(),
            card_border: rgb(0x303030).into(),
            
            // Interaction
            selection: rgb(0x1e3a5f).into(),
            hover: rgb(0x2a2a2a).into(),
            disabled: rgb(0x3a3a3a).into(),
            disabled_foreground: rgb(0x666666).into(),
            focus_ring: rgb(0x4d8cff).into(),
            
            // Design tokens
            radius_sm: 4.0,
            radius_md: 6.0,
            radius_lg: 8.0,
        }
    }

    pub fn from_mode(mode: ThemeMode, window: &Window) -> Self {
        match mode {
            ThemeMode::Light => Self::light(),
            ThemeMode::Dark => Self::dark(),
            ThemeMode::System => {
                if window.appearance() == gpui::WindowAppearance::Dark {
                    Self::dark()
                } else {
                    Self::light()
                }
            }
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::light()
    }
}

impl Global for Theme {}