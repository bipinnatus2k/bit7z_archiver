use gpui::prelude::FluentBuilder;
use gpui::private::serde_json;
use gpui::{div, px, AnyView, App, AppContext, Context, EventEmitter, Focusable, Hsla, InteractiveElement, IntoElement, ParentElement, Pixels, Render, SharedString, Styled, Window};
use gpui_component::dock::{Panel, PanelControl, PanelEvent};
use gpui_component::scroll::ScrollableElement;
use serde::{Deserialize, Serialize};

pub struct StoryContainer {
    focus_handle: gpui::FocusHandle,
    pub name: SharedString,
    pub title_bg: Option<Hsla>,
    // pub description: SharedString,
    width: Option<gpui::Pixels>,
    height: Option<gpui::Pixels>,
    story: Option<AnyView>,
    // story_klass: Option<SharedString>,
    closable: bool,
    zoomable: Option<PanelControl>,
    paddings: Pixels,
    on_active: Option<fn(AnyView, bool, &mut Window, &mut App)>,
}


#[derive(Debug)]
pub enum ContainerEvent {
    Close,
}

impl EventEmitter<ContainerEvent> for StoryContainer {}

impl StoryContainer {
    pub fn new(cx: &mut App) -> Self {
        let focus_handle = cx.focus_handle();

        Self {
            focus_handle,
            name: "".into(),
            title_bg: None,
            // description: "".into(),
            width: None,
            height: None,
            story: None,
            // story_klass: None,
            closable: true,
            zoomable: Some(PanelControl::default()),
            paddings: px(16.),
            on_active: None,
        }
    }

    pub fn width(mut self, width: gpui::Pixels) -> Self {
        self.width = Some(width);
        self
    }

    pub fn height(mut self, height: gpui::Pixels) -> Self {
        self.height = Some(height);
        self
    }

    pub fn subview(mut self, story: AnyView) -> Self {
        self.story = Some(story);
        // self.story_klass = Some(story_klass.into());
        self
    }

    pub fn on_active(mut self, on_active: fn(AnyView, bool, &mut Window, &mut App)) -> Self {
        self.on_active = Some(on_active);
        self
    }
}

impl EventEmitter<PanelEvent> for StoryContainer {}
impl Focusable for StoryContainer {
    fn focus_handle(&self, _: &App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}
impl Render for StoryContainer {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("story-container")
            .size_full()
            .overflow_y_scrollbar()
            .track_focus(&self.focus_handle)
            .when_some(self.story.clone(), |this, story| {
                this.child(div().size_full().p(self.paddings).child(story))
            })
    }
}

// impl Render for StoryContainer {
//     fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
//         div().size_full().relative()
//             .child(
//                 v_flex().size_full()
//                     .justify_between()
//                     .child(
//                         v_flex()
//                             .child(self.menu.clone())
//                             .child(self.toolbar.clone())
//                             .child(div().flex_1().child(
//                                 h_resizable("main-hz")
//                                     .child(resizable_panel().size(px(255.)).size_range(px(200.)..px(320.)).child(self.archive_browser.clone()))
//                                     .child(v_resizable("main-vt")
//                                         .child(resizable_panel().child(self.entry_list.clone()))
//                                         .child(resizable_panel().size(px(200.)).size_range(px(100.)..px(500.)).child(self.preview_panel.clone()))
//                                     )
//                             ))
//                     )
//                     .child(AppStatusBar::new().left(self.left_status.clone()).right(self.right_status.clone()))
//             )
//             .children(Root::render_dialog_layer(window, cx))
//     }
// }