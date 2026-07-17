use gpui::*;

use super::command::{CommandContext, CommandRegistry};
use super::group::RibbonGroupView;
use super::model::RibbonTab;
use super::theme::ribbon_theme_or_default;

/// The content panel below the tab bar, showing groups for the active tab.
#[derive(IntoElement)]
pub struct RibbonPanel {
    tab: RibbonTab,
    registry: CommandRegistry,
    ctx: CommandContext,
}

impl RibbonPanel {
    pub fn new(tab: RibbonTab, registry: CommandRegistry, ctx: CommandContext) -> Self {
        Self { tab, registry, ctx }
    }
}

impl RenderOnce for RibbonPanel {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = ribbon_theme_or_default(cx);
        let group_count = self.tab.groups.len();

        let mut panel = div()
            .flex()
            .flex_row()
            .w_full()
            .h(px(86.0))
            .bg(theme.panel_background)
            .border_b_1()
            .border_color(theme.border)
            .overflow_x_hidden();

        for (i, group) in self.tab.groups.into_iter().enumerate() {
            let is_last = i == group_count - 1;
            panel = panel.child(RibbonGroupView::new(
                group,
                self.registry.clone(),
                self.ctx.clone(),
                is_last,
            ));
        }

        panel
    }
}
