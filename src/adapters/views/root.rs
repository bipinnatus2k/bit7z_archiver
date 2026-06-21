use crate::adapters::view_models::archive_vm::ArchiveViewModel;
use crate::adapters::views::archive_browser::ArchiveBrowser;
use crate::adapters::views::archive_file_list::ArchiveFileList;
use crate::adapters::views::menu::Menu;
use crate::adapters::views::preview_panel::PreviewPanel;
use crate::adapters::views::status_bar::StatusBar;
use crate::adapters::views::toolbar::{Toolbar, ToolbarIntent};
use crate::adapters::views::dialogs::extract::{ExtractDialog, ExtractDialogEvent};
use crate::adapters::views::dialogs::create::{CreateArchiveDialog, CreateDialogEvent};
use crate::adapters::views::dialogs::password::{PasswordDialog, PasswordDialogEvent};
use crate::adapters::views::dialogs::settings::{SettingsDialog, SettingsDialogEvent};
use crate::adapters::views::dialogs::add_files::{AddFilesDialog, AddFilesDialogEvent};
use crate::adapters::events::ArchiveVmEvent;
use crate::application::extract::ExtractEntriesUseCase;
use crate::application::add_to::AddToArchiveUseCase;
use crate::domain::archive::*;
use crate::domain::preferences::ThemeMode;
use crate::domain::repository::ArchiveRepository;
use crate::gui::IpcReceiver;
use crate::ipc::GuiCommand;
use crate::theme::Theme;
use crossbeam::channel::unbounded;
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
    archive_browser: Entity<ArchiveBrowser>,
    entry_list: Entity<ArchiveFileList>,
    preview_panel: Entity<PreviewPanel>,
    status_bar: Entity<StatusBar>,
    extract_dialog: Option<Entity<ExtractDialog>>,
    password_dialog: Option<Entity<PasswordDialog>>,
    pending_password_path: Option<String>,
    repo: Arc<dyn ArchiveRepository>,
}

