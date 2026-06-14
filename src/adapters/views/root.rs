use crate::adapters::view_models::archive_vm::ArchiveViewModel;
use crate::adapters::view_models::preview_vm::PreviewViewModel;
use crate::adapters::views::archive_browser::ArchiveBrowser;
use crate::adapters::views::entry_list::EntryList;
use crate::adapters::views::preview_panel::PreviewPanel;
use crate::adapters::views::status_bar::StatusBar;
use crate::adapters::views::toolbar::Toolbar;
use crate::adapters::views::dialogs::extract::{ExtractDialog, ExtractDialogEvent};
use crate::adapters::views::dialogs::create::{CreateArchiveDialog, CreateDialogEvent};
use crate::domain::archive::*;
use crate::domain::repository::ArchiveRepository;
use gpui::prelude::FluentBuilder as _;
use gpui::*;
use std::sync::Arc;

pub struct RootView {
    toolbar: Entity<Toolbar>,
    archive_vm: Entity<ArchiveViewModel>,
    preview_vm: Entity<PreviewViewModel>,
    archive_browser: Entity<ArchiveBrowser>,
    entry_list: Entity<EntryList>,
    preview_panel: Entity<PreviewPanel>,
    status_bar: Entity<StatusBar>,
    extract_dialog: Option<Entity<ExtractDialog>>,
    create_dialog: Option<Entity<CreateArchiveDialog>>,
    repo: Arc<dyn ArchiveRepository>,
}

impl RootView {
    pub fn new(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            let repo = cx.global::<crate::domain::repository::RepoGlobal>().0.clone();
            let archive_vm = cx.new(|cx| ArchiveViewModel::new(cx));
            let preview_vm = cx.new(|cx| PreviewViewModel::new(cx));

            let toolbar = cx.new(|_| Toolbar::new(archive_vm.clone()));
            let archive_browser = cx.new(|cx| ArchiveBrowser::new(archive_vm.clone(), window, cx));
            let entry_list = cx.new(|_| EntryList { archive_vm: archive_vm.clone() });
            let preview_panel = cx.new(|_| PreviewPanel::new(preview_vm.clone()));
            let status_bar = cx.new(|_| StatusBar::new(archive_vm.clone()));

            cx.subscribe::<ArchiveViewModel, ArchiveVmEvent>(&archive_vm, {
                let archive_vm = archive_vm.clone();
                let repo = repo.clone();
                move |this: &mut RootView, _src, event: &ArchiveVmEvent, cx| {
                    match event {
                        ArchiveVmEvent::SelectionChanged(Some((handle, index))) => {
                            this.preview_vm.update(cx, |vm, cx| vm.load(handle.clone(), *index, cx));
                        }
                        ArchiveVmEvent::SelectionChanged(None) => {
                            this.preview_vm.update(cx, |vm, cx| vm.clear(cx));
                        }
                        ArchiveVmEvent::RequestShowExtract => {
                            let vm = archive_vm.read(cx);
                            let indices: Vec<u32> = vm.selection.iter().copied().collect();
                            let entries: Vec<ArchiveEntry> = indices.iter()
                                .filter_map(|i| vm.entries.get(*i as usize).cloned())
                                .collect();
                            let handle = vm.archive.clone();
                            drop(vm);
                            if !entries.is_empty() {
                                let entries_clone = entries.clone();
                            let entries_count = entries.len();
                                let dialog = cx.new(|_cx| ExtractDialog { entries, destination: String::new(), preserve_paths: true, entries_count });
                                let repo = repo.clone();
                                cx.subscribe::<ExtractDialog, ExtractDialogEvent>(&dialog, move |this: &mut RootView, _, event: &ExtractDialogEvent, cx| {
                                    match event {
                                        ExtractDialogEvent::Canceled => {
                                            this.extract_dialog = None;
                                            cx.notify();
                                        }
                                        ExtractDialogEvent::ExtractRequested { destination, .. } => {
                                            // Perform extraction
                                            if let Some(ref handle) = handle {
                                                let indices: Vec<u32> = entries_clone.iter().enumerate().map(|(i, _)| i as u32).collect();
                                                let _ = repo.extract(handle, &indices, destination);
                                            }
                                            this.extract_dialog = None;
                                            cx.notify();
                                        }
                                    }
                                }).detach();
                                this.extract_dialog = Some(dialog);
                                cx.notify();
                            }
                        }
                        ArchiveVmEvent::RequestShowCreate => {
                            let repo = repo.clone();
                            let dialog = cx.new(|cx| CreateArchiveDialog::new(cx, vec![]));
                            cx.subscribe::<CreateArchiveDialog, CreateDialogEvent>(&dialog, move |this: &mut RootView, _, event: &CreateDialogEvent, cx| {
                                match event {
                                    CreateDialogEvent::Canceled => {
                                        this.create_dialog = None;
                                        cx.notify();
                                    }
                                    CreateDialogEvent::CreateRequested(input) => {
                                        // Create archive (file addition coming later)
                                        let _ = repo.create(&input.destination, input.format, input.encryption.as_ref());
                                        this.create_dialog = None;
                                        cx.notify();
                                    }
                                }
                            }).detach();
                            this.create_dialog = Some(dialog);
                            cx.notify();
                        }
                        ArchiveVmEvent::RequestTest => {
                            let vm = archive_vm.read(cx);
                            if let Some(ref archive) = vm.archive {
                                let handle = archive.clone();
                                let repo = repo.clone();
                                drop(vm);
                                std::thread::spawn(move || {
                                    let _result = repo.test(&handle);
                                });
                            }
                        }
                    }
                }
            }).detach();

            Self {
                toolbar, archive_vm, preview_vm,
                archive_browser, entry_list, preview_panel, status_bar,
                extract_dialog: None,
                create_dialog: None,
                repo,
            }
        })
    }
}

pub enum ArchiveVmEvent {
    SelectionChanged(Option<(ArchiveHandle, u32)>),
    RequestShowExtract,
    RequestShowCreate,
    RequestTest,
}

impl EventEmitter<ArchiveVmEvent> for RootView {}

impl Render for RootView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        gpui_component::v_flex().size_full().relative()
            .child(self.toolbar.clone())
            .child(gpui_component::h_flex().flex_1()
                .child(self.archive_browser.clone())
                .child(gpui_component::v_flex().flex_1()
                    .child(self.entry_list.clone())
                    .child(self.preview_panel.clone())
                )
            )
            .child(self.status_bar.clone())
            .when_some(self.extract_dialog.clone(), |el, dialog| {
                el.child(
                    div().absolute().size_full().top(px(0.)).left(px(0.))
                        .bg(hsla(0., 0., 0., 0.2))
                        .flex().items_center().justify_center()
                        .child(dialog)
                )
            })
            .when_some(self.create_dialog.clone(), |el, dialog| {
                el.child(
                    div().absolute().size_full().top(px(0.)).left(px(0.))
                        .bg(hsla(0., 0., 0., 0.2))
                        .flex().items_center().justify_center()
                        .child(dialog)
                )
            })
    }
}
