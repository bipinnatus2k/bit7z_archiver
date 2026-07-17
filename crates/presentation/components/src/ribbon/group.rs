use gpui::*;

use super::buttons::render_ribbon_item;
use super::command::{CommandContext, CommandRegistry};
use super::model::RibbonGroup;
use super::theme::ribbon_theme_or_default;

/// Renders a single ribbon group with items, a right-side separator, and a bottom label.
#[derive(IntoElement)]
pub struct RibbonGroupView {
    group: RibbonGroup,
    registry: CommandRegistry,
    ctx: CommandContext,
    is_last: bool,
}

impl RibbonGroupView {
    pub fn new(
        group: RibbonGroup,
        registry: CommandRegistry,
        ctx: CommandContext,
        is_last: bool,
    ) -> Self {
        Self {
            group,
            registry,
            ctx,
            is_last,
        }
    }
}

impl RenderOnce for RibbonGroupView {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = ribbon_theme_or_default(cx);

        let mut items_row = div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(2.0))
            .flex_1()
            .h_full();

        for item in &self.group.items {
            items_row = items_row.child(render_ribbon_item(item, &self.registry, &self.ctx));
        }

        let group_el = div()
            .flex()
            .flex_col()
            .h_full()
            .child(
                // Items area
                div()
                    .flex()
                    .flex_row()
                    .flex_1()
                    .px(px(4.0))
                    .pt(px(2.0))
                    .child(items_row),
            )
            .child(
                // Bottom label
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .h(px(16.0))
                    .child(
                        div()
                            .text_size(px(9.0))
                            .text_color(theme.group_label_text)
                            .child(self.group.label.clone()),
                    ),
            );

        if !self.is_last {
            // Wrap with right separator
            div()
                .flex()
                .flex_row()
                .h_full()
                .child(group_el)
                .child(
                    div()
                        .w(px(1.0))
                        .h_full()
                        .my(px(4.0))
                        .bg(theme.group_separator),
                )
        } else {
            div().flex().flex_row().h_full().child(group_el)
        }
    }
}
