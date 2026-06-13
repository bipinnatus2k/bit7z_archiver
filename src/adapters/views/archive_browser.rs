use crate::adapters::view_models::archive_vm::ArchiveViewModel;
use crate::theme::Theme;
use gpui::*;

pub struct ArchiveBrowser {
    archive_vm: Entity<ArchiveViewModel>,
    filter_buf: SharedString,
    filter_focus: FocusHandle,
}

impl ArchiveBrowser {
    pub fn new(archive_vm: Entity<ArchiveViewModel>, cx: &mut Context<Self>) -> Self {
        Self {
            archive_vm,
            filter_buf: SharedString::new(""),
            filter_focus: cx.focus_handle(),
        }
    }
}

impl Focusable for ArchiveBrowser {
    fn focus_handle(&self, _app: &gpui::App) -> FocusHandle {
        self.filter_focus.clone()
    }
}

impl Render for ArchiveBrowser {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let vm = self.archive_vm.read(cx);
        div().flex().flex_col().w(px(240.)).border_r_1().border_color(cx.global::<Theme>().border).p_2().gap_2()
            .child(
                div()
                    .key_context("FilterInput")
                    .on_mouse_down(MouseButton::Left, cx.listener(|_this: &mut ArchiveBrowser, _event: &MouseDownEvent, _window: &mut Window, _cx| {

                    }))
                    .on_key_down(cx.listener(|this: &mut ArchiveBrowser, event: &KeyDownEvent, _window: &mut Window, cx| {
                        let mut text = this.filter_buf.to_string();
                        if let Some(ref ch) = event.keystroke.key_char {
                            text.push_str(&ch);
                        } else if event.keystroke.key == "backspace" {
                            text.pop();
                        }
                        this.filter_buf = SharedString::new(text);
                        this.archive_vm.update(cx, |vm, cx| {
                            vm.set_filter(&this.filter_buf, cx);
                        });
                    }))
                    .px_2().py_1().border_1().border_color(cx.global::<Theme>().border).rounded_md()
                    .bg(cx.global::<Theme>().surface)
                    .child(
                        if self.filter_buf.is_empty() {
                            div().text_color(cx.global::<Theme>().muted).child("Filter...")
                        } else {
                            div().child(self.filter_buf.to_string())
                        }
                    )
            )
            .child(div().flex().flex_col().text_sm().children(
                vm.entries.iter().filter(|e| e.is_directory).map(|e|
                    div().px_2().py_1().cursor_pointer().child(format!("📁 {}", e.name))
                ).collect::<Vec<_>>()
            ))
    }
}
