use std::rc::Rc;
use gpui::{AnyElement, App, InteractiveElement as _, IntoElement, ParentElement, Pixels, RenderOnce, ScrollHandle, SharedString, StatefulInteractiveElement as _, StyleRefinement, Styled, Window, prelude::FluentBuilder as _, px, ClickEvent};
use smallvec::SmallVec;

use gpui_component::{ActiveTheme, Icon, IconName, Sizable, StyledExt, h_flex};
use gpui_component::label::Label;

use crate::toolbar::alignment::ToolbarAlignment;
use crate::toolbar::ToolbarButton;

/// A horizontal toolbar, usually placed at the top of a window or pane.
///
/// Supports configurable height, horizontal overflow scrolling with chevron
/// indicators, background blur simulation, and action-region alignment.
///
/// # Scroll overflow
///
/// When `overflow_scroll` is enabled (default) and the action items exceed the
/// toolbar width, chevron buttons (`<<` / `>>`) appear at the respective ends
/// to scroll the content into view.
#[derive(IntoElement)]
pub struct ToolBar {
    style: StyleRefinement,
    title: SharedString,
    leading: SmallVec<[AnyElement; 1]>,
    actions: SmallVec<[AnyElement; 1]>,
    height: Option<Pixels>,
    overflow_scroll: bool,
    backdrop_blur: bool,
    blur_amount: f32,
    actions_alignment: ToolbarAlignment,
}

pub struct ToolbarAction {
    id: String,
    icon: IconName,
    label: SharedString,
    show_label:bool,
    on_click: Option<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>>,
    on_hover: Option<Rc<dyn Fn(&bool, &mut Window, &mut App)>>,
}

impl ToolBar {
    /// Create a new, empty [`ToolBar`].
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            title: SharedString::default(),
            leading: SmallVec::new(),
            actions: SmallVec::new(),
            height: None,
            overflow_scroll: true,
            backdrop_blur: false,
            blur_amount: 8.0,
            actions_alignment: ToolbarAlignment::Start,
        }
    }

    pub fn title(mut self, text: impl Into<SharedString>) -> Self {
        self.title = text.into();
        self
    }

    /// Append an element to the leading region. Call multiple times to add more.
    pub fn leading(mut self, child: impl IntoElement) -> Self {
        self.leading.push(child.into_any_element());
        self
    }

    pub fn action(mut self, child: impl IntoElement) -> Self {
        self.actions.push(child.into_any_element());
        self
    }
    
    pub fn actions(mut self, child: Vec<impl IntoElement>) -> Self {
        self.actions.extend(child.into_iter().map(|child| child.into_any_element()));
        self
    }
    
    // pub fn actions_button(mut self, children: Vec<ToolbarAction>) -> Self {
    //     self.actions.extend(children.iter().map(|x| {
    //         ToolbarButton::new(x.id.clone())
    //             .icon(x.icon.clone())
    //             .when(x.show_label,|this| {
    //                 this.label(&x.label)
    //             })
    //             .into_any_element()
    //     }));
    //     self
    // }

    /// Set the toolbar height.
    ///
    /// Defaults to auto-height based on content.
    pub fn height(mut self, height: impl Into<Pixels>) -> Self {
        self.height = Some(height.into());
        self
    }

    /// Enable or disable horizontal overflow scrolling with chevron buttons.
    ///
    /// Enabled by default.
    pub fn overflow_scroll(mut self, enabled: bool) -> Self {
        self.overflow_scroll = enabled;
        self
    }

    /// Enable background blur.
    ///
    /// When enabled the toolbar background becomes semi-transparent to simulate
    /// a blurred appearance. The `amount` controls transparency (0.0 – 1.0).
    ///
    /// Note: GPUI does not currently support native backdrop blur; this uses
    /// transparency as an approximation.
    pub fn backdrop_blur(mut self, enabled: bool, amount: f32) -> Self {
        self.backdrop_blur = enabled;
        self.blur_amount = amount.clamp(0.0, 1.0);
        self
    }

    /// Set the alignment of action items within the actions region.
    ///
    /// Defaults to [`ToolbarAlignment::Start`].
    pub fn actions_alignment(mut self, alignment: ToolbarAlignment) -> Self {
        self.actions_alignment = alignment;
        self
    }
}

