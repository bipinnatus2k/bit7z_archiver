use gpui::*;

use super::command::{CommandContext, CommandRegistry};
use super::theme::ribbon_theme_or_default;

/// Quick Access Toolbar — small toolbar above the ribbon with frequently used buttons.
#[derive(IntoElement)]
pub struct QuickAccessToolbar {
    command_ids: Vec<SharedString>,
    registry: CommandRegistry,
    ctx: CommandContext,
    title: SharedString,
}

impl QuickAccessToolbar {
    pub fn new(
        command_ids: Vec<SharedString>,
        registry: CommandRegistry,
        ctx: CommandContext,
        title: impl Into<SharedString>,
    ) -> Self {
        Self {
            command_ids,
            registry,
            ctx,
            title: title.into(),
        }
    }
}

impl RenderOnce for QuickAccessToolbar {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = ribbon_theme_or_default(cx);

        let mut toolbar = div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(1.0));

        for cmd_id in &self.command_ids {
            if let Some(cmd) = self.registry.get(cmd_id) {
                let enabled = cmd.is_enabled(&self.ctx);
                let cmd_clone = cmd.clone();
                let ctx_clone = self.ctx.clone();
                let text_color = if enabled {
                    theme.icon_color
                } else {
                    theme.button_disabled_text
                };

                let mut btn = div()
                    .id(ElementId::Name(SharedString::from(format!("qat_{}", cmd.id))))
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(px(20.0))
                    .rounded(px(2.0))
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.button_hover))
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(text_color)
                            .child(
                                cmd.title
                                    .chars()
                                    .next()
                                    .unwrap_or('?')
                                    .to_string(),
                            ),
                    );

                if enabled {
                    btn = btn.on_click(move |_, _, _| {
                        cmd_clone.execute(&ctx_clone);
                    });
                }

                toolbar = toolbar.child(btn);
            }
        }

        div()
            .flex()
            .flex_row()
            .items_center()
            .h(px(24.0))
            .bg(theme.qat_background)
            .px(px(8.0))
            .child(toolbar)
            .child(div().flex_1())
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(theme.title_text)
                    .child(self.title),
            )
            .child(div().flex_1())
    }
}
