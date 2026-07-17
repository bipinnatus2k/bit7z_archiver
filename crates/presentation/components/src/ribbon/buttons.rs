use gpui::*;

use super::command::{Command, CommandContext, CommandRegistry};
use super::model::RibbonItemKind;
use super::theme::ribbon_theme_or_default;

/// A large ribbon button with icon above text.
#[derive(IntoElement)]
pub struct RibbonLargeButton {
    command: Command,
    enabled: bool,
    on_click: Option<Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>>,
}

impl RibbonLargeButton {
    pub fn new(command: Command, ctx: &CommandContext) -> Self {
        let enabled = command.is_enabled(ctx);
        Self {
            command,
            enabled,
            on_click: None,
        }
    }

    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }
}

impl RenderOnce for RibbonLargeButton {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = ribbon_theme_or_default(cx);
        let text_color = if self.enabled {
            theme.button_text
        } else {
            theme.button_disabled_text
        };

        let mut el = div()
            .id(ElementId::Name(self.command.id.clone()))
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .px(px(6.0))
            .py(px(4.0))
            .min_w(px(52.0))
            .h_full()
            .rounded(px(3.0))
            .cursor_pointer()
            .hover(|s| s.bg(theme.button_hover))
            .active(|s| s.bg(theme.button_pressed))
            // Icon placeholder (colored square)
            .child(
                div()
                    .size(px(28.0))
                    .rounded(px(4.0))
                    .bg(theme.icon_color.opacity(0.15))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .text_size(px(14.0))
                            .text_color(theme.icon_color)
                            .child(
                                self.command
                                    .title
                                    .chars()
                                    .next()
                                    .unwrap_or('?')
                                    .to_string(),
                            ),
                    ),
            )
            // Label
            .child(
                div()
                    .text_size(px(10.0))
                    .text_color(text_color)
                    .mt(px(3.0))
                    .max_w(px(60.0))
                    .overflow_hidden()
                    .child(self.command.title.clone()),
            );

        if let Some(on_click) = self.on_click {
            if self.enabled {
                el = el.on_click(on_click);
            }
        }

        el
    }
}

/// A small ribbon button with icon to the left of text.
#[derive(IntoElement)]
pub struct RibbonSmallButton {
    command: Command,
    enabled: bool,
    on_click: Option<Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>>,
}

impl RibbonSmallButton {
    pub fn new(command: Command, ctx: &CommandContext) -> Self {
        let enabled = command.is_enabled(ctx);
        Self {
            command,
            enabled,
            on_click: None,
        }
    }

    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }
}

impl RenderOnce for RibbonSmallButton {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = ribbon_theme_or_default(cx);
        let text_color = if self.enabled {
            theme.button_text
        } else {
            theme.button_disabled_text
        };

        let mut el = div()
            .id(ElementId::Name(self.command.id.clone()))
            .flex()
            .flex_row()
            .items_center()
            .gap(px(4.0))
            .px(px(6.0))
            .py(px(2.0))
            .rounded(px(2.0))
            .cursor_pointer()
            .hover(|s| s.bg(theme.button_hover))
            .active(|s| s.bg(theme.button_pressed))
            // Small icon placeholder
            .child(
                div()
                    .size(px(16.0))
                    .rounded(px(2.0))
                    .bg(theme.icon_color.opacity(0.15))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(theme.icon_color)
                            .child(
                                self.command
                                    .title
                                    .chars()
                                    .next()
                                    .unwrap_or('?')
                                    .to_string(),
                            ),
                    ),
            )
            // Label
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(text_color)
                    .child(self.command.title.clone()),
            );

        if let Some(on_click) = self.on_click {
            if self.enabled {
                el = el.on_click(on_click);
            }
        }

        el
    }
}

/// A column of 2-3 small buttons stacked vertically.
#[derive(IntoElement)]
pub struct RibbonButtonColumn {
    buttons: Vec<(Command, bool)>,
    registry: CommandRegistry,
    ctx: CommandContext,
    on_command: Option<Box<dyn Fn(SharedString) + 'static>>,
}

impl RibbonButtonColumn {
    pub fn new(
        command_ids: &[SharedString],
        registry: &CommandRegistry,
        ctx: &CommandContext,
    ) -> Self {
        let buttons = command_ids
            .iter()
            .filter_map(|id| {
                registry.get(id).map(|cmd| {
                    let enabled = cmd.is_enabled(ctx);
                    (cmd.clone(), enabled)
                })
            })
            .collect();

        Self {
            buttons,
            registry: registry.clone(),
            ctx: ctx.clone(),
            on_command: None,
        }
    }
}

impl RenderOnce for RibbonButtonColumn {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let _theme = ribbon_theme_or_default(cx);
        let ctx = self.ctx.clone();

        let mut col = div().flex().flex_col().gap(px(1.0)).h_full().justify_center();

        for (command, _enabled) in self.buttons {
            let cmd_ctx = ctx.clone();
            let cmd = command.clone();
            col = col.child(
                RibbonSmallButton::new(command, &cmd_ctx).on_click(move |_, _, _| {
                    cmd.execute(&cmd_ctx);
                }),
            );
        }

