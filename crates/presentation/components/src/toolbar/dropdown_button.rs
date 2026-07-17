use gpui::Corners;
use gpui::{
    Anchor, App, Context, Edges, ElementId, InteractiveElement as _, IntoElement,
    ParentElement, RenderOnce, SharedString, StyleRefinement, Styled, Window, div,
    prelude::FluentBuilder,
};

use gpui_component::{
    Disableable, IconName, Selectable, Sizable, Size, StyledExt as _,
    menu::{DropdownMenu, PopupMenu},
    // tooltip::ComponentTooltip,
};
use gpui_component::tooltip::Tooltip;
use crate::toolbar::button::{ToolbarButton, ToolbarButtonRounded, ToolbarButtonVariant, ButtonVariants};

#[derive(IntoElement)]
pub struct ToolbarDropdownButton {
    id: ElementId,
    style: StyleRefinement,
    button: Option<ToolbarButton>,
    menu:
        Option<Box<dyn Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static>>,
    selected: bool,
    disabled: bool,
    // The button props
    compact: bool,
    outline: bool,
    loading: bool,
    variant: ToolbarButtonVariant,
    size: Size,
    rounded: ToolbarButtonRounded,
    anchor: Anchor,
    tooltip: Tooltip,
}

impl ToolbarDropdownButton {
    /// Create a new DropdownButton.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            button: None,
            menu: None,
            selected: false,
            disabled: false,
            compact: false,
            outline: false,
            loading: false,
            variant: ToolbarButtonVariant::default(),
            size: Size::default(),
            rounded: ToolbarButtonRounded::default(),
            anchor: Anchor::TopRight,
            tooltip: Tooltip::new(""),
        }
    }

    /// Set tooltip text for the dropdown button.
    pub fn tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.tooltip = Tooltip::new(tooltip.into());
        self
    }

    /// Set the left button of the dropdown button.
    pub fn button(mut self, button: ToolbarButton) -> Self {
        self.button = Some(button);
        self
    }

    /// Set the dropdown menu of the button.
    pub fn dropdown_menu(
        mut self,
        menu: impl Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
    ) -> Self {
        self.menu = Some(Box::new(menu));
        self
    }

    /// Set the dropdown menu of the button with anchor corner.
    pub fn dropdown_menu_with_anchor(
        mut self,
        anchor: impl Into<Anchor>,
        menu: impl Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
    ) -> Self {
        self.menu = Some(Box::new(menu));
        self.anchor = anchor.into();
        self
    }

    /// Set the rounded style of the button.
    pub fn rounded(mut self, rounded: impl Into<ToolbarButtonRounded>) -> Self {
        self.rounded = rounded.into();
        self
    }

    /// Set the button to compact style.
    ///
    /// See also: [`ToolbarButton::compact`]
    pub fn compact(mut self) -> Self {
        self.compact = true;
        self
    }

    /// Set the button to outline style.
    ///
    /// See also: [`ToolbarButton::outline`]
    pub fn outline(mut self) -> Self {
        self.outline = true;
        self
    }

    /// Set the button to loading state.
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }
}

impl Disableable for ToolbarDropdownButton {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Styled for ToolbarDropdownButton {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Sizable for ToolbarDropdownButton {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl ButtonVariants for ToolbarDropdownButton {
    fn with_variant(mut self, variant: ToolbarButtonVariant) -> Self {
        self.variant = variant;
        self
    }
}

impl Selectable for ToolbarDropdownButton {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.selected
    }
}

impl RenderOnce for ToolbarDropdownButton {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let rounded = self.variant.is_ghost() && !self.selected;

        div()
            .id(self.id)
            .h_flex()
            .refine_style(&self.style)
            .when_some(self.button, |this, button| {
                this.child(
                    button
                        .rounded(self.rounded)
                        .border_corners(Corners {
                            top_left: true,
                            top_right: rounded,
                            bottom_left: true,
                            bottom_right: rounded,
                        })
                        .border_edges(Edges {
                            left: true,
                            top: true,
                            right: true,
                            bottom: true,
                        })
                        .loading(self.loading)
                        .selected(self.selected)
                        .disabled(self.disabled || self.loading)
                        .when(self.compact, |this| this.compact())
                        .when(self.outline, |this| this.outline())
                        .with_size(self.size)
                        .with_variant(self.variant),
                )
                .when_some(self.menu, |this, menu| {
                    this.child(
                        ToolbarButton::new("popup")
                            .icon(IconName::ChevronDown)
                            .rounded(self.rounded)
                            .border_edges(Edges {
                                left: rounded,
                                top: true,
                                right: true,
                                bottom: true,
                            })
                            .border_corners(Corners {
                                top_left: rounded,
                                top_right: true,
                                bottom_left: rounded,
                                bottom_right: true,
                            })
                            .selected(self.selected)
                            .disabled(self.disabled || self.loading)
                            .when(self.compact, |this| this.compact())
                            .when(self.outline, |this| this.outline())
                            .with_size(self.size)
                            .with_variant(self.variant)
                            .dropdown_menu_with_anchor(self.anchor, menu),
                    )
                })
            })
            // .map(|this| self.tooltip.apply(this))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[gpui::test]
    fn test_dropdown_button_builder(_cx: &mut gpui::TestAppContext) {
        let button = ToolbarButton::new("inner").label("Action");
        let dropdown = ToolbarDropdownButton::new("complex-dropdown")
            .button(button)
            .primary()
            .outline()
            .large()
            .compact()
            .loading(false)
            .disabled(false)
            .selected(false)
            .rounded(ToolbarButtonRounded::Medium)
            .dropdown_menu_with_anchor(Anchor::BottomLeft, |menu, _, _| menu);

        assert!(dropdown.button.is_some());
        assert_eq!(dropdown.variant, ToolbarButtonVariant::Primary);
        assert!(dropdown.outline);
        assert_eq!(dropdown.size, Size::Large);
        assert!(dropdown.compact);
        assert!(!dropdown.loading);
        assert!(!dropdown.disabled);
        assert!(!dropdown.selected);
        assert!(matches!(dropdown.rounded, ToolbarButtonRounded::Medium));
        assert!(dropdown.menu.is_some());
        assert_eq!(dropdown.anchor, Anchor::BottomLeft);
    }
}
