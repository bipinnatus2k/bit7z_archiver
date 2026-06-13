use crate::adapters::view_models::archive_vm::{ArchiveViewModel, ViewStatus};
use crate::theme::Theme;
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
                    let mut children = vec![
                        // Header row with sortable columns and sort indicators
                        div().flex().flex_row().gap_2().px_2().py_1()
                            .bg(cx.global::<Theme>().surface).font_weight(FontWeight::BOLD)
                            .child(div().flex_1().cursor_pointer().child(
                                if vm.sort_column == 0 {
                                    if vm.sort_ascending { "Name ▲" } else { "Name ▼" }
                                } else { "Name" }
                            ).on_mouse_down(MouseButton::Left, cx.listener(|this: &mut EntryList, _event: &MouseDownEvent, _window: &mut Window, cx| {
                                this.archive_vm.update(cx, |vm, cx| vm.sort_by(0, cx));
                            })))
                            .child(div().w(px(80.)).cursor_pointer().child(
                                if vm.sort_column == 1 {
                                    if vm.sort_ascending { "Size ▲" } else { "Size ▼" }
                                } else { "Size" }
                            ).on_mouse_down(MouseButton::Left, cx.listener(|this: &mut EntryList, _event: &MouseDownEvent, _window: &mut Window, cx| {
                                this.archive_vm.update(cx, |vm, cx| vm.sort_by(1, cx));
                            })))
                            .child(div().w(px(80.)).cursor_pointer().child(
                                if vm.sort_column == 2 {
                                    if vm.sort_ascending { "Packed ▲" } else { "Packed ▼" }
                                } else { "Packed" }
                            ).on_mouse_down(MouseButton::Left, cx.listener(|this: &mut EntryList, _event: &MouseDownEvent, _window: &mut Window, cx| {
                                this.archive_vm.update(cx, |vm, cx| vm.sort_by(2, cx));
                            })))
                            .child(div().w(px(80.)).cursor_pointer().child(
                                if vm.sort_column == 3 {
                                    if vm.sort_ascending { "Ratio ▲" } else { "Ratio ▼" }
                                } else { "Ratio" }
                            ).on_mouse_down(MouseButton::Left, cx.listener(|this: &mut EntryList, _event: &MouseDownEvent, _window: &mut Window, cx| {
                                this.archive_vm.update(cx, |vm, cx| vm.sort_by(3, cx));
                            })))
                            .child(div().w(px(140.)).cursor_pointer().child(
                                if vm.sort_column == 4 {
                                    if vm.sort_ascending { "Date ▲" } else { "Date ▼" }
                                } else { "Date" }
                            ).on_mouse_down(MouseButton::Left, cx.listener(|this: &mut EntryList, _event: &MouseDownEvent, _window: &mut Window, cx| {
                                this.archive_vm.update(cx, |vm, cx| vm.sort_by(4, cx));
                            })))
                    ];
                    // Use displayed_entries() to respect filter
                    for (original_idx, entry) in vm.displayed_entries().iter() {
                        let idx = *original_idx as u32;
                        let selected = vm.selection.contains(&idx);
                        let mut row = div().flex().flex_row().gap_2().px_2().py_1()
                            .cursor_pointer()
                            .on_mouse_down(MouseButton::Left, cx.listener(move |this: &mut EntryList, event: &MouseDownEvent, _window: &mut Window, cx| {
                                this.archive_vm.update(cx, |vm, cx| vm.select(idx, &event.modifiers, cx));
                            }))
                            .child(div().flex_1().child(
                                if entry.is_directory { format!("📁 {}", entry.name) } else { format!("📄 {}", entry.name) }
                            ))
                            .child(div().w(px(80.)).text_sm().child(format_size(entry.size)))
                            .child(div().w(px(80.)).text_sm().child(format_size(entry.compressed_size)))
                            .child(div().w(px(80.)).text_sm().child(format!("{:.0}%", entry.compression_ratio() * 100.)))
                            .child(div().w(px(140.)).text_sm().text_color(cx.global::<Theme>().muted).child(
                                entry.modified.map(|t| t.format("%Y-%m-%d %H:%M").to_string()).unwrap_or_default()
                            ));
                        if selected {
                            row = row.bg(cx.global::<Theme>().selection);
                        } else if original_idx % 2 == 0 {
                            row = row.bg(cx.global::<Theme>().surface);
                        }
                        children.push(row);
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
