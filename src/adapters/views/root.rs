use crate::adapters::view_models::archive_vm::ArchiveViewModel;
use crate::adapters::view_models::preview_vm::PreviewViewModel;
use crate::adapters::views::archive_browser::ArchiveBrowser;
use crate::adapters::views::archive_file_list::ArchiveFileList;
use crate::adapters::views::menu::Menu;
use crate::adapters::views::preview_panel::PreviewPanel;
use crate::adapters::views::status_bar::StatusBar;
use crate::adapters::views::toolbar::Toolbar;
use crate::adapters::views::dialogs::extract::{ExtractDialog, ExtractDialogEvent};
use crate::adapters::views::dialogs::create::{CreateArchiveDialog, CreateDialogEvent};
use crate::adapters::views::dialogs::settings::{SettingsDialog, SettingsDialogEvent};
use crate::adapters::views::dialogs::add_files::{AddFilesDialog, AddFilesDialogEvent};
use crate::application::events::ArchiveVmEvent;
use crate::application::extract::ExtractEntriesUseCase;
use crate::application::add_to::AddToArchiveUseCase;
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
    menu: Entity<Menu>,
    toolbar: Entity<Toolbar>,
    archive_vm: Entity<ArchiveViewModel>,
    preview_vm: Entity<PreviewViewModel>,
    archive_browser: Entity<ArchiveBrowser>,
    entry_list: Entity<ArchiveFileList>,
    preview_panel: Entity<PreviewPanel>,
    status_bar: Entity<StatusBar>,
    extract_dialog: Option<Entity<ExtractDialog>>,
    repo: Arc<dyn ArchiveRepository>,
}

impl RootView {
    pub fn new(window: &mut Window, cx: &mut App, open_path: Option<String>, open_password: Option<String>) -> Entity<Self> {
        cx.new(|cx| {
            let repo = cx.global::<crate::domain::repository::RepoGlobal>().0.clone();
            let archive_vm = cx.new(|cx| ArchiveViewModel::new(cx));
            let preview_vm = cx.new(|cx| PreviewViewModel::new(cx));

            let menu = cx.new(|_| Menu::new(archive_vm.clone()));
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
                                let dialog = cx.new(|_cx| ExtractDialog::new(entries));
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
                                                let _ = uc.execute(handle, &indices, destination, None);
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
                            let dialog = cx.new(|cx| CreateArchiveDialog::new(cx, vec![]));
                            cx.subscribe::<CreateArchiveDialog, _>(&dialog, move |this: &mut RootView, _, event: &crate::adapters::views::dialogs::create::CreateDialogEvent, cx| {
                                match event {
                                    crate::adapters::views::dialogs::create::CreateDialogEvent::CreateCompleted { success, error } => {
                                        if *success {
                                            this.archive_vm.update(cx, |vm, cx| vm.refresh(cx));
                                        } else if let Some(e) = error {
                                            log::error!("Create archive failed: {}", e);
                                        }
                                        cx.notify();
                                    }
                                    crate::adapters::views::dialogs::create::CreateDialogEvent::Canceled => {
                                        cx.notify();
                                    }
                                    _ => {}
                                }
                            }).detach();
                        }
                        ArchiveVmEvent::RequestTest => {
                            archive_vm.update(cx, |vm, cx| vm.test_all(cx));
                        }
                        ArchiveVmEvent::RequestShowAdd => {
                            let format = ArchiveFormat::SevenZip;
                            let dialog = cx.new(move |cx| {
                                crate::adapters::views::dialogs::add_files::AddFilesDialog::new(cx, format)
                            });
                            let repo = repo.clone();
                            cx.subscribe(&dialog, move |this: &mut RootView, _, event: &crate::adapters::views::dialogs::add_files::AddFilesDialogEvent, cx| {
                                match event {
                                    crate::adapters::views::dialogs::add_files::AddFilesDialogEvent::AddRequested { files, format: _, compression_level: _, encryption: _ } => {
                                        if let Some(ref handle) = this.archive_vm.read(cx).archive {
                                            let uc = crate::application::add_to::AddToArchiveUseCase::new(repo.clone());
                                            let mut handle = handle.clone();
                                            let files = files.clone();
                                            let (tx, rx) = crate::application::progress::progress_channel();
                                            cx.update_global::<crate::adapters::view_models::progress_vm::ProgressState, _>(|state, _cx| {
                                                state.is_active = true;
                                                state.is_complete = false;
                                                state.is_paused = false;
                                                state.receiver = Some(std::sync::Arc::new(std::sync::Mutex::new(rx)));
                                                state.message = format!("Adding {} files...", files.len());
                                                state.current = 0;
                                                state.total = files.len() as u64;
                                                state.error = None;
                                            });
                                            cx.background_spawn(async move {
                                                let _ = uc.execute(&mut handle, &files, Some(tx));
                                            }).detach();
                                        }
                                        cx.notify();
                                    }
                                    crate::adapters::views::dialogs::add_files::AddFilesDialogEvent::Canceled => {
                                        cx.notify();
                                    }
                                }
                            }).detach();
                        }
                        ArchiveVmEvent::RequestShowSettings => {
                            cx.spawn(async move |_, cx: &mut AsyncApp| {
                                let _ = cx.open_window(
                                    WindowOptions {
                                        window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                                            point(px(150.), px(150.)),
                                            size(px(480.), px(500.)),
                                        ))),
                                        window_background: WindowBackgroundAppearance::Opaque,
                                        window_decorations: Some(WindowDecorations::Client),
                                        ..Default::default()
                                    },
                                    |window, cx| {
                                        let settings = cx.new(|cx| {
                                            let prefs = cx.global::<crate::domain::preferences::Preferences>().clone();
                                            crate::adapters::views::dialogs::settings::SettingsDialog {
                                                prefs,
                                                active_tab: crate::adapters::views::dialogs::settings::SettingsTab::General,
                                            }
                                        });
                                        cx.new(|cx| gpui_component::Root::new(settings, window, cx))
                                    }
                                );
                            }).detach();
                        }
                        ArchiveVmEvent::RequestDelete => {
                            archive_vm.update(cx, |vm, cx| vm.delete_selected(cx));
                        }
                        ArchiveVmEvent::RequestAddFiles => {
                            archive_vm.update(cx, |vm, cx| vm.add_files(cx));
                        }
                        ArchiveVmEvent::RequestTestEntries { selected_only } => {
                            if *selected_only {
                                archive_vm.update(cx, |vm, cx| vm.test_selected(cx));
                            } else {
                                archive_vm.update(cx, |vm, cx| vm.test_all(cx));
                            }
                        }
                        ArchiveVmEvent::RequestRename { index, new_name } => {
                            archive_vm.update(cx, |vm, cx| vm.rename_entry(*index, new_name, cx));
                        }
                        ArchiveVmEvent::RequestNewFolder => {
                            archive_vm.update(cx, |vm, cx| vm.request_new_folder(cx));
                        }
                        ArchiveVmEvent::RequestNewFile => {
                            archive_vm.update(cx, |vm, cx| vm.request_new_file(cx));
                        }
                        ArchiveVmEvent::RequestOpenEntry => {
                            archive_vm.update(cx, |vm, cx| vm.open_entry(cx));
                        }
                        ArchiveVmEvent::RequestViewEntry => {
                            archive_vm.update(cx, |vm, cx| vm.preview_entry(cx));
                        }
                        ArchiveVmEvent::RequestEditEntry => {
                            archive_vm.update(cx, |vm, cx| vm.edit_entry(cx));
                        }
                        ArchiveVmEvent::RequestProperties => {
                            archive_vm.update(cx, |vm, cx| vm.show_properties(cx));
                        }
                        ArchiveVmEvent::RequestChecksum { algorithm } => {
                            archive_vm.update(cx, |vm, cx| vm.request_checksum(cx, *algorithm));
                        }
                        ArchiveVmEvent::RefreshListing => {
                            cx.notify();
                        }
                    }
                }
            }).detach();

            // Settings dialog subscription is handled in the dialog creation code

            Self {
                menu, toolbar, archive_vm, preview_vm,
                archive_browser, entry_list, preview_panel, status_bar,
                extract_dialog: None,
                repo,
            }
        })
    }
}

