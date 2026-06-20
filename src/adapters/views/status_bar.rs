use crate::adapters::view_models::archive_vm::{ArchiveViewModel, ViewStatus};
use crate::theme::Theme;
use gpui::*;
use gpui::prelude::FluentBuilder;
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
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let vm = self.archive_vm.read(cx);
        let window_width = window.bounds().size.width;
        let compact = window_width < px(640.);

        let status = match &vm.status {
            ViewStatus::Empty => "No archive open".into(),
            ViewStatus::Loading => "Loading...".into(),
            ViewStatus::Ready => {
                let mut parts = vec![];
                let count = vm.level_entries.len();
                if count > 0 {
                    parts.push(format!("{} entries", count));
                }
                if !compact {
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
                }
                if parts.is_empty() { String::new() } else { parts.join(" | ") }
            }
            ViewStatus::Error(e) => {
                if compact {
                    "Error".to_string()
                } else {
                    format!("Error: {}", e)
                }
            }
        };
        div().flex().flex_row().justify_between().px_3().py_1()
            .border_t_1().border_color(cx.global::<Theme>().border).text_sm()
            .child(div().child(status))
            .when(!compact, |el| {
                el.child(div().text_color(cx.global::<Theme>().muted).child("bit7z Archiver"))
            })
    }
}
