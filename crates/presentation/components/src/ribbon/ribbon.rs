use std::collections::HashMap;

use gpui::*;

use super::command::{CommandContext, CommandRegistry, ContextPredicate};
use super::model::{RibbonDisplayMode, RibbonModel};
use super::panel::RibbonPanel;
use super::quick_access::QuickAccessToolbar;
use super::tab_bar::RibbonTabBar;
use super::theme::ribbon_theme_or_default;

/// The top-level Ribbon view that composes tab bar, panel, QAT, and search.
pub struct Ribbon {
    model: RibbonModel,
    registry: CommandRegistry,
    context: CommandContext,
    context_predicates: HashMap<SharedString, ContextPredicate>,
    title: SharedString,
}

impl Ribbon {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            model: RibbonModel::new(),
            registry: CommandRegistry::new(),
            context: CommandContext::default(),
            context_predicates: HashMap::new(),
            title: title.into(),
        }
    }

    pub fn set_model(mut self, model: RibbonModel) -> Self {
        self.model = model;
        self
    }

    pub fn set_registry(mut self, command_registry: CommandRegistry) -> Self {
        self.registry = command_registry;
        self
    }

    /// Register a predicate for a contextual tab.
    pub fn register_context_predicate(
        &mut self,
        tab_id: impl Into<SharedString>,
        predicate: impl Fn(&CommandContext) -> bool + Send + Sync + 'static,
    ) {
        self.context_predicates
            .insert(tab_id.into(), std::sync::Arc::new(predicate));
    }

    /// Set the current command context (triggers re-evaluation of contextual tabs).
    pub fn set_context(&mut self, ctx: CommandContext) {
        self.context = ctx;
    }

    /// Switch to a specific tab by original index.
    pub fn set_active_tab(&mut self, index: usize) {
        if index < self.model.tabs.len() {
            self.model.active_tab_index = index;
        }
    }

    /// Toggle display mode.
    pub fn toggle_display_mode(&mut self) {
        self.model.display_mode = match self.model.display_mode {
            RibbonDisplayMode::Expanded => RibbonDisplayMode::Compact,
            RibbonDisplayMode::Compact => RibbonDisplayMode::AutoHide,
            RibbonDisplayMode::AutoHide => RibbonDisplayMode::Expanded,
        };
    }
}

impl Render for Ribbon {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = ribbon_theme_or_default(cx);

        // Get visible tabs
        let visible_tabs = self
            .model
            .visible_tabs(&self.context, &self.context_predicates);

        // Build tab bar data
        let tab_data: Vec<(usize, SharedString, bool, Option<SharedString>)> = visible_tabs
            .iter()
            .map(|(idx, tab)| {
                (
                    *idx,
                    tab.label.clone(),
                    tab.is_contextual,
                    tab.contextual_color.clone(),
                )
            })
            .collect();

        let active_index = self.model.active_tab_index;

        // QAT
        let qat_ids: Vec<SharedString> = self
            .model
            .quick_access_items
            .iter()
            .map(|item| item.command_id.clone())
            .collect();

        let mut container = div()
            .flex()
            .flex_col()
            .w_full()
            .bg(theme.ribbon_background);

        // Quick Access Toolbar
        container = container.child(QuickAccessToolbar::new(
            qat_ids,
            self.registry.clone(),
            self.context.clone(),
            self.title.clone(),
        ));

        // Tab bar
        container = container.child(RibbonTabBar::new(tab_data, active_index));

        // Panel (only in Expanded mode)
        match self.model.display_mode {
            RibbonDisplayMode::Expanded => {
                if let Some(active_tab) = self.model.tabs.get(active_index).cloned() {
                    container = container.child(RibbonPanel::new(
                        active_tab,
                        self.registry.clone(),
                        self.context.clone(),
                    ));
                }
            }
            RibbonDisplayMode::Compact => {
                // Show a thin bar with just small icons — simplified for now
                container = container.child(
                    div()
                        .h(px(24.0))
                        .w_full()
                        .bg(theme.panel_background)
                        .border_b_1()
                        .border_color(theme.border),
                );
            }
            RibbonDisplayMode::AutoHide => {
                // Nothing shown below tabs
            }
        }

        container
    }
}
