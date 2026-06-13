//! Application color theme, exposed as a GPUI global.

use gpui::{rgb, Global, Hsla};

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
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            primary: rgb(0x2a63d9).into(),
            selection: rgb(0x2a63d9).into(),
            border: rgb(0xd9d9d9).into(),
            muted: rgb(0xb0b0b0).into(),
            hover: rgb(0xe6e6e6).into(),
            surface: rgb(0xf4f5f5).into(),
            foreground: rgb(0x252525).into(),
            error: rgb(0xcc3333).into(),
        }
    }
}

impl Global for Theme {}

