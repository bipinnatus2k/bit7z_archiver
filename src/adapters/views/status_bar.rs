use crate::adapters::view_models::archive_vm::{ArchiveViewModel, ViewStatus};
use crate::theme::Theme;
use gpui::*;
use humansize::{format_size, BINARY};

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
            ViewStatus::Loading => format!("Loading page {}...", vm.current_offset / 200 + 1),
            ViewStatus::Ready => {
                let mut parts = vec![];
                let count = vm.entries.len();
                if count > 0 {
                    parts.push(format!("{} entries", count));
                }
                if let Some(ref props) = vm.properties {
                    if props.folders_count > 0 {
                        parts.push(format!("{} folders", props.folders_count));
                    }
                    if props.files_count > 0 {
                        parts.push(format!("{} files", props.files_count));
                    }
                    if props.is_encrypted {
                        parts.push("Encrypted".to_string());
                    }
                    if props.total_size > 0 {
                        parts.push(format!("Size: {}", format_size(props.total_size, BINARY)));
                    }
                    if props.packed_size > 0 {
                        parts.push(format!("Packed: {}", format_size(props.packed_size, BINARY)));
                    }
                }
                if parts.is_empty() { String::new() } else { parts.join(" | ") }
            }
            ViewStatus::Error(e) => format!("Error: {}", e),
        };
        div().flex().flex_row().justify_between().px_3().py_1()
            .border_t_1().border_color(cx.global::<Theme>().border).text_sm()
            .child(div().child(status))
            .child(div().text_color(cx.global::<Theme>().muted).child("bit7z Archiver"))
    }
}

