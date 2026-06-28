use crate::theme::Theme;
use gpui::*;
use gpui::prelude::FluentBuilder;

pub struct StatusBar {
    status_text: String,
}

impl StatusBar {
    pub fn new() -> Self {
        Self { status_text: String::new() }
    }

    pub fn set_status(&mut self, text: &str) {
        self.status_text = text.to_string();
    }
}

impl Render for StatusBar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let window_width = window.bounds().size.width;
        let compact = window_width < px(640.);

        div().flex().flex_row().h(px(30.)).w(relative(1.)).justify_between().px_3().py_1()
            .border_t_1().border_color(cx.global::<Theme>().border).text_sm()
            .child(div().child(self.status_text.clone()))
            .when(!compact, |el| {
                el.child(div().text_color(cx.global::<Theme>().muted).child("bit7z Archiver"))
            })
    }
}
