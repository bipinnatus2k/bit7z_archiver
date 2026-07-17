use gpui::{AnyElement, App, IntoElement, ParentElement, RenderOnce, StyleRefinement, Styled, Window, div};
use gpui_component::StyledExt;

/// A generic toolbar item that wraps any content and adapts to toolbar sizing.
#[derive(IntoElement)]
pub struct ToolbarItem {
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl ToolbarItem {
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            children: Vec::new(),
        }
    }
}

impl Default for ToolbarItem {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for ToolbarItem {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for ToolbarItem {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for ToolbarItem {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .flex_none()
            .h_full()
            .items_center()
            .refine_style(&self.style)
            .children(self.children)
    }
}
