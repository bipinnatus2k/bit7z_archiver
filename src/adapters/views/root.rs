use crate::adapters::view_models::archive_state::{ArchiveState, ViewStatus, KeyModifiers};
use crate::adapters::view_models::archive_vm::ArchiveViewModel;
use crate::adapters::views::archive_browser::{ArchiveBrowser, BrowserIntent};
use crate::adapters::views::archive_file_list::{ArchiveFileList, FileListIntent};
use crate::adapters::views::menu::{Menu, MenuIntent};
use crate::adapters::views::preview_panel::PreviewPanel;
use crate::adapters::views::status_bar::StatusBar;
use crate::adapters::views::toolbar::{Toolbar, ToolbarIntent};
use crate::adapters::views::root_controller::RootController;
use crate::adapters::views::dialogs::extract::ExtractDialog;
use crate::adapters::views::dialogs::password::PasswordDialog;
use crate::adapters::views::dialogs::create::CreateArchiveDialog;
use crate::adapters::views::dialogs::settings::SettingsDialog;
use crate::adapters::views::dialogs::add_files::AddFilesDialog;
use crate::application::extract::ExtractEntriesUseCase;
use crate::adapters::events::ArchiveVmEvent;
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
    pending_password_path: Option<String>,
    repo: Arc<dyn ArchiveRepository>,
    state: ArchiveState,
    controller: RootController,
}

