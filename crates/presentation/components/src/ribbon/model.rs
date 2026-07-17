use gpui::SharedString;
use crate::ribbon::{CommandContext, ContextPredicate};
use super::command::CommandId;

/// Defines what type of button to render.
#[derive(Clone, Debug)]
pub enum RibbonItemKind {
    /// Large button with icon above text.
    LargeButton {
        command_id: CommandId,
    },
    /// Small button with icon left of text.
    SmallButton {
        command_id: CommandId,
    },
    /// A column of 2-3 small buttons stacked vertically.
    ButtonColumn {
        items: Vec<CommandId>,
    },
    /// Split button: top is primary action, bottom arrow opens dropdown.
    SplitButton {
        primary_command_id: CommandId,
        dropdown_items: Vec<CommandId>,
    },
    /// A gallery grid that can expand into a popup.
    DropdownGallery {
        label: SharedString,
        items: Vec<GalleryItem>,
        columns: usize,
    },
    /// Visual separator between items.
    Separator,
}

/// An item in a dropdown gallery.
#[derive(Clone, Debug)]
pub struct GalleryItem {
    pub id: SharedString,
    pub label: SharedString,
    pub icon: Option<SharedString>,
    pub command_id: CommandId,
}

impl GalleryItem {
    pub fn new(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        command_id: impl Into<CommandId>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: None,
            command_id: command_id.into(),
        }
    }

    pub fn icon(mut self, icon: impl Into<SharedString>) -> Self {
        self.icon = Some(icon.into());
        self
    }
}

/// A group of ribbon items with a label.
#[derive(Clone, Debug)]
pub struct RibbonGroup {
    pub id: SharedString,
    pub label: SharedString,
    pub items: Vec<RibbonItemKind>,
}

impl RibbonGroup {
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            items: Vec::new(),
        }
    }

    pub fn item(mut self, item: RibbonItemKind) -> Self {
        self.items.push(item);
        self
    }

    pub fn large_button(mut self, command_id: impl Into<CommandId>) -> Self {
        self.items.push(RibbonItemKind::LargeButton {
            command_id: command_id.into(),
        });
        self
    }

    pub fn small_button(mut self, command_id: impl Into<CommandId>) -> Self {
        self.items.push(RibbonItemKind::SmallButton {
            command_id: command_id.into(),
        });
        self
    }

    pub fn button_column(mut self, command_ids: Vec<CommandId>) -> Self {
        self.items.push(RibbonItemKind::ButtonColumn {
            items: command_ids,
        });
        self
    }

    pub fn separator(mut self) -> Self {
        self.items.push(RibbonItemKind::Separator);
        self
    }

    pub fn split_button(
        mut self,
        primary: impl Into<CommandId>,
        dropdown: Vec<CommandId>,
    ) -> Self {
        self.items.push(RibbonItemKind::SplitButton {
            primary_command_id: primary.into(),
            dropdown_items: dropdown,
        });
        self
    }
}

/// Predicate that determines when a contextual tab should be visible.
pub type TabContextPredicate = Option<Box<dyn Fn(&CommandContext) -> bool + Send + Sync>>;

/// A tab in the ribbon containing groups.
#[derive(Clone)]
pub struct RibbonTab {
    pub id: SharedString,
    pub label: SharedString,
    pub groups: Vec<RibbonGroup>,
    /// If true, this tab only shows when its context_predicate returns true.
    pub is_contextual: bool,
    /// Accent color for contextual tab header (hex string).
    pub contextual_color: Option<SharedString>,
}

impl RibbonTab {
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            groups: Vec::new(),
            is_contextual: false,
            contextual_color: None,
        }
    }

    pub fn contextual(
        mut self,
        color: impl Into<SharedString>,
    ) -> Self {
        self.is_contextual = true;
        self.contextual_color = Some(color.into());
        self
    }

    pub fn group(mut self, group: RibbonGroup) -> Self {
        self.groups.push(group);
        self
    }
}

/// The display mode of the ribbon.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RibbonDisplayMode {
    /// Full ribbon with large icons and text.
    Expanded,
    /// Compact mode with small icons only.
    Compact,
    /// Auto-hide: only tab bar visible, click to show content as popup.
    AutoHide,
}

/// Quick access toolbar item.
#[derive(Clone, Debug)]
pub struct QuickAccessItem {
    pub command_id: CommandId,
}

/// The complete ribbon model.
#[derive(Clone)]
pub struct RibbonModel {
    pub tabs: Vec<RibbonTab>,
    pub quick_access_items: Vec<QuickAccessItem>,
    pub display_mode: RibbonDisplayMode,
    pub active_tab_index: usize,
}

impl RibbonModel {
    pub fn new() -> Self {
        Self {
            tabs: Vec::new(),
            quick_access_items: Vec::new(),
            display_mode: RibbonDisplayMode::Expanded,
            active_tab_index: 0,
        }
    }

    pub fn tab(mut self, tab: RibbonTab) -> Self {
        self.tabs.push(tab);
        self
    }

    pub fn quick_access(mut self, command_id: impl Into<CommandId>) -> Self {
        self.quick_access_items.push(QuickAccessItem {
            command_id: command_id.into(),
        });
        self
    }

    pub fn active_tab(&self) -> Option<&RibbonTab> {
        self.tabs.get(self.active_tab_index)
    }

    pub fn visible_tabs<'a>(
        &'a self,
        ctx: &'a CommandContext,
        context_predicates: &'a std::collections::HashMap<SharedString, ContextPredicate>,
    ) -> Vec<(usize, &'a RibbonTab)> {
        self.tabs
            .iter()
            .enumerate()
            .filter(|(_, tab)| {
                if tab.is_contextual {
                    context_predicates
                        .get(&tab.id)
                        .map_or(false, |pred| pred(ctx))
                } else {
                    true
                }
            })
            .collect()
    }
}

impl Default for RibbonModel {
    fn default() -> Self {
        Self::new()
    }
}
