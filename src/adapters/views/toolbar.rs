use crate::adapters::view_models::archive_vm::ArchiveViewModel;
use gpui::*;
use gpui_component::button::Button;

pub struct Toolbar {
    archive_vm: Entity<ArchiveViewModel>,
}

impl Toolbar {
    pub fn new(archive_vm: Entity<ArchiveViewModel>) -> Self {
        Self { archive_vm }
    }
}

impl Render for Toolbar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {

        gpui_component::h_flex().gap_2().p_2()
            .child(
                Button::new("open")
                    .label("Open")
                    .on_click({
                        let vm = self.archive_vm.clone();
                        move |_, _, cx| {
                            if let Some(path) = crate::adapters::platform::pick_archive_file() {
                                vm.update(cx, |vm, cx| {
                                    vm.open_archive(&path, None, cx);
                                });
                            }
                        }
                    })
            )
            .child(
                Button::new("create")
                    .label("Create")
                    .on_click({
                        let vm = self.archive_vm.clone();
                        move |_, _, cx| {
                            vm.update(cx, |vm, cx| vm.request_create(cx));
                        }
                    })
            )
            .child(
                Button::new("extract")
                    .label("Extract")
                    .on_click({
                        let vm = self.archive_vm.clone();
                        move |_, _, cx| {
                            vm.update(cx, |vm, cx| vm.request_extract(cx));
                        }
                    })
            )
            .child(
                Button::new("test")
                    .label("Test")
                    .on_click({
                        let vm = self.archive_vm.clone();
                        move |_, _, cx| {
                            vm.update(cx, |vm, cx| vm.request_test(cx));
                        }
                    })
            )
            .child(
                Button::new("close")
                    .label("Close")
                    .on_click({
                        let vm = self.archive_vm.clone();
                        move |_, _, cx| {
                            vm.update(cx, |vm, cx| vm.close(cx));
                        }
                    })
            )
    }
}

// ---------------------------------------------------------------------------
// Raw Win32 file dialog via GetOpenFileNameW (avoids windows-sys feature deps)
