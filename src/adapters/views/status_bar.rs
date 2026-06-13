use crate::adapters::view_models::archive_vm::{ArchiveViewModel, ViewStatus};
use crate::theme::Theme;
use gpui::*;

pub struct StatusBar {
    archive_vm: Entity<ArchiveViewModel>,
}
impl StatusBar {
    pub fn new(archive_vm: Entity<ArchiveViewModel>) -> Self {
        Self { archive_vm }
    }
}
impl Render for StatusBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let vm = self.archive_vm.read(cx);
        let status = match &vm.status {
            ViewStatus::Empty => "No archive open".into(),
            ViewStatus::Loading => "Loading...".into(),
            ViewStatus::Ready => format!("{} entries", vm.entries.len()),
            ViewStatus::Error(e) => format!("Error: {}", e),
        };
        div().flex().flex_row().justify_between().px_3().py_1()
            .border_t_1().border_color(cx.global::<Theme>().border).text_sm()
            .child(div().child(status))
            .child(div().text_color(cx.global::<Theme>().muted).child("bit7z Archiver"))
    }
}
