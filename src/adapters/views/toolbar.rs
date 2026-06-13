use crate::adapters::view_models::archive_vm::ArchiveViewModel;
use gpui::*;

pub struct Toolbar;
impl Toolbar {
    pub fn new(_archive_vm: Entity<ArchiveViewModel>) -> impl IntoElement {
        div().flex().flex_row().gap_2().p_2().border_b_1()
            .child(div().px_2().py_1().rounded_md().cursor_pointer().child("Open"))
            .child(div().px_2().py_1().rounded_md().cursor_pointer().child("Create"))
            .child(div().px_2().py_1().rounded_md().cursor_pointer().child("Extract"))
            .child(div().px_2().py_1().rounded_md().cursor_pointer().child("Test"))
    }
}