impl Default for ToolBar {
    fn default() -> Self {
        Self::new()
    }
}

/// `child` / `children` add to the center region, so a `ToolBar` without
/// `leading` items behaves like a plain container.
impl ParentElement for ToolBar {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.actions.extend(elements);
    }
}

impl Styled for ToolBar {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for ToolBar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let has_leading = !self.leading.is_empty();
        let has_title = !self.title.is_empty();
        let has_actions = !self.actions.is_empty();
        let scroll_id = "toolbar-scroll";
        let chevron_left_id = "toolbar-chevron-left";
        let chevron_right_id = "toolbar-chevron-right";

        let scroll_handle: ScrollHandle = window
            .use_keyed_state(scroll_id, cx, |_, _| ScrollHandle::default())
            .read(cx)
            .clone();

        let offset = scroll_handle.offset();
        let max_offset = scroll_handle.max_offset();
        let at_start = offset.x >= px(0.);
        let at_end = offset.x <= max_offset.x && max_offset.x != px(0.);
        let can_scroll = self.overflow_scroll && max_offset.x != px(0.);

        let region = || h_flex().overflow_hidden().items_center().gap_2();

        let scroll_step = px(100.);

        let base_bg = if self.backdrop_blur {
            cx.theme()
                .tokens
                .status_bar
                .background
                .opacity(1.0 - self.blur_amount * 0.5)
        } else {
            cx.theme().tokens.status_bar.background
        };

        h_flex()
            .items_center()
            .gap_1()
            .px_2()
            .border_t_1()
            .border_color(cx.theme().status_bar_border)
            .bg(base_bg)
            .text_xs()
            .text_color(cx.theme().muted_foreground)
            .when_some(self.height, |this, h| this.h(h))
            .refine_style(&self.style)
            .when(has_leading, |this| this.child(region().children(self.leading)))
            .when(has_title, |this| this.child(region().child(Label::new(self.title))))
            .when(has_actions && self.overflow_scroll && !at_start && can_scroll, |this| {
                this.child(
                    h_flex()
                        .id(chevron_left_id)
                        .flex_none()
                        .items_center()
                        .child(
                            Icon::new(IconName::ChevronLeft)
                                .small()
                                .text_color(cx.theme().muted_foreground),
                        )
                        .cursor_pointer()
                        .on_click({
                            let handle = scroll_handle.clone();
                            move |_, window, cx| {
                                let current = handle.offset();
                                let new_x = (current.x + scroll_step).min(px(0.));
                                handle.set_offset(gpui::point(new_x, current.y));
                                cx.notify(window.current_view());
                            }
                        }),
                )
            })
            .child(
                region()
                    .flex_1()
                    .id("toolbar-actions-scroll")
                    .when(self.overflow_scroll, |this| {
                        this.overflow_x_scroll().track_scroll(&scroll_handle)
                    })
                    .map(|this| match self.actions_alignment {
                        ToolbarAlignment::Start => this.justify_start(),
                        ToolbarAlignment::Center => this.justify_center(),
                        ToolbarAlignment::End => this.justify_end(),
                    })
                    .when(has_leading && has_title, |this| this.justify_center())
                    .children(self.actions),
            )
            .when(has_actions && self.overflow_scroll && !at_end && can_scroll, |this| {
                this.child(
                    h_flex()
                        .id(chevron_right_id)
                        .flex_none()
                        .items_center()
                        .child(
                            Icon::new(IconName::ChevronRight)
                                .small()
                                .text_color(cx.theme().muted_foreground),
                        )
                        .cursor_pointer()
                        .on_click({
                            let handle = scroll_handle.clone();
                            move |_, window, cx| {
                                let current = handle.offset();
                                let new_x = (current.x - scroll_step).max(max_offset.x);
                                handle.set_offset(gpui::point(new_x, current.y));
                                cx.notify(window.current_view());
                            }
                        }),
                )
            })
    }
}
