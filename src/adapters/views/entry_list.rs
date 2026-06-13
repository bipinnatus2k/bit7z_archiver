use crate::adapters::view_models::archive_vm::{ArchiveViewModel, ViewStatus};
use crate::theme::Theme;
use gpui::prelude::FluentBuilder as _;
use gpui::*;

pub struct EntryList {
    pub archive_vm: Entity<ArchiveViewModel>,
}

impl Render for EntryList {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let vm = self.archive_vm.read(cx);
        div().flex().flex_col().flex_1()
            .border_b_1().border_color(cx.global::<Theme>().border)
            .children(match &vm.status {
                ViewStatus::Empty => vec![div().p_8().text_center().text_color(cx.global::<Theme>().muted)
                    .child("Open an archive to browse its contents")],
                ViewStatus::Loading => vec![div().p_8().text_center().child("Loading...")],
                ViewStatus::Error(msg) => vec![div().p_8().text_color(cx.global::<Theme>().error).child(format!("Error: {}", msg))],
                ViewStatus::Ready => {
                    let mut children = vec![div().flex().flex_row().gap_2().px_2().py_1()
                        .bg(cx.global::<Theme>().surface).font_weight(FontWeight::BOLD)
                        .child(div().flex_1().child("Name"))
                        .child(div().w(px(80.)).child("Size"))
                        .child(div().w(px(80.)).child("Packed"))
                        .child(div().w(px(80.)).child("Ratio"))
                        .child(div().w(px(140.)).child("Date"))];
                    for (i, entry) in vm.entries.iter().enumerate() {
                        let selected = vm.selection.contains(&(i as u32));
                        children.push(
                            div().flex().flex_row().gap_2().px_2().py_1()
                                .when(selected, |el| el.bg(cx.global::<Theme>().selection))
                                .cursor_pointer()
                                .child(div().flex_1().child(if entry.is_directory { format!("📁 {}", entry.name) } else { format!("📄 {}", entry.name) }))
                                .child(div().w(px(80.)).text_sm().child(format_size(entry.size)))
                                .child(div().w(px(80.)).text_sm().child(format_size(entry.compressed_size)))
                                .child(div().w(px(80.)).text_sm().child(format!("{:.0}%", entry.compression_ratio() * 100.)))
                                .child(div().w(px(140.)).text_sm().text_color(cx.global::<Theme>().muted).child(
                                    entry.modified.map(|t| t.format("%Y-%m-%d %H:%M").to_string()).unwrap_or_default()
                                ))
                        );
                    }
                    children
                }
            })
    }
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 { format!("{} B", bytes) }
    else if bytes < 1_048_576 { format!("{:.1} KB", bytes as f64 / 1024.0) }
    else if bytes < 1_073_741_824 { format!("{:.1} MB", bytes as f64 / 1_048_576.0) }
    else { format!("{:.2} GB", bytes as f64 / 1_073_741_824.0) }
}
