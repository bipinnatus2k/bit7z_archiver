use bit7z_app_preview::PreviewData;
use bit7z_pres_components::state_view::{empty_view, loading_view};
use bit7z_pres_theme::Theme;
use gpui::*;
use gpui_component::scroll::ScrollableElement;

pub struct PreviewPanel {
    data: Option<PreviewData>,
    is_loading: bool,
}
impl PreviewPanel {
    pub fn new() -> Self {
        Self { data: None, is_loading: false }
    }

    pub fn set_data(&mut self, data: Option<PreviewData>) {
        self.data = data;
        self.is_loading = false;
    }

    pub fn set_loading(&mut self) {
        self.is_loading = true;
    }
}
impl Render for PreviewPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().flex_1().overflow_hidden().border_t_1().border_color(cx.global::<Theme>().border).p_2()
            .child(match &self.data {
                None if self.is_loading => loading_view(cx).into_any_element(),
                None => empty_view(cx, "Select a file to preview").into_any_element(),
                Some(PreviewData::Text(text)) => div().font_family("monospace").text_sm().overflow_y_scrollbar().child(text.clone()).into_any_element(),
                Some(PreviewData::Hex(_)) => div().font_family("monospace").text_sm().child("Binary data (hex view)").into_any_element(),
                Some(PreviewData::Image(_)) => div().child("Image preview").into_any_element(),
                Some(PreviewData::Unsupported(msg)) => div().text_color(cx.global::<Theme>().muted).child(msg.clone()).into_any_element(),
            })
    }
}
