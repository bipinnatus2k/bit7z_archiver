//! Application color theme, exposed as a GPUI global.

use crate::domain::preferences::ThemeMode;
use gpui::{rgb, Global, Hsla, Window};

/// Theme colors used throughout the application.
#[derive(Clone, Debug)]
pub struct Theme {
    pub primary: Hsla,
    pub selection: Hsla,
    pub hover: Hsla,
    pub border: Hsla,
    pub muted: Hsla,
    pub error: Hsla,
    pub surface: Hsla,
    pub foreground: Hsla,
    pub background: Hsla,
    pub input_bg: Hsla,
    pub input_border: Hsla,
    pub card_bg: Hsla,
    pub card_border: Hsla,
}

impl Theme {
    pub fn light() -> Self {
        Self {
            primary: rgb(0x2a63d9).into(),
            selection: rgb(0x2a63d9).into(),
            hover: rgb(0xe6e6e6).into(),
            border: rgb(0xd9d9d9).into(),
            muted: rgb(0x8c8c8c).into(),
            error: rgb(0xcc3333).into(),
            surface: rgb(0xf4f5f5).into(),
            foreground: rgb(0x252525).into(),
            background: rgb(0xffffff).into(),
            input_bg: rgb(0xffffff).into(),
            input_border: rgb(0xd9d9d9).into(),
            card_bg: rgb(0xffffff).into(),
            card_border: rgb(0xe8e8e8).into(),
        }
    }

    pub fn dark() -> Self {
        Self {
            primary: rgb(0x4d8cff).into(),
            selection: rgb(0x1e3a5f).into(),
            hover: rgb(0x2a2a2a).into(),
            border: rgb(0x3a3a3a).into(),
            muted: rgb(0x888888).into(),
            error: rgb(0xff6b6b).into(),
            surface: rgb(0x1e1e1e).into(),
            foreground: rgb(0xe0e0e0).into(),
            background: rgb(0x181818).into(),
            input_bg: rgb(0x252525).into(),
            input_border: rgb(0x3a3a3a).into(),
            card_bg: rgb(0x202020).into(),
            card_border: rgb(0x303030).into(),
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