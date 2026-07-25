use std::sync::Arc;

use bit7z_app_preview::PreviewData;
use bit7z_pres_components::state_view::{empty_view, loading_view};
use gpui::*;
use gpui_component::ActiveTheme;
use gpui_component::scroll::ScrollableElement;

#[derive(IntoElement)]
pub struct PreviewPanelView {
    data: Option<Arc<PreviewData>>,
    is_loading: bool,
}

impl PreviewPanelView {
    pub fn new(data: Option<Arc<PreviewData>>, is_loading: bool) -> Self {
        Self { data, is_loading }
    }
}

impl RenderOnce for PreviewPanelView {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .flex_1()
            .overflow_hidden()
            .border_t_1()
            .border_color(cx.theme().border)
            .p_2()
            .child(match &self.data {
                None if self.is_loading => loading_view(cx).into_any_element(),
                None => empty_view(cx, "Select a file to preview").into_any_element(),
                Some(data) => match data.as_ref() {
                    PreviewData::Text(text) => div()
                        .font_family("monospace")
                        .text_sm()
                        .overflow_y_scrollbar()
                        .child(text.clone())
                        .into_any_element(),
                    PreviewData::Image(_) => div().child("Image preview").into_any_element(),
                    PreviewData::Unsupported(msg) => div()
                        .text_color(cx.theme().muted)
                        .child(msg.clone())
                        .into_any_element(),
                },
            })
    }
}
