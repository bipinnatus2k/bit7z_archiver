use gpui::*;

use super::command::{Command, CommandRegistry};
use super::theme::ribbon_theme_or_default;

/// Command search box embedded in the ribbon area.
/// Provides fuzzy search over all registered commands.
pub struct RibbonCommandSearch {
    query: String,
    results: Vec<Command>,
    is_focused: bool,
    registry: CommandRegistry,
    focus_handle: FocusHandle,
}

impl RibbonCommandSearch {
    pub fn new(registry: CommandRegistry, cx: &mut App) -> Self {
        Self {
            query: String::new(),
            results: Vec::new(),
            is_focused: false,
            registry,
            focus_handle: cx.focus_handle(),
        }
    }

    fn update_results(&mut self) {
        if self.query.is_empty() {
            self.results.clear();
        } else {
            self.results = self
                .registry
                .search(&self.query)
                .into_iter()
                .take(10)
                .cloned()
                .collect();
        }
    }
}

impl Render for RibbonCommandSearch {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = ribbon_theme_or_default(cx);

        let mut container = div()
            .flex()
            .flex_col()
            .relative();

        // Search input
        container = container.child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .h(px(22.0))
                .w(px(200.0))
                .bg(theme.search_background)
                .border_1()
                .border_color(theme.search_border)
                .rounded(px(3.0))
                .px(px(6.0))
                .child(
                    div()
                        .text_size(px(10.0))
                        .text_color(if self.query.is_empty() {
                            theme.search_placeholder
                        } else {
                            theme.search_text
                        })
                        .child(if self.query.is_empty() {
                            SharedString::from("Search commands...")
                        } else {
                            SharedString::from(self.query.clone())
                        }),
                ),
        );

        // Results dropdown (when focused and has results)
        if !self.results.is_empty() && self.is_focused {
            let mut results_panel = div()
                .absolute()
                .top(px(24.0))
                .left_0()
                .w(px(250.0))
                .bg(theme.panel_background)
                .border_1()
                .border_color(theme.border)
                .rounded(px(3.0))
                .shadow_lg()
                .py(px(4.0));

            for cmd in &self.results {
                results_panel = results_panel.child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(8.0))
                        .px(px(8.0))
                        .py(px(4.0))
                        .hover(|s| s.bg(theme.button_hover))
                        .cursor_pointer()
                        .child(
                            div()
                                .text_size(px(11.0))
                                .text_color(theme.button_text)
                                .child(cmd.title.clone()),
                        )
                        .child(
                            div()
                                .text_size(px(9.0))
                                .text_color(theme.group_label_text)
                                .child(
                                    cmd.shortcut
                                        .clone()
                                        .unwrap_or_else(|| SharedString::from("")),
                                ),
                        ),
                );
            }

            container = container.child(results_panel);
        }

        container
    }
}