impl RootView {
    pub fn new(window: &mut Window, cx: &mut App, open_path: Option<String>, open_password: Option<String>) -> Entity<Self> {
        cx.new(|cx| {
            let repo = cx.global::<crate::gui::RepoGlobal>().0.clone();
            let archive_vm = cx.new(|cx| ArchiveViewModel::new(cx));

            let menu = cx.new(|_| Menu::new());
            let toolbar = cx.new(|_| Toolbar::new());
            let archive_browser = cx.new(|cx| ArchiveBrowser::new(window, cx));
            let entry_list = cx.new(|cx| ArchiveFileList::new(window, cx));
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
                            let entries = vm.selected_entries();
                            let indices: Vec<u32> = vm.selection.iter().copied().collect();
                            let handle = vm.archive.clone();
                            let r = repo.clone();
                            drop(vm);
                            if !entries.is_empty() {
                                cx.spawn(async move |_, cx| {
                                    let rx = ExtractDialog::open(entries, cx);
                                    use crossbeam::channel::TryRecvError;
                                    loop {
                                        match rx.try_recv() {
                                            Ok(evt) => {
                                                match evt {
                                                    crate::adapters::views::dialogs::extract::ExtractDialogEvent::ExtractRequested { destination, .. } => {
                                                        if let Some(ref h) = handle {
                                                            let uc = ExtractEntriesUseCase::new(r.clone());
                                                            let _ = uc.execute(h, &indices, &destination);
                                                        }
                                                    }
                                                    _ => {}
                                                }
                                                break;
                                            }
                                            Err(TryRecvError::Empty) => {
                                                cx.background_spawn(std::future::ready(())).await;
                                            }
                                            Err(TryRecvError::Disconnected) => break,
                                        }
                                    }
                                }).detach();
                            }
                        }
                        ArchiveVmEvent::RequestShowCreate => {
                            cx.spawn(async move |_, cx| {
                                CreateArchiveDialog::open(cx, vec![]);
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
                            cx.spawn(async move |_, cx| {
                                AddFilesDialog::open(cx, format, archive_handle, None, is_solid);
                            }).detach();
                        }
                        ArchiveVmEvent::RequestShowSettings => {
                            let prefs_repo = cx.global::<crate::gui::PreferencesRepoGlobal>().0.clone();
                            cx.spawn(async move |_, cx| {
                                let rx = SettingsDialog::open(cx);
                                use crossbeam::channel::TryRecvError;
                                loop {
                                    match rx.try_recv() {
                                        Ok(evt) => {
                                            if let crate::adapters::views::dialogs::settings::SettingsDialogEvent::Saved(prefs) = evt {
                                                let _ = cx.update_global::<crate::gui::PreferencesGlobal, _>(|g, app| {
                                                    g.0 = prefs.clone();
                                                    let current = app.global::<crate::gui::PreferencesGlobal>().0.clone();
                                                    if let Err(e) = prefs_repo.save(&current) {
                                                        log::error!("Failed to save preferences: {}", e);
                                                    }
                                                });
                                            }
                                            break;
                                        }
                                        Err(TryRecvError::Empty) => {
                                            cx.background_spawn(std::future::ready(())).await;
                                        }
                                        Err(TryRecvError::Disconnected) => break,
                                    }
                                }
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

            // Menu intent subscription
            cx.subscribe::<Menu, MenuIntent>(&menu, {
                let archive_vm = archive_vm.clone();
                let repo = repo.clone();
                move |this: &mut RootView, _emitter, intent: &MenuIntent, cx| {
                    match intent {
                        MenuIntent::OpenArchive => {
                            if let Some(path) = crate::adapters::platform::pick_archive_file() {
                                archive_vm.update(cx, |vm, cx| vm.open_archive(&path, None, cx));
                            }
                        }
                        MenuIntent::CreateArchive => {
                            archive_vm.update(cx, |vm, cx| vm.request_create(cx));
                        }
                        MenuIntent::AddFiles => {
                            archive_vm.update(cx, |vm, cx| vm.request_add_files(cx));
                        }
                        MenuIntent::TestSelected => {
                            archive_vm.update(cx, |vm, cx| vm.test_selected(cx));
                        }
                        MenuIntent::TestAll => {
                            archive_vm.update(cx, |vm, cx| vm.test_all(cx));
                        }
                        MenuIntent::CloseArchive => {
                            archive_vm.update(cx, |vm, cx| vm.close(cx));
                        }
                        MenuIntent::ShowProperties => {
                            archive_vm.update(cx, |vm, cx| vm.show_properties(cx));
                        }
                        MenuIntent::SelectAll => {
                            this.state.select_all();
                            this.sync_children(cx);
                        }
                        MenuIntent::InvertSelection => {
                            this.state.invert_selection();
                            this.sync_children(cx);
                        }
                        MenuIntent::DeleteSelected => {
                            archive_vm.update(cx, |vm, cx| vm.delete_selected(cx));
                        }
                        MenuIntent::RenameSelected => {
                            archive_vm.update(cx, |vm, cx| {
                                if let Some(idx) = vm.first_selected_index() {
                                    cx.emit(crate::adapters::events::ArchiveVmEvent::RequestRename {
                                        index: idx,
                                        new_name: String::new(),
                                    });
                                }
                            });
                        }
                        MenuIntent::Checksum(algo) => {
                            archive_vm.update(cx, |vm, cx| vm.request_checksum(cx, *algo));
                        }
                        MenuIntent::ShowSettings => {
                            archive_vm.update(cx, |vm, cx| vm.request_show_settings(cx));
                        }
                        MenuIntent::About => {
                            log::info!("bit7z Archiver {}", env!("CARGO_PKG_VERSION"));
                        }
                    }
                }
            }).detach();

            // Browser intent subscription
            cx.subscribe::<ArchiveBrowser, BrowserIntent>(&archive_browser, {
                let archive_vm = archive_vm.clone();
                move |this: &mut RootView, _emitter, intent: &BrowserIntent, cx| {
                    match intent {
                        BrowserIntent::NavigateInto(dir) => {
                            this.state.navigate_into(dir);
                            this.sync_children(cx);
                            this.load_current_directory(cx);
                        }
                        BrowserIntent::SetFilter(text) => {
                            this.state.set_filter(text);
                            this.sync_children(cx);
                        }
                        BrowserIntent::OpenRecentFile(path) => {
                            archive_vm.update(cx, |vm, cx| vm.open_archive(std::path::Path::new(path), None, cx));
                        }
                    }
                }
            }).detach();

            // FileList intent subscription — uses ArchiveState for pure state ops
            cx.subscribe::<ArchiveFileList, FileListIntent>(&entry_list, {
                let archive_vm = archive_vm.clone();
                move |this: &mut RootView, _emitter, intent: &FileListIntent, cx| {
                    match intent {
                        FileListIntent::RowClicked(row, mods) => {
                            let km = KeyModifiers { shift: mods.shift, control: mods.control, platform: mods.platform };
                            this.state.update_selection(*row, km);
                            this.sync_children(cx);
                            // Trigger preview
                            if let Some(idx) = this.state.first_selected_index() {
                                if let Some(ref archive) = this.state.archive {
                                    let repo = this.controller.repo();
                                    let panel = this.preview_panel.clone();
                                    let h = archive.clone();
                                    cx.spawn(async move |this, cx| {
                                        panel.update(cx, |p, _| p.set_loading());
                                        let uc = crate::application::preview::PreviewEntryUseCase::new(repo);
                                        match uc.execute(&h, idx, 1_048_576) {
                                            Ok(data) => { panel.update(cx, |p, _| p.set_data(Some(data))); }
                                            Err(_) => { panel.update(cx, |p, _| p.set_data(None)); }
                                        }
                                        let _ = this.update(cx, |_, cx| cx.notify());
                                    }).detach();
                                }
                            }
                        }
                        FileListIntent::SortByColumn(col, asc) => {
                            this.state.apply_sort(*col, *asc);
                            this.sync_children(cx);
                        }
                        FileListIntent::NavigateUp => {
                            this.state.navigate_up();
                            this.sync_children(cx);
                            this.load_current_directory(cx);
                        }
                        FileListIntent::SelectAll => {
                            this.state.select_all();
                            this.sync_children(cx);
                        }
                        FileListIntent::ClearSelection => {
                            this.state.clear_selection();
                            this.sync_children(cx);
                        }
                        FileListIntent::Refresh => {
                            archive_vm.update(cx, |vm, cx| vm.refresh(cx));
                        }
                        // Complex ops still delegate to archive_vm
                        FileListIntent::OpenEntry => {
                            archive_vm.update(cx, |vm, cx| vm.open_entry(cx));
                        }
                        FileListIntent::PreviewEntry => {
                            archive_vm.update(cx, |vm, cx| vm.preview_entry(cx));
                        }
                        FileListIntent::ExtractSelected => {
                            archive_vm.update(cx, |vm, cx| vm.request_extract(cx));
                        }
                        FileListIntent::RenameEntry(idx) => {
                            archive_vm.update(cx, |vm, cx| {
                                let idx = if *idx == 0 { vm.first_selected_index().unwrap_or(0) } else { *idx };
                                cx.emit(crate::adapters::events::ArchiveVmEvent::RequestRename {
                                    index: idx,
                                    new_name: String::new(),
                                });
                            });
                        }
                        FileListIntent::DeleteSelected => {
                            archive_vm.update(cx, |vm, cx| vm.delete_selected(cx));
                        }
                        FileListIntent::Checksum(algo) => {
                            archive_vm.update(cx, |vm, cx| vm.request_checksum(cx, *algo));
                        }
                        FileListIntent::ShowProperties => {
                            archive_vm.update(cx, |vm, cx| vm.show_properties(cx));
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

            let controller_repo = repo.clone();
            Self {
                menu, toolbar, archive_vm,
                archive_browser, entry_list, preview_panel, status_bar,
                pending_password_path: None,
                repo,
                state: ArchiveState::new(),
                controller: RootController::new(controller_repo),
            }
        })
    }

    fn sync_children(&mut self, cx: &mut Context<Self>) {
        let entries = self.state.displayed_entries().to_vec();
        let selection = self.state.selection.clone();
        let status = self.state.status.clone();
        let path = self.state.current_path.clone();
        let is_ready = self.state.is_ready();
        let has_sel = self.state.has_selection();
        let single = self.state.selection.len() == 1;
        let is_open = self.state.archive.is_some();
        let subdirs = self.state.filtered_subdirs();
        let status_text = self.state.status_text();

        self.entry_list.update(cx, |c, _| c.set_state(entries, selection, status, path));
        self.toolbar.update(cx, |c, _| c.set_state(is_open, is_ready, has_sel));
        self.menu.update(cx, |c, _| c.set_state(is_open, has_sel, single));
        self.archive_browser.update(cx, |c, _| c.set_state(subdirs, vec![]));
        self.status_bar.update(cx, |c, _| c.set_status(&status_text));
    }

    fn load_current_directory(&mut self, cx: &mut Context<Self>) {
        let handle = self.state.archive.clone();
        let path = self.state.current_path.clone();
        let controller = self.controller.repo();
        let entry_list = self.entry_list.clone();

        cx.spawn(async move |this, cx| {
            if let Some(ref h) = handle {
                match controller.list_directory(h, &path) {
                    Ok(entries) => {
                        this.update(cx, |this, cx| {
                            this.state.directory_cache.insert(path.clone(), entries);
                            this.state.navigate_root();  // re-apply filter
                            this.state.status = ViewStatus::Ready;
                            this.sync_children(cx);
                            cx.notify();
                        });
                    }
                    Err(_) => {}
                }
            }
        }).detach();
    }
}

impl Render for RootView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Open password dialog as independent window if pending
        if let Some(path) = self.pending_password_path.take() {
            let p = path.clone();
            let archive_vm = self.archive_vm.clone();
            cx.spawn(async move |this, cx| {
                let rx = PasswordDialog::open(path, cx);
                use crossbeam::channel::TryRecvError;
                loop {
                    match rx.try_recv() {
                        Ok(result) => {
                            use crate::adapters::views::dialogs::password::PasswordResult;
                            match result {
                                PasswordResult::Submitted(pw) => {
                                    archive_vm.update(cx, |vm, cx| {
                                        vm.open_archive(std::path::Path::new(&p), Some(pw), cx);
                                    });
                                }
                                PasswordResult::Canceled => {}
                            }
                            let _ = this.update(cx, |_, cx| cx.notify());
                            break;
                        }
                        Err(TryRecvError::Empty) => {
                            cx.background_spawn(std::future::ready(())).await;
                        }
                        Err(TryRecvError::Disconnected) => break,
                    }
                }
            }).detach();
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
                    "Escape" => {}
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


    }
}
