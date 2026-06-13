use crate::adapters::view_models::archive_vm::{ArchiveViewModel, ViewStatus};
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
        let vm = self.archive_vm.read(cx);
        let has_archive = matches!(vm.status, ViewStatus::Ready);

        gpui_component::h_flex().gap_2().p_2()
            .child(
                Button::new("open")
                    .label("Open")
                    .on_click({
                        let vm = self.archive_vm.clone();
                        move |_, _, cx| {
                            vm.update(cx, |vm, cx| {
                                // TODO: Add native file dialog
                                // For now, show an error encouraging file dialog integration
                                vm.status = ViewStatus::Error("Use Open button with file dialog (WIP)".into());
                                cx.notify();
                            });
                        }
                    })
            )
            .child(
                Button::new("create")
                    .label("Create")
                    .on_click(|_, _, _| {})
            )
            .child(
                Button::new("extract")
                    .label("Extract")
                    .on_click(|_, _, _| {})
            )
            .child(
                Button::new("test")
                    .label("Test")
                    .on_click(|_, _, _| {})
            )
    }
}
