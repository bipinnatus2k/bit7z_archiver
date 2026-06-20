use crate::adapters::view_models::archive_vm::{ArchiveViewModel, ViewStatus};
use gpui::*;
use gpui_component::button::Button;
use gpui_component::Disableable;

pub struct Toolbar {
    archive_vm: Entity<ArchiveViewModel>,
}

impl Toolbar {
    pub fn new(archive_vm: Entity<ArchiveViewModel>) -> Self {
        Self { archive_vm }
    }
}

impl Render for Toolbar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let vm = self.archive_vm.read(cx);
        let is_open = vm.archive.is_some();
        let is_ready = matches!(vm.status, ViewStatus::Ready);
        let has_selection = !vm.selection.is_empty();
        drop(vm);

        let window_width = window.bounds().size.width;
        let compact = window_width < px(640.);
        let show_labels = !compact;

        let mut row = gpui_component::h_flex().gap_2().p_2().w_full();

        // Open
        let open_btn = Button::new("open").on_click({
            let vm = self.archive_vm.clone();
            move |_, _, cx| {
                if let Some(path) = crate::adapters::platform::pick_archive_file() {
                    vm.update(cx, |vm, cx| { vm.open_archive(&path, None, cx); });
                }
            }
        });
        let open_btn = if show_labels { open_btn.label("Open") } else { open_btn };
        row = row.child(open_btn);

        // Create
        let create_btn = Button::new("create").on_click({
            let vm = self.archive_vm.clone();
            move |_, _, cx| { vm.update(cx, |vm, cx| vm.request_create(cx)); }
        });
        let create_btn = if show_labels { create_btn.label("Create") } else { create_btn };
        row = row.child(create_btn);

        // Add
        let add_btn = Button::new("add").disabled(!is_open).on_click({
            let vm = self.archive_vm.clone();
            move |_, _, cx| { vm.update(cx, |vm, cx| vm.request_add_files(cx)); }
        });
        let add_btn = if show_labels { add_btn.label("Add") } else { add_btn };
        row = row.child(add_btn);

        // Extract
        let extract_btn = Button::new("extract").disabled(!is_ready || !has_selection).on_click({
            let vm = self.archive_vm.clone();
            move |_, _, cx| { vm.update(cx, |vm, cx| vm.request_extract(cx)); }
        });
        let extract_btn = if show_labels { extract_btn.label("Extract") } else { extract_btn };
        row = row.child(extract_btn);

        // Test
        let test_btn = Button::new("test").disabled(!is_open).on_click({
            let vm = self.archive_vm.clone();
            move |_, _, cx| { vm.update(cx, |vm, cx| vm.request_test(cx)); }
        });
        let test_btn = if show_labels { test_btn.label("Test") } else { test_btn };
        row = row.child(test_btn);

        // Close
        let close_btn = Button::new("close").disabled(!is_open).on_click({
            let vm = self.archive_vm.clone();
            move |_, _, cx| { vm.update(cx, |vm, cx| vm.close(cx)); }
        });
        let close_btn = if show_labels { close_btn.label("Close") } else { close_btn };
        row = row.child(close_btn);

        row.child(div().flex_1())
            .child(
                Button::new("settings").on_click({
                    let vm = self.archive_vm.clone();
                    move |_, _, cx| { vm.update(cx, |vm, cx| vm.request_show_settings(cx)); }
                })
            )
    }
}
