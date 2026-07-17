use gpui::*;

use super::theme::ribbon_theme_or_default;

/// The tab bar at the top of the ribbon, showing tab labels.
#[derive(IntoElement)]
pub struct RibbonTabBar {
    /// (original_index, label, is_contextual, contextual_color)
    tabs: Vec<(usize, SharedString, bool, Option<SharedString>)>,
    active_index: usize,
    on_tab_click: Option<Box<dyn Fn(usize, &mut Window, &mut App) + 'static>>,
    on_collapse_toggle: Option<Box<dyn Fn(&mut Window, &mut App) + 'static>>,
}

impl RibbonTabBar {
    pub fn new(
        tabs: Vec<(usize, SharedString, bool, Option<SharedString>)>,
        active_index: usize,
    ) -> Self {
        Self {
            tabs,
            active_index,
            on_tab_click: None,
            on_collapse_toggle: None,
        }
    }

    pub fn on_tab_click(
        mut self,
        handler: impl Fn(usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_tab_click = Some(Box::new(handler));
        self
    }

    pub fn on_collapse_toggle(
        mut self,
        handler: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_collapse_toggle = Some(Box::new(handler));
        self
    }
}

impl RenderOnce for RibbonTabBar {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = ribbon_theme_or_default(cx);

        let mut bar = div()
            .flex()
            .flex_row()
            .items_end()
            .h(px(28.0))
            .bg(theme.tab_bar_background)
            .border_b_1()
            .border_color(theme.border)
            .px(px(4.0))
            .gap(px(1.0));

        for (original_index, label, is_contextual, contextual_color) in &self.tabs {
            let is_active = *original_index == self.active_index;
            let idx = *original_index;
            let _on_click = self.on_tab_click.as_ref().map(|_f| {
                idx
            });

            let text_color = if is_active {
                theme.tab_active_text
            } else {
                theme.tab_inactive_text
            };

            let mut tab = div()
                .id(ElementId::Name(SharedString::from(format!("tab_{}", idx))))
                .flex()
                .items_center()
                .justify_center()
                .px(px(12.0))
                .py(px(4.0))
                .cursor_pointer()
                .rounded_t(px(3.0))
                .hover(|s| s.bg(theme.button_hover));

            if is_active {
                tab = tab
                    .bg(theme.panel_background)
                    .border_b_2()
                    .border_color(if *is_contextual {
                        contextual_color
                            .as_ref()
                            .map(|c| parse_hex_color(c))
                            .unwrap_or(theme.tab_active_indicator)
                    } else {
                        theme.tab_active_indicator
                    });
            }

            if *is_contextual {
                let accent = contextual_color
                    .as_ref()
                    .map(|c| parse_hex_color(c))
                    .unwrap_or(theme.contextual_tab_accent);
                tab = tab.border_t_2().border_color(accent);
            }

            tab = tab.child(
                div()
                    .text_size(px(11.0))
                    .text_color(text_color)
                    .child(label.clone()),
            );

            bar = bar.child(tab);
        }

        // Collapse toggle at far right
        bar = bar.child(div().flex_1()).child(
            div()
                .id("collapse_toggle")
                .flex()
                .items_center()
                .justify_center()
                .px(px(6.0))
                .py(px(4.0))
                .cursor_pointer()
                .hover(|s| s.bg(theme.button_hover))
                .rounded(px(2.0))
                .child(
                    div()
                        .text_size(px(10.0))
                        .text_color(theme.tab_inactive_text)
                        .child("▲"),
                ),
        );

        bar
    }
}

fn parse_hex_color(hex: &str) -> Hsla {
    let hex = hex.trim_start_matches('#');
    if let Ok(val) = u32::from_str_radix(hex, 16) {
        gpui::rgb(val).into()
    } else {
        gpui::rgb(0x007acc).into()
    }
}
