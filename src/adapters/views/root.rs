use crate::adapters::view_models::archive_vm::ArchiveViewModel;
use crate::adapters::view_models::preview_vm::PreviewViewModel;
use crate::adapters::views::archive_browser::ArchiveBrowser;
use crate::adapters::views::entry_list::EntryList;
use crate::adapters::views::preview_panel::PreviewPanel;
use crate::adapters::views::status_bar::StatusBar;
use crate::adapters::views::toolbar::Toolbar;
use crate::domain::archive::*;
use gpui::*;

pub struct RootView {
    archive_vm: Entity<ArchiveViewModel>,
    preview_vm: Entity<PreviewViewModel>,
    archive_browser: Entity<ArchiveBrowser>,
    entry_list: Entity<EntryList>,
    preview_panel: Entity<PreviewPanel>,
    status_bar: Entity<StatusBar>,
}

impl RootView {
    pub fn new(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            let archive_vm = cx.new(|cx| ArchiveViewModel::new(cx));
            let preview_vm = cx.new(|cx| PreviewViewModel::new(cx));

            let archive_browser = cx.new(|_| ArchiveBrowser::new(archive_vm.clone()));
            let entry_list = cx.new(|_| EntryList { archive_vm: archive_vm.clone() });
            let preview_panel = cx.new(|_| PreviewPanel::new(preview_vm.clone()));
            let status_bar = cx.new(|_| StatusBar::new(archive_vm.clone()));

            cx.subscribe::<ArchiveViewModel, ArchiveVmEvent>(&archive_vm, |this: &mut Self, src, event: &ArchiveVmEvent, cx| {
                match event {
                    ArchiveVmEvent::SelectionChanged(Some((handle, index))) => {
                        this.preview_vm.update(cx, |vm, cx| vm.load(handle.clone(), *index, cx));
                    }
                    ArchiveVmEvent::SelectionChanged(None) => {
                        this.preview_vm.update(cx, |vm, cx| vm.clear(cx));
                    }
                }
            }).detach();

            Self { archive_vm, preview_vm, archive_browser, entry_list, preview_panel, status_bar }
        })
    }
}

pub enum ArchiveVmEvent {
    SelectionChanged(Option<(ArchiveHandle, u32)>),
}

impl EventEmitter<ArchiveVmEvent> for RootView {}

impl Render for RootView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().flex().flex_col().size_full()
            .child(Toolbar::new(self.archive_vm.clone()))
            .child(div().flex().flex_row().flex_1()
                .child(self.archive_browser.clone())
                .child(div().flex().flex_col().flex_1()
                    .child(self.entry_list.clone())
                    .child(self.preview_panel.clone())
                )
            )
            .child(self.status_bar.clone())
    }
}



