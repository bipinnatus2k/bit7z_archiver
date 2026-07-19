use gpui::*;
use gpui_component::separator::Separator;
use smallvec::SmallVec;

#[derive(IntoElement)]
pub struct AppStatusBar {
    left_icon: SmallVec<[AnyElement; 1]>,
    left_status_text: SharedString,
    right_status_text: SharedString,
}

impl AppStatusBar {
    pub fn new() -> Self {
        Self {
            left_icon: SmallVec::new(),
            left_status_text: SharedString::default(),
            right_status_text: SharedString::default(),
        }
    }

    pub fn default() -> Self {
        Self::new()
    }

    /// Append an element to the left region. Call multiple times to add more.
    pub fn left_icon(mut self, child: impl IntoElement) -> Self {
        self.left_icon.push(child.into_any_element());
        self
    }

    pub fn left(mut self, text: impl Into<SharedString>) -> Self {
        self.left_status_text = text.into();
        self
    }

    pub fn right(mut self, text: impl Into<SharedString>) -> Self {
        self.right_status_text = text.into();
        self
    }
}

impl RenderOnce for AppStatusBar {
    fn render(self, _window: &mut Window, cx: &mut gpui::App) -> impl IntoElement {
        gpui_component::status_bar::StatusBar::new()
            .left(div().children(self.left_icon))
            .left(self.left_status_text.clone())
            .child(Separator::vertical())
            .right(self.right_status_text.clone())
    }
}
