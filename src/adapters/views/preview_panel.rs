use crate::adapters::view_models::preview_vm::PreviewViewModel;
use crate::application::preview::PreviewData;
use crate::adapters::views::components::state_view::{empty_view, loading_view};
use crate::theme::Theme;
use gpui::*;

pub struct PreviewPanel {
    preview_vm: Entity<PreviewViewModel>,
}
impl PreviewPanel {
    pub fn new(preview_vm: Entity<PreviewViewModel>) -> Self {
        Self { preview_vm }
    }
}
impl Render for PreviewPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let vm = self.preview_vm.read(cx);
        div().h(px(200.)).border_t_1().border_color(cx.global::<Theme>().border).p_2()
            .child(match &vm.data {
                None if vm.is_loading => loading_view(cx).into_any_element(),
                None => empty_view(cx, "Select a file to preview").into_any_element(),
                Some(PreviewData::Text(text)) => div().font_family("monospace").text_sm().child(text.clone()).into_any_element(),
                Some(PreviewData::Hex(_)) => div().font_family("monospace").text_sm().child("Binary data (hex view)").into_any_element(),
                Some(PreviewData::Image(_)) => div().child("Image preview").into_any_element(),
                Some(PreviewData::Unsupported(msg)) => div().text_color(cx.global::<Theme>().muted).child(msg.clone()).into_any_element(),
            })
    }
}
