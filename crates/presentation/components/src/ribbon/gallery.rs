use gpui::*;

use super::model::GalleryItem;
use super::theme::ribbon_theme_or_default;

/// A gallery popup showing a grid of items.
#[derive(IntoElement)]
pub struct RibbonGallery {
    label: SharedString,
    items: Vec<GalleryItem>,
    columns: usize,
}

impl RibbonGallery {
    pub fn new(label: impl Into<SharedString>, items: Vec<GalleryItem>, columns: usize) -> Self {
        Self {
            label: label.into(),
            items,
            columns,
        }
    }
}

impl RenderOnce for RibbonGallery {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = ribbon_theme_or_default(cx);

        let mut grid = div()
            .flex()
            .flex_row()
            .flex_wrap()
            .gap(px(4.0))
            .p(px(8.0))
            .bg(theme.panel_background)
            .border_1()
            .border_color(theme.border)
            .rounded(px(4.0))
            .shadow_lg();

        for item in &self.items {
            grid = grid.child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .w(px(64.0))
                    .h(px(56.0))
                    .rounded(px(3.0))
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.button_hover))
                    .child(
                        div()
                            .size(px(32.0))
                            .rounded(px(4.0))
                            .bg(theme.icon_color.opacity(0.1))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(theme.icon_color)
                                    .child(
                                        item.label
                                            .chars()
                                            .next()
                                            .unwrap_or('?')
                                            .to_string(),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(9.0))
                            .text_color(theme.button_text)
                            .mt(px(2.0))
                            .child(item.label.clone()),
                    ),
            );
        }

        div()
            .flex()
            .flex_col()
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(theme.button_text)
                    .font_weight(FontWeight::BOLD)
                    .pb(px(4.0))
                    .child(self.label),
            )
            .child(grid)
    }
}
