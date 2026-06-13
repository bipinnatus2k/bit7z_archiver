use crate::adapters::view_models::archive_vm::ArchiveViewModel;
use crate::theme::Theme;
use gpui::*;

pub struct ArchiveBrowser {
    archive_vm: Entity<ArchiveViewModel>,
}

impl ArchiveBrowser {
    pub fn new(archive_vm: Entity<ArchiveViewModel>) -> Self {
        Self { archive_vm }
    }
}

impl Render for ArchiveBrowser {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let vm = self.archive_vm.read(cx);
        div().flex().flex_col().w(px(240.)).border_r_1().border_color(cx.global::<Theme>().border).p_2().gap_2()
            .child(div().px_2().py_1().border_1().border_color(cx.global::<Theme>().border).rounded_md().child("Filter..."))
            .child(div().flex().flex_col().text_sm().children(
                vm.entries.iter().filter(|e| e.is_directory).map(|e|
                    div().px_2().py_1().cursor_pointer().child(format!("📁 {}", e.name))
                ).collect::<Vec<_>>()
            ))
    }
}
