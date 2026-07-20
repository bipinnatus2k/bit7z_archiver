use std::rc::Rc;

use gpui::{
    AnyElement, App, ClickEvent, Div, ElementId, InteractiveElement, IntoElement, ParentElement,
    Pixels, RenderOnce, SharedString, Stateful, StatefulInteractiveElement as _, StyleRefinement,
    Styled, Window, div, prelude::FluentBuilder as _, px, relative,
};
use smallvec::SmallVec;

use gpui_component::{ActiveTheme, Colorize as _, Disableable, Icon, IconName, Sizable, StyledExt, h_flex};

/// The size of a [`ToolbarButton`].
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum ToolbarButtonSize {
    XSmall,
    Small,
    #[default]
    Medium,
    Large,
}

impl ToolbarButtonSize {
    fn height(self) -> Pixels {
        match self {
            ToolbarButtonSize::XSmall => px(28.),
            ToolbarButtonSize::Small => px(32.),
            ToolbarButtonSize::Medium => px(38.),
            ToolbarButtonSize::Large => px(46.),
        }
    }
}

/// A horizontal toolbar, usually placed below a title bar.
///
/// Split into three regions — `left`, `center` (via `child`/`children`),
/// and `right`.  Items added to `left` are pinned left, items added to
/// `right` are pinned right, and items added via `child`/`children` sit
/// in the middle.
///
/// # Usage
/// ```ignore
/// use bit7z_pres_components::toolbar::{Toolbar, ToolbarButton};
/// use gpui_component::IconName;
///
/// Toolbar::new()
///     .child(ToolbarButton::new("open").icon(IconName::FolderOpen).label("Open"))
///     .right(ToolbarButton::new("settings").icon(IconName::Settings));
/// ```
#[derive(IntoElement)]
pub struct Toolbar {
    style: StyleRefinement,
    left: SmallVec<[AnyElement; 2]>,
    right: SmallVec<[AnyElement; 2]>,
    children: SmallVec<[AnyElement; 8]>,
}

impl Toolbar {
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            left: SmallVec::new(),
            right: SmallVec::new(),
            children: SmallVec::new(),
        }
    }

    pub fn left(mut self, child: impl IntoElement) -> Self {
        self.left.push(child.into_any_element());
        self
    }

    pub fn right(mut self, child: impl IntoElement) -> Self {
        self.right.push(child.into_any_element());
        self
    }
}

impl ParentElement for Toolbar {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for Toolbar {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Toolbar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let has_left = !self.left.is_empty();
        let has_right = !self.right.is_empty();
        let region = || h_flex().overflow_hidden().items_center().gap_1();

        h_flex()
            .items_center()
            .gap_1()
            .py_1()
            .px_2()
            .border_b_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().tokens.background)
            .text_sm()
            .refine_style(&self.style)
            .when(has_left, |this| this.child(region().children(self.left)))
            .child(
                region()
                    .flex_1()
                    .when(has_left && has_right, |this| this.justify_center())
                    .when(has_left && !has_right, |this| this.justify_end())
                    .children(self.children),
            )
            .when(has_right, |this| this.child(region().children(self.right)))
    }
}

/// A toolbar-specific button, designed for PC desktop toolbars with
/// optional vertical icon+label layout.
///
/// By default the icon and label are laid out horizontally. Call
/// [`vertical`](Self::vertical) to stack the icon above the label.
///
/// # Example
///
/// ```ignore
/// ToolbarButton::new("open")
///     .icon(IconName::FolderOpen)
///     .label("Open")
///     .vertical()
///     .on_click(|_, _, _| {});
///
/// ToolbarButton::new("save")
///     .icon(IconName::Save)
///     .small()
///     .disabled(true);
/// ```
#[derive(IntoElement)]
pub struct ToolbarButton {
    id: ElementId,
    base: Stateful<Div>,
    style: StyleRefinement,
    icon: Option<IconName>,
    label: Option<SharedString>,
    disabled: bool,
    size: ToolbarButtonSize,
    vertical: bool,
    on_click: Option<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>>,
}

impl ToolbarButton {
    pub fn new(id: impl Into<ElementId>) -> Self {
        let id = id.into();
        Self {
            id: id.clone(),
            base: div().id(id),
            style: StyleRefinement::default(),
            icon: None,
            label: None,
            disabled: false,
            size: ToolbarButtonSize::default(),
            vertical: false,
            on_click: None,
        }
    }

    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn vertical(mut self) -> Self {
        self.vertical = true;
        self
    }

    pub fn xsmall(mut self) -> Self {
        self.size = ToolbarButtonSize::XSmall;
        self
    }

    pub fn small(mut self) -> Self {
        self.size = ToolbarButtonSize::Small;
        self
    }

    pub fn medium(mut self) -> Self {
        self.size = ToolbarButtonSize::Medium;
        self
    }

    pub fn large(mut self) -> Self {
        self.size = ToolbarButtonSize::Large;
        self
    }

    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }
}

impl Disableable for ToolbarButton {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Styled for ToolbarButton {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for ToolbarButton {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let is_disabled = self.disabled;
        let is_icon_only = self.label.is_none();
        let size = self.size;

        let focus_handle = window
            .use_keyed_state(self.id.clone(), cx, |_, cx| cx.focus_handle())
            .read(cx)
            .clone();

        let fg = if is_disabled {
            cx.theme().muted_foreground.opacity(0.4)
        } else {
            cx.theme().muted_foreground
        };

        let content = {
            let mut inner = div().flex().items_center().justify_center().gap_1();

            if self.vertical {
                inner = inner
                    .flex_col()
                    .gap_0()
                    .px_2()
                    .py_1();
            } else {
                inner = inner
                    .px_3();
            }

            if let Some(name) = self.icon {
                let icon_size = match size {
                    ToolbarButtonSize::XSmall | ToolbarButtonSize::Small => gpui_component::Size::XSmall,
                    _ => gpui_component::Size::Small,
                };
                inner = inner.child(Icon::new(name).with_size(icon_size).flex_none());
            }

            if let Some(text) = self.label {
                inner = inner.child(
                    div()
                        .flex_none()
                        .line_height(relative(1.))
                        .when(self.vertical, |this| this.text_center().text_xs().w_full())
                        .when(!self.vertical, |this| this.text_sm())
                        .child(text),
                );
            }

            inner
        };

        self.base
            .track_focus(&focus_handle)
            .cursor_pointer()
            .flex()
            .flex_shrink_0()
            .items_center()
            .justify_center()
            .rounded_md()
            .h(size.height())
            .when(is_icon_only, |this| this.w(size.height()))
            .text_color(fg)
            .bg(cx.theme().transparent)
            .when(!is_disabled, |this| {
                this.hover(|this| this.bg(cx.theme().secondary.opacity(0.15)))
                    .active(|this| this.bg(cx.theme().secondary.opacity(0.25)))
            })
            .when(is_disabled, |this| {
                this.hover(|this| this).active(|this| this)
            })
            .refine_style(&self.style)
            .when_some(self.on_click, |this, handler| {
                this.on_click(move |event, window, cx| {
                    if is_disabled {
                        cx.stop_propagation();
                        return;
                    }
                    handler(event, window, cx);
                })
            })
            .child(content)
    }
}