        col
    }
}

/// A split button: top portion is primary action, bottom is dropdown arrow.
#[derive(IntoElement)]
pub struct RibbonSplitButton {
    primary: Command,
    dropdown_commands: Vec<Command>,
    enabled: bool,
    show_dropdown: bool,
}

impl RibbonSplitButton {
    pub fn new(
        primary_id: &SharedString,
        dropdown_ids: &[SharedString],
        registry: &CommandRegistry,
        ctx: &CommandContext,
    ) -> Self {
        let primary = registry
            .get(primary_id)
            .cloned()
            .unwrap_or_else(|| Command::new("unknown", "???"));
        let dropdown_commands: Vec<Command> = dropdown_ids
            .iter()
            .filter_map(|id| registry.get(id).cloned())
            .collect();
        let enabled = primary.is_enabled(ctx);

        Self {
            primary,
            dropdown_commands,
            enabled,
            show_dropdown: false,
        }
    }
}

impl RenderOnce for RibbonSplitButton {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = ribbon_theme_or_default(cx);
        let text_color = if self.enabled {
            theme.button_text
        } else {
            theme.button_disabled_text
        };

        div()
            .id(ElementId::Name(
                SharedString::from(format!("split_{}", self.primary.id)),
            ))
            .flex()
            .flex_col()
            .items_center()
            .min_w(px(52.0))
            .h_full()
            .rounded(px(3.0))
            // Top: primary button area
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .px(px(6.0))
                    .py(px(2.0))
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.button_hover))
                    // Icon
                    .child(
                        div()
                            .size(px(28.0))
                            .rounded(px(4.0))
                            .bg(theme.icon_color.opacity(0.15))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                div()
                                    .text_size(px(14.0))
                                    .text_color(theme.icon_color)
                                    .child(
                                        self.primary
                                            .title
                                            .chars()
                                            .next()
                                            .unwrap_or('?')
                                            .to_string(),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(text_color)
                            .mt(px(1.0))
                            .child(self.primary.title.clone()),
                    ),
            )
            // Bottom: dropdown arrow
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .w_full()
                    .h(px(14.0))
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.button_hover))
                    .rounded_b(px(3.0))
                    .child(
                        div()
                            .text_size(px(8.0))
                            .text_color(text_color)
                            .child("▼"),
                    ),
            )
    }
}

/// Renders a ribbon item based on its kind.
pub fn render_ribbon_item(
    item: &RibbonItemKind,
    registry: &CommandRegistry,
    ctx: &CommandContext,
) -> AnyElement {
    match item {
        RibbonItemKind::LargeButton { command_id } => {
            if let Some(cmd) = registry.get(command_id) {
                let cmd_clone = cmd.clone();
                let ctx_clone = ctx.clone();
                RibbonLargeButton::new(cmd.clone(), ctx)
                    .on_click(move |_, _, _| {
                        cmd_clone.execute(&ctx_clone);
                    })
                    .into_any_element()
            } else {
                div().into_any_element()
            }
        }
        RibbonItemKind::SmallButton { command_id } => {
            if let Some(cmd) = registry.get(command_id) {
                let cmd_clone = cmd.clone();
                let ctx_clone = ctx.clone();
                RibbonSmallButton::new(cmd.clone(), ctx)
                    .on_click(move |_, _, _| {
                        cmd_clone.execute(&ctx_clone);
                    })
                    .into_any_element()
            } else {
                div().into_any_element()
            }
        }
        RibbonItemKind::ButtonColumn { items } => {
            RibbonButtonColumn::new(items, registry, ctx).into_any_element()
        }
        RibbonItemKind::SplitButton {
            primary_command_id,
            dropdown_items,
        } => {
            RibbonSplitButton::new(primary_command_id, dropdown_items, registry, ctx)
                .into_any_element()
        }
        RibbonItemKind::DropdownGallery { label, items, columns } => {
            // Render as a large button with gallery indicator for now
            div()
                .id(ElementId::Name(label.clone()))
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .px(px(6.0))
                .py(px(4.0))
                .min_w(px(52.0))
                .h_full()
                .rounded(px(3.0))
                .cursor_pointer()
                .child(
                    div()
                        .size(px(28.0))
                        .rounded(px(4.0))
                        .bg(Hsla::from(gpui::rgb(0x555555)))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(div().text_size(px(10.0)).text_color(gpui::white()).child("⊞")),
                )
                .child(
                    div()
                        .text_size(px(10.0))
                        .text_color(Hsla::from(gpui::rgb(0xd4d4d4)))
                        .mt(px(3.0))
                        .child(label.clone()),
                )
                .into_any_element()
        }
        RibbonItemKind::Separator => div()
            .w(px(1.0))
            .h_full()
            .mx(px(2.0))
            .bg(Hsla::from(gpui::rgb(0x4a4a4d)))
            .into_any_element(),
    }
}