impl Render for RootView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        gpui_component::v_flex().size_full().relative()
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _window, cx| {
                let modifiers = event.keystroke.modifiers;
                let key = event.keystroke.key.clone();
                let cmd = modifiers.platform || modifiers.control;
                let shift = modifiers.shift;
                match key.as_str() {
                    "a" if cmd && !shift => {
                        this.archive_vm.update(cx, |vm, cx| vm.select_all(cx));
                    }
                    "o" if cmd && !shift => {
                        if let Some(path) = crate::adapters::platform::pick_archive_file() {
                            this.archive_vm.update(cx, |vm, cx| vm.open_archive(&path, None, cx));
                        }
                    }
                    "n" if cmd && shift => {
                        this.archive_vm.update(cx, |vm, cx| vm.request_new_folder(cx));
                    }
                    "n" if cmd => {
                        this.archive_vm.update(cx, |vm, cx| vm.request_create(cx));
                    }
                    "e" if cmd => {
                        this.archive_vm.update(cx, |vm, cx| vm.request_extract(cx));
                    }
                    "t" if cmd => {
                        this.archive_vm.update(cx, |vm, cx| vm.request_test(cx));
                    }
                    "v" if cmd => {
                        this.archive_vm.update(cx, |vm, cx| vm.preview_entry(cx));
                    }
                    "f5" => {
                        this.archive_vm.update(cx, |vm, cx| vm.refresh(cx));
                    }
                    "f4" => {
                        this.archive_vm.update(cx, |vm, cx| vm.edit_entry(cx));
                    }
                    "f2" => {
                        this.archive_vm.update(cx, |vm, cx| {
                            if let Some(idx) = vm.first_selected_index() {
                                cx.emit(ArchiveVmEvent::RequestRename { index: idx, new_name: String::new() });
                            }
                        });
                    }
                    "enter" if modifiers.alt => {
                        this.archive_vm.update(cx, |vm, cx| vm.show_properties(cx));
                    }
                    "enter" => {
                        this.archive_vm.update(cx, |vm, cx| vm.open_entry(cx));
                    }
                    "Backspace" | "Delete" => {
                        this.archive_vm.update(cx, |vm, cx| vm.delete_selected(cx));
                    }
                    "Escape" => {
                        this.extract_dialog = None;
                        cx.notify();
                    }
                    _ => {}
                }
            }))
            .child(self.menu.clone())
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
