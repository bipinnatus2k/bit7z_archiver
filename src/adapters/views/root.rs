use crate::adapters::view_models::archive_vm::ArchiveViewModel;
use crate::adapters::view_models::preview_vm::PreviewViewModel;
use crate::adapters::views::archive_browser::ArchiveBrowser;
use crate::adapters::views::archive_file_list::ArchiveFileList;
use crate::adapters::views::preview_panel::PreviewPanel;
use crate::adapters::views::status_bar::StatusBar;
use crate::adapters::views::toolbar::Toolbar;
use crate::adapters::views::dialogs::extract::{ExtractDialog, ExtractDialogEvent};
use crate::adapters::views::dialogs::create::{CreateArchiveDialog, CreateDialogEvent};
use crate::adapters::views::dialogs::settings::{SettingsDialog, SettingsDialogEvent};
use crate::application::events::ArchiveVmEvent;
use crate::application::extract::ExtractEntriesUseCase;
use crate::domain::archive::*;
use crate::domain::preferences::ThemeMode;
use crate::domain::repository::ArchiveRepository;
use crate::theme::Theme;
use gpui::prelude::FluentBuilder as _;
use gpui::*;
use std::path::Path;
use std::sync::Arc;
use gpui_component::resizable::{h_resizable, resizable_panel, v_resizable};
use gpui_component::Root;

pub struct RootView {
    toolbar: Entity<Toolbar>,
    archive_vm: Entity<ArchiveViewModel>,
    preview_vm: Entity<PreviewViewModel>,
    archive_browser: Entity<ArchiveBrowser>,
    entry_list: Entity<ArchiveFileList>,
    preview_panel: Entity<PreviewPanel>,
    status_bar: Entity<StatusBar>,
    extract_dialog: Option<Entity<ExtractDialog>>,
    settings_dialog: Option<Entity<SettingsDialog>>,
    repo: Arc<dyn ArchiveRepository>,
}

impl RootView {
    pub fn new(window: &mut Window, cx: &mut App, open_path: Option<String>, open_password: Option<String>) -> Entity<Self> {
        cx.new(|cx| {
            let repo = cx.global::<crate::domain::repository::RepoGlobal>().0.clone();
            let archive_vm = cx.new(|cx| ArchiveViewModel::new(cx));
            let preview_vm = cx.new(|cx| PreviewViewModel::new(cx));

            let toolbar = cx.new(|_| Toolbar::new(archive_vm.clone()));
            let archive_browser = cx.new(|cx| ArchiveBrowser::new(archive_vm.clone(), window, cx));
            let entry_list = cx.new(|cx| ArchiveFileList::new(archive_vm.clone(), window, cx));
            let preview_panel = cx.new(|_| PreviewPanel::new(preview_vm.clone()));
            let status_bar = cx.new(|_| StatusBar::new(archive_vm.clone()));

            // Auto-open archive if provided (CLI handoff)
            if let Some(path) = open_path {
                let vm = archive_vm.clone();
                let pw = open_password.clone();
                vm.update(cx, |vm, cx| {
                    vm.open_archive(Path::new(&path), pw, cx);
                });
            }

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
                            this.entry_list.update(cx, |_, cx| cx.notify());
                            cx.notify();
                        }
                        ArchiveVmEvent::RequestShowExtract => {
                            let vm = archive_vm.read(cx);
                            let entries: Vec<ArchiveEntry> = vm.selected_entries();
                            let indices: Vec<u32> = entries.iter().map(|e| e.original_index).collect();
                            let handle = vm.archive.clone();
                            drop(vm);
                            if !entries.is_empty() {
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
                                            if let Some(ref handle) = handle {
                                                let uc = ExtractEntriesUseCase::new(repo.clone());
                                                let _ = uc.execute(handle, &indices, destination);
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
                            cx.spawn(async move |_, cx: &mut AsyncApp| {
                                let _ = cx.open_window(
                                    WindowOptions {
                                        window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                                            point(px(100.), px(100.)),
                                            size(px(560.), px(600.)),
                                        ))),
                                        window_background: WindowBackgroundAppearance::Opaque,
                                        window_decorations: Some(WindowDecorations::Client),
                                        ..Default::default()
                                    },
                                    |window, cx| {
                                        let dialog = cx.new(|cx| CreateArchiveDialog::new(window, cx, vec![]));
                                        cx.new(|cx| gpui_component::Root::new(dialog, window, cx))
                                    }
                                );
                            }).detach();
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

            // Settings dialog subscription is handled in the dialog creation code

            Self {
                toolbar, archive_vm, preview_vm,
                archive_browser, entry_list, preview_panel, status_bar,
                extract_dialog: None,
                settings_dialog: None,
                repo,
            }
        })
    }
}

impl Render for RootView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        gpui_component::v_flex().size_full().relative()
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                let modifiers = event.keystroke.modifiers;
                let key = event.keystroke.key.clone();
                let cmd = modifiers.platform || modifiers.control;
                match key.as_str() {
                    "a" if cmd => {
                        this.archive_vm.update(cx, |vm, cx| vm.select_all(cx));
                    }
                    "o" if cmd => {
                        if let Some(path) = crate::adapters::platform::pick_archive_file() {
                            this.archive_vm.update(cx, |vm, cx| vm.open_archive(&path, None, cx));
                        }
                    }
                    "n" if cmd => {
                        this.archive_vm.update(cx, |vm, cx| vm.request_create(cx));
                    }
                    "e" if cmd => {
                        this.archive_vm.update(cx, |vm, cx| vm.request_extract(cx));
                    }
                    "f5" => {
                        this.archive_vm.update(cx, |vm, cx| vm.refresh(cx));
                    }
                    "Backspace" | "Delete" => {
                        this.archive_vm.update(cx, |vm, cx| vm.delete_selected(cx));
                    }
                    "Escape" => {
                        this.extract_dialog = None;
                        this.settings_dialog = None;
                        cx.notify();
                    }
                    _ => {}
                }
            }))
            .child(self.toolbar.clone())
            .child(div().flex_1().child(
                h_resizable("main-hz")
                    .child(
                        resizable_panel()
                            .size(px(240.))
                            .size_range(px(150.)..px(500.))
                            .flex_none()
                            .child(self.archive_browser.clone())
                    )
                    .child(
                        v_resizable("main-vt")
                            .child(
                                resizable_panel()
                                    .child(self.entry_list.clone())
                            )
                            .child(
                                resizable_panel()
                                    .size(px(200.))
                                    .size_range(px(100.)..px(500.))
                                    .flex_none()
                                    .child(self.preview_panel.clone())
                            )
                    )
            ))
            .child(self.status_bar.clone())
            .when_some(self.extract_dialog.clone(), |el, dialog| {
                el.child(
                    div().absolute().size_full().top(px(0.)).left(px(0.))
                        .bg(hsla(0., 0., 0., 0.2))
                        .flex().items_center().justify_center()
                        .child(dialog)
                )
            })

    }
}