impl RootView {
    pub fn new(window: &mut Window, cx: &mut App, open_path: Option<String>, open_password: Option<String>) -> Entity<Self> {
        cx.new(|cx| {
            let repo = cx.global::<crate::gui::RepoGlobal>().0.clone();
            let archive_vm = cx.new(|cx| ArchiveViewModel::new(cx));

            let menu = cx.new(|_| Menu::new(archive_vm.clone()));
            let toolbar = cx.new(|_| Toolbar::new());
            let archive_browser = cx.new(|cx| ArchiveBrowser::new(archive_vm.clone(), window, cx));
            let entry_list = cx.new(|cx| ArchiveFileList::new(archive_vm.clone(), window, cx));
            let preview_panel = cx.new(|_| PreviewPanel::new());
            let status_bar = cx.new(|_| StatusBar::new());

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
                            let h = handle.clone();
                            let idx = *index;
                            this.preview_panel.update(cx, |panel, _| panel.set_loading());
                            cx.notify();
                            let repo = repo.clone();
                            let preview_panel = this.preview_panel.clone();
                            cx.spawn(async move |this, cx| {
                                let use_case = crate::application::preview::PreviewEntryUseCase::new(repo);
                                match use_case.execute(&h, idx, 1_048_576) {
                                    Ok(data) => {
                                        preview_panel.update(cx, |panel, _| panel.set_data(Some(data)));
                                    }
                                    Err(_) => {
                                        preview_panel.update(cx, |panel, _| panel.set_data(None));
                                    }
                                }
                                let _ = this.update(cx, |_, cx| cx.notify());
                            }).detach();
                        }
                        ArchiveVmEvent::SelectionChanged(None) => {
                            this.preview_panel.update(cx, |panel, _| panel.set_data(None));
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
                                        ExtractDialogEvent::ExtractRequested { destination, preserve_paths: _, overwrite_mode, keep_broken } => {
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
                                        let dialog = cx.new(|cx| CreateArchiveDialog::new(cx, vec![]));
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
                                    match repo.test(&handle) {
                                        Ok(result) => {
                                            log::info!("Test completed: {}/{} passed", result.passed, result.total);
                                        }
                                        Err(e) => {
                                            log::error!("Test failed: {}", e);
                                        }
                                    }
                                });
                            }
                        }
                        ArchiveVmEvent::RequestShowAdd => {
                            let vm = archive_vm.read(cx);
                            let format = ArchiveFormat::SevenZip;
                            let is_solid = vm.properties.as_ref().map(|p| p.is_solid).unwrap_or(false);
                            let archive_handle = vm.archive.clone();
                            drop(vm);
                            let repo_clone = repo.clone();
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
                                    move |window, cx| {
                                        let dialog = cx.new(|cx| {
                                            crate::adapters::views::dialogs::add_files::AddFilesDialog::new(
                                                cx,
                                                format,
                                                archive_handle.clone(),
                                                Some(repo_clone.clone()),
                                                is_solid,
                                            )
                                        });
                                        cx.new(|cx| gpui_component::Root::new(dialog, window, cx))
                                    }
                                );
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
                                            let prefs = cx.global::<crate::gui::PreferencesGlobal>().0.clone();
                                            crate::adapters::views::dialogs::settings::SettingsDialog { prefs }
                                        });
                                        cx.subscribe::<crate::adapters::views::dialogs::settings::SettingsDialog, crate::adapters::views::dialogs::settings::SettingsDialogEvent>(&settings, |_this, event, cx| {
                                            match event {
                                                crate::adapters::views::dialogs::settings::SettingsDialogEvent::Saved(prefs) => {
                                                    cx.set_global(crate::gui::PreferencesGlobal(prefs.clone()));
                                                    if let Err(e) = cx.global::<crate::gui::PreferencesRepoGlobal>().0.save(&cx.global::<crate::gui::PreferencesGlobal>().0) {
                                                        log::error!("Failed to save preferences: {}", e);
                                                    }
                                                }
                                                crate::adapters::views::dialogs::settings::SettingsDialogEvent::Canceled => {}
                                            }
                                        }).detach();
                                        cx.new(|cx| gpui_component::Root::new(settings, window, cx))
                                    }
                                );
                            }).detach();
                        }
                        ArchiveVmEvent::RequestPassword { path } => {
                            this.pending_password_path = Some(path.clone());
                            cx.notify();
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

            // Toolbar intent subscription
            cx.subscribe::<Toolbar, ToolbarIntent>(&toolbar, {
                let archive_vm = archive_vm.clone();
                let repo = repo.clone();
                move |this: &mut RootView, _emitter, intent: &ToolbarIntent, cx| {
                    match intent {
                        ToolbarIntent::OpenArchive => {
                            if let Some(path) = crate::adapters::platform::pick_archive_file() {
                                archive_vm.update(cx, |vm, cx| vm.open_archive(&path, None, cx));
                            }
                        }
                        ToolbarIntent::CreateArchive => {
                            archive_vm.update(cx, |vm, cx| vm.request_create(cx));
                        }
                        ToolbarIntent::AddFiles => {
                            archive_vm.update(cx, |vm, cx| vm.request_add_files(cx));
                        }
                        ToolbarIntent::ExtractSelected => {
                            archive_vm.update(cx, |vm, cx| vm.request_extract(cx));
                        }
                        ToolbarIntent::TestArchive => {
                            archive_vm.update(cx, |vm, cx| vm.request_test(cx));
                        }
                        ToolbarIntent::CloseArchive => {
                            archive_vm.update(cx, |vm, cx| vm.close(cx));
                        }
                        ToolbarIntent::ShowSettings => {
                            archive_vm.update(cx, |vm, cx| vm.request_show_settings(cx));
                        }
                    }
                }
            }).detach();

            // Poll IPC commands using a dedicated background thread with std::thread::sleep
            // GPUI does not use tokio, so we cannot use tokio::time::sleep anywhere.
            let archive_vm_ipc = archive_vm.clone();
            let (ipc_cmd_tx, ipc_cmd_rx) = unbounded::<GuiCommand>();
            let ipc_receiver_arc = cx.global::<IpcReceiver>().0.clone();

            // Background thread polls IPC receiver with std::thread::sleep
            std::thread::Builder::new()
                .name("ipc-poll".into())
                .spawn(move || {
                    loop {
                        let cmd = ipc_receiver_arc.lock()
                            .ok()
                            .and_then(|rx| rx.recv().ok());
                        if let Some(cmd) = cmd {
                            let _ = ipc_cmd_tx.send(cmd);
                        } else {
                            std::thread::sleep(std::time::Duration::from_millis(50));
                        }
                    }
                })
                .ok();

            // Main thread processes commands forwarded from background task
            cx.spawn(async move |_, cx| {
                loop {
                    while let Ok(cmd) = ipc_cmd_rx.try_recv() {
                        match cmd {
                            GuiCommand::Open { path, password } => {
                                log::info!("IPC open: {} (password: {:?})", path, password.is_some());
                                archive_vm_ipc.update(cx, |vm, cx| {
                                    vm.open_archive(std::path::Path::new(&path), password, cx);
                                });
                            }
                            GuiCommand::Activate => {
                                log::info!("IPC activate");
                            }
                        }
                    }
                    // Yield to GPUI event loop without using tokio
                    cx.background_spawn(std::future::ready(())).await;
                }
            }).detach();

            // Settings dialog subscription is handled in the dialog creation code

            Self {
                menu, toolbar, archive_vm,
                archive_browser, entry_list, preview_panel, status_bar,
                extract_dialog: None,
                password_dialog: None,
                pending_password_path: None,
                repo,
            }
        })
    }
}

impl Render for RootView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Create password dialog if pending (only once)
        if self.password_dialog.is_none() {
            if let Some(path) = self.pending_password_path.take() {
                let dialog_path = path.clone();
                let dialog = cx.new(|cx| PasswordDialog::new(dialog_path, window, cx));
                let archive_vm = self.archive_vm.clone();
                cx.subscribe::<PasswordDialog, PasswordDialogEvent>(&dialog, move |this: &mut RootView, _emitter, event: &PasswordDialogEvent, cx| {
                    match event {
                        PasswordDialogEvent::Submitted(password) => {
                            this.password_dialog = None;
                            archive_vm.update(cx, |vm, cx| {
                                vm.open_archive(std::path::Path::new(&path), Some(password.clone()), cx);
                            });
                            cx.notify();
                        }
                        PasswordDialogEvent::Canceled => {
                            this.password_dialog = None;
                            cx.notify();
                        }
                    }
                }).detach();
                self.password_dialog = Some(dialog);
            }
        }

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
            .when_some(self.password_dialog.clone(), |el, dialog| {
                el.child(
                    div().absolute().size_full().top(px(0.)).left(px(0.))
                        .bg(hsla(0., 0., 0., 0.2))
                        .flex().items_center().justify_center()
                        .child(dialog)
                )
            })

    }
}
