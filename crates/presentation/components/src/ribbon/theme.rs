use gpui::{rgb, Hsla};

/// Theme colors for the ribbon UI.
#[derive(Clone, Debug)]
pub struct RibbonTheme {
    /// Background of the entire ribbon area.
    pub ribbon_background: Hsla,
    /// Background of the tab bar.
    pub tab_bar_background: Hsla,
    /// Color of the active tab label.
    pub tab_active_text: Hsla,
    /// Color of inactive tab labels.
    pub tab_inactive_text: Hsla,
    /// Underline / indicator for the active tab.
    pub tab_active_indicator: Hsla,
    /// Background of the ribbon panel (below tabs).
    pub panel_background: Hsla,
    /// Group separator line color.
    pub group_separator: Hsla,
    /// Group label text color.
    pub group_label_text: Hsla,
    /// Button text color.
    pub button_text: Hsla,
    /// Button hover background.
    pub button_hover: Hsla,
    /// Button pressed/active background.
    pub button_pressed: Hsla,
    /// Button disabled text color.
    pub button_disabled_text: Hsla,
    /// Quick access toolbar background.
    pub qat_background: Hsla,
    /// Contextual tab header accent (default, individual tabs override).
    pub contextual_tab_accent: Hsla,
    /// Search box background.
    pub search_background: Hsla,
    /// Search box border.
    pub search_border: Hsla,
    /// Search box text color.
    pub search_text: Hsla,
    /// Search box placeholder color.
    pub search_placeholder: Hsla,
    /// Border color around ribbon.
    pub border: Hsla,
    /// Title text in QAT area.
    pub title_text: Hsla,
    /// Icon default tint.
    pub icon_color: Hsla,
    /// Window title bar background color.
    pub title_bar_background: Hsla,
}

impl RibbonTheme {
    /// A dark theme inspired by ANSYS / engineering software.
    pub fn dark() -> Self {
        Self {
            ribbon_background: rgb(0x2d2d30).into(),
            tab_bar_background: rgb(0x1e1e1e).into(),
            tab_active_text: rgb(0xffffff).into(),
            tab_inactive_text: rgb(0x969696).into(),
            tab_active_indicator: rgb(0x007acc).into(),
            panel_background: rgb(0x333337).into(),
            group_separator: rgb(0x4a4a4d).into(),
            group_label_text: rgb(0x808080).into(),
            button_text: rgb(0xd4d4d4).into(),
            button_hover: rgb(0x3e3e42).into(),
            button_pressed: rgb(0x094771).into(),
            button_disabled_text: rgb(0x5a5a5a).into(),
            qat_background: rgb(0x1e1e1e).into(),
            contextual_tab_accent: rgb(0x007acc).into(),
            search_background: rgb(0x3c3c3c).into(),
            search_border: rgb(0x4a4a4d).into(),
            search_text: rgb(0xd4d4d4).into(),
            search_placeholder: rgb(0x6a6a6a).into(),
            border: rgb(0x3f3f46).into(),
            title_text: rgb(0xcccccc).into(),
            icon_color: rgb(0xcccccc).into(),
            title_bar_background: rgb(0x1e1e1e).into(),
        }
    }

    /// A light theme.
    pub fn light() -> Self {
        Self {
            ribbon_background: rgb(0xf3f3f3).into(),
            tab_bar_background: rgb(0xe8e8e8).into(),
            tab_active_text: rgb(0x1e1e1e).into(),
            tab_inactive_text: rgb(0x6e6e6e).into(),
            tab_active_indicator: rgb(0x0078d4).into(),
            panel_background: rgb(0xfafafa).into(),
            group_separator: rgb(0xd0d0d0).into(),
            group_label_text: rgb(0x999999).into(),
            button_text: rgb(0x1e1e1e).into(),
            button_hover: rgb(0xe0e0e0).into(),
            button_pressed: rgb(0xc0deff).into(),
            button_disabled_text: rgb(0xb0b0b0).into(),
            qat_background: rgb(0xe8e8e8).into(),
            contextual_tab_accent: rgb(0x0078d4).into(),
            search_background: rgb(0xffffff).into(),
            search_border: rgb(0xc8c8c8).into(),
            search_text: rgb(0x1e1e1e).into(),
            search_placeholder: rgb(0xa0a0a0).into(),
            border: rgb(0xd0d0d0).into(),
            title_text: rgb(0x333333).into(),
            icon_color: rgb(0x424242).into(),
            title_bar_background: rgb(0xe8e8e8).into(),
        }
    }

    /// ANSYS-style blue theme.
    pub fn ansys_blue() -> Self {
        Self {
            ribbon_background: rgb(0x1b2838).into(),
            tab_bar_background: rgb(0x141e2b).into(),
            tab_active_text: rgb(0xffffff).into(),
            tab_inactive_text: rgb(0x8899aa).into(),
            tab_active_indicator: rgb(0x4fc3f7).into(),
            panel_background: rgb(0x1e2d3d).into(),
            group_separator: rgb(0x2a3f52).into(),
            group_label_text: rgb(0x6688aa).into(),
            button_text: rgb(0xc8d8e8).into(),
            button_hover: rgb(0x264060).into(),
            button_pressed: rgb(0x0d5d8c).into(),
            button_disabled_text: rgb(0x4a5a6a).into(),
            qat_background: rgb(0x141e2b).into(),
            contextual_tab_accent: rgb(0x4fc3f7).into(),
            search_background: rgb(0x1a2a3a).into(),
            search_border: rgb(0x2a3f52).into(),
            search_text: rgb(0xc8d8e8).into(),
            search_placeholder: rgb(0x5a7a9a).into(),
            border: rgb(0x2a3f52).into(),
            title_text: rgb(0xb0c4de).into(),
            icon_color: rgb(0xb0c4de).into(),
            title_bar_background: rgb(0x141e2b).into(),
        }
    }
}

impl Default for RibbonTheme {
    fn default() -> Self {
        Self::dark()
    }
}

/// Global wrapper so the theme can be stored in GPUI's global state.
pub struct GlobalRibbonTheme(pub RibbonTheme);

impl gpui::Global for GlobalRibbonTheme {}

/// Helper to read the current theme from context.
pub fn ribbon_theme(cx: &gpui::App) -> &RibbonTheme {
    &cx.global::<GlobalRibbonTheme>().0
}

/// Helper to get theme or default if not set.
pub fn ribbon_theme_or_default(cx: &gpui::App) -> RibbonTheme {
    cx.try_global::<GlobalRibbonTheme>()
        .map(|g| g.0.clone())
        .unwrap_or_default()
}

/// Set the ribbon theme globally.
pub fn set_ribbon_theme(cx: &mut gpui::App, theme: RibbonTheme) {
    cx.set_global(GlobalRibbonTheme(theme));
}
