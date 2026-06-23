use crate::adapters::view_models::archive_state::{ArchiveState, ViewStatus, KeyModifiers};
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
use crate::adapters::views::dialogs::checksum::ChecksumDialog;
use crate::adapters::views::dialogs::delete::DeleteDialog;
use crate::adapters::views::dialogs::test::TestDialog;

impl EventEmitter<ArchiveVmEvent> for RootView {}
use crate::domain::archive::*;
use crate::domain::preferences::ThemeMode;
use crate::domain::repository::{ArchiveError, ArchiveRepository};
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

            let menu = cx.new(|_| Menu::new());
            let toolbar = cx.new(|_| Toolbar::new());
            let archive_browser = cx.new(|cx| ArchiveBrowser::new(window, cx));
            let entry_list = cx.new(|cx| ArchiveFileList::new(window, cx));
            let preview_panel = cx.new(|_| PreviewPanel::new());
            let status_bar = cx.new(|_| StatusBar::new());

            // Auto-open archive if provided (CLI handoff)
            let deferred_open = open_path.map(|path| {
                let pw = open_password.clone();
                let p = path;
                move |this: &mut RootView, cx: &mut Context<RootView>| {
                    this.handle_open_archive(std::path::Path::new(&p), pw, cx);
                }
            });

            // Toolbar intent subscription
            cx.subscribe::<Toolbar, ToolbarIntent>(&toolbar, {
                move |this: &mut RootView, _emitter, intent: &ToolbarIntent, cx| {
                    match intent {
                        ToolbarIntent::OpenArchive => {
                            if let Some(path) = crate::adapters::platform::pick_archive_file() {
                                this.handle_open_archive(&path, None, cx);
                            }
                        }
                        ToolbarIntent::CreateArchive => { cx.emit(ArchiveVmEvent::RequestShowCreate); }
                        ToolbarIntent::AddFiles => { cx.emit(ArchiveVmEvent::RequestShowAdd); }
                        ToolbarIntent::ExtractSelected => { cx.emit(ArchiveVmEvent::RequestShowExtract); }
                        ToolbarIntent::TestArchive => {
                            let handle = this.state.archive.clone();
                            let repo = this.controller.repo();
                            cx.spawn(async move |_, cx| {
                                if let Some(h) = handle {
                                    crate::adapters::views::dialogs::test::TestDialog::open_with_entries(cx, h, repo);
                                }
                            }).detach();
                        }
                        ToolbarIntent::CloseArchive => {
                            if let Some(h) = this.state.archive.take() { this.controller.close_archive(h); }
                            this.state = ArchiveState::new();
                            this.sync_children(cx);
                        }
                        ToolbarIntent::ShowSettings => { cx.emit(ArchiveVmEvent::RequestShowSettings); }
                    }
                }
            }).detach();

            // Menu intent subscription
            cx.subscribe::<Menu, MenuIntent>(&menu, {
                move |this: &mut RootView, _emitter, intent: &MenuIntent, cx| {
                    match intent {
                        MenuIntent::OpenArchive => {
                            if let Some(path) = crate::adapters::platform::pick_archive_file() {
                                this.handle_open_archive(&path, None, cx);
                            }
                        }
                        MenuIntent::CreateArchive => { cx.emit(ArchiveVmEvent::RequestShowCreate); }
                        MenuIntent::AddFiles => { cx.emit(ArchiveVmEvent::RequestShowAdd); }
                        MenuIntent::TestSelected => {
                            let handle = this.state.archive.clone();
                            let repo = this.controller.repo();
                            cx.spawn(async move |_, cx| {
                                if let Some(h) = handle {
                                    crate::adapters::views::dialogs::test::TestDialog::open_with_entries(cx, h, repo);
                                }
                            }).detach();
                        }
                        MenuIntent::TestAll => {
                            let handle = this.state.archive.clone();
                            let repo = this.controller.repo();
                            cx.spawn(async move |_, cx| {
                                if let Some(h) = handle {
                                    crate::adapters::views::dialogs::test::TestDialog::open_with_entries(cx, h, repo);
                                }
                            }).detach();
                        }
                        MenuIntent::CloseArchive => {
                            if let Some(h) = this.state.archive.take() { this.controller.close_archive(h); }
                            this.state = ArchiveState::new();
                            this.sync_children(cx);
                        }
                        MenuIntent::ShowProperties => { cx.emit(ArchiveVmEvent::RequestProperties); }
                        MenuIntent::SelectAll => { this.state.select_all(); this.sync_children(cx); }
                        MenuIntent::InvertSelection => { this.state.invert_selection(); this.sync_children(cx); }
                        MenuIntent::DeleteSelected => {
                            if !this.state.selection.is_empty() {
                                if let Some(ref h) = this.state.archive {
                                    let repo = this.controller.repo(); let handle = h.clone(); let indices: Vec<u32> = this.state.selection.iter().copied().collect();
                                    cx.spawn(async move |_, cx| { crate::adapters::views::dialogs::delete::DeleteDialog::open(cx, indices, handle, repo); }).detach();
                                }
                            }
                        }
                        MenuIntent::RenameSelected => {
                            if let Some(idx) = this.state.first_selected_index() {
                                cx.emit(ArchiveVmEvent::RequestRename { index: idx, new_name: String::new() });
                            }
                        }
                        MenuIntent::Checksum(_algo) => {
                            let handle = this.state.archive.clone();
                            let indices: Vec<u32> = this.state.selection.iter().copied().collect();
                            let repo = this.controller.repo();
                            cx.spawn(async move |_, cx| {
                                if let Some(h) = handle {
                                    crate::adapters::views::dialogs::checksum::ChecksumDialog::open_with_entries(cx, h, indices, repo);
                                }
                            }).detach();
                        }
                        MenuIntent::ShowSettings => { cx.emit(ArchiveVmEvent::RequestShowSettings); }
                        MenuIntent::About => { log::info!("bit7z Archiver {}", env!("CARGO_PKG_VERSION")); }
                    }
                }
            }).detach();

            // Browser intent subscription
            cx.subscribe::<ArchiveBrowser, BrowserIntent>(&archive_browser, {
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
                            this.handle_open_archive(std::path::Path::new(path), None, cx);
                        }
                    }
                }
            }).detach();

            // FileList intent subscription — uses ArchiveState for pure state ops
            cx.subscribe::<ArchiveFileList, FileListIntent>(&entry_list, {
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
                            if this.state.archive.is_some() {
                                let key = this.state.current_path.clone();
                                this.state.directory_cache.remove(&key);
                            }
                        }
                        FileListIntent::OpenEntry => {
                            if let Some(ref h) = this.state.archive {
                                let idx = this.state.first_selected_index();
                                let repo = this.controller.repo();
                                let handle = h.clone();
                                cx.background_spawn(async move {
                                    if let Some(idx_val) = idx {
                                        let uc = crate::application::open_entry::OpenEntryUseCase::new(repo);
                                        let _ = uc.execute(&handle, idx_val);
                                    }
                                }).detach();
                            }
                        }
                        FileListIntent::PreviewEntry => {
                            // Handled by SelectionChanged -> inline preview load
                        }
                        FileListIntent::ExtractSelected => {
                            cx.emit(ArchiveVmEvent::RequestShowExtract);
                        }
                        FileListIntent::RenameEntry(idx) => {
                            let actual_idx = if *idx == 0 { this.state.first_selected_index().unwrap_or(0) } else { *idx };
                            cx.emit(ArchiveVmEvent::RequestRename { index: actual_idx, new_name: String::new() });
                        }
                        FileListIntent::DeleteSelected => {
                            let handle = this.state.archive.clone();
                            let indices: Vec<u32> = this.state.selection.iter().copied().collect();
                            if !indices.is_empty() {
                                let repo = this.controller.repo();
                                cx.spawn(async move |_, cx| {
                                    crate::adapters::views::dialogs::delete::DeleteDialog::open(cx, indices, handle.unwrap(), repo);
                                }).detach();
                            }
                        }
                        FileListIntent::Checksum(_algo) => {
                            let handle = this.state.archive.clone();
                            let indices: Vec<u32> = this.state.selection.iter().copied().collect();
                            if !indices.is_empty() {
                                let repo = this.controller.repo();
                                cx.spawn(async move |_, cx| {
                                    crate::adapters::views::dialogs::checksum::ChecksumDialog::open_with_entries(cx, handle.unwrap(), indices, repo);
                                }).detach();
                            }
                        }
                        FileListIntent::ShowProperties => {
                            cx.emit(ArchiveVmEvent::RequestProperties);
                        }
                    }
                }
            }).detach();

            // Poll IPC commands using a dedicated background thread with std::thread::sleep
            // GPUI does not use tokio, so we cannot use tokio::time::sleep anywhere.
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
            cx.spawn(async move |this, cx| {
                loop {
                    while let Ok(cmd) = ipc_cmd_rx.try_recv() {
                        match cmd {
                            GuiCommand::Open { path, password } => {
                                log::info!("IPC open: {} (password: {:?})", path, password.is_some());
                                let p = std::path::PathBuf::from(&path);
                                let pw = password.clone();
                                this.update(cx, |this, cx| {
                                    this.handle_open_archive(&p, pw, cx);
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
            let mut root = Self {
                menu, toolbar,
                archive_browser, entry_list, preview_panel, status_bar,
                pending_password_path: None,
                repo,
                state: ArchiveState::new(),
                controller: RootController::new(controller_repo),
            };
            if let Some(cb) = deferred_open {
                cb(&mut root, cx);
            }
            root
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

    fn handle_open_archive(&mut self, path: &Path, password: Option<String>, cx: &mut Context<Self>) {
        self.state.status = ViewStatus::Loading;
        self.sync_children(cx);
        cx.emit(ArchiveVmEvent::SelectionChanged(None));

        let repo = self.controller.repo();
        let path_buf = path.to_path_buf();
        let path_string = path.to_string_lossy().to_string();
        let use_case = crate::application::open::OpenArchiveUseCase::new(repo);
        let pw = password.map(|s| crate::domain::archive::Password::new(s));
        let pw_clone = pw.clone();

        let bg_task = cx.background_spawn(async move {
            use_case.execute(&path_buf, pw.as_ref())
        });

        cx.spawn(async move |this, cx| {
            let result = bg_task.await;
            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok(output) => {
                        this.state.archive = Some(output.handle);
                        this.state.properties = Some(output.properties);
                        this.state.archive_password = pw_clone;
                        this.state.current_path = String::new();
                        this.state.path_history.clear();
                        this.state.directory_cache.clear();
                        let mut prefs = cx.global::<crate::gui::PreferencesGlobal>().0.clone();
                        prefs.archive.add_recent(path_string);
                        cx.set_global(crate::gui::PreferencesGlobal(prefs));
                        if let Err(e) = cx.global::<crate::gui::PreferencesRepoGlobal>().0.save(&cx.global::<crate::gui::PreferencesGlobal>().0) {
                            log::warn!("Failed to persist preferences: {}", e);
                        }
                        this.load_current_directory(cx);
                    }
                    Err(e) => {
                        match e {
                            ArchiveError::EncryptedArchiveRequiresPassword => {
                                this.state.status = ViewStatus::Empty;
                                cx.emit(ArchiveVmEvent::RequestPassword { path: path_string });
                            }
                            _ => {
                                this.state.status = ViewStatus::Error(e.to_string());
                            }
                        }
                        cx.notify();
                    }
                }
            });
        }).detach();
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
            cx.spawn(async move |this, cx| {
                let rx = PasswordDialog::open(path, cx);
                use crossbeam::channel::TryRecvError;
                loop {
                    match rx.try_recv() {
                        Ok(result) => {
                            use crate::adapters::views::dialogs::password::PasswordResult;
                            match result {
                                PasswordResult::Submitted(pw) => {
                                    let p_buf = std::path::PathBuf::from(&p);
                                    this.update(cx, |this, cx| {
                                        this.handle_open_archive(&p_buf, Some(pw), cx);
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
                        this.state.select_all();
                        this.sync_children(cx);
                    }
                    "o" if cmd && !shift => {
                        if let Some(path) = crate::adapters::platform::pick_archive_file() {
                            this.handle_open_archive(&path, None, cx);
                        }
                    }
                    "n" if cmd && shift => {
                        cx.emit(ArchiveVmEvent::RequestNewFolder);
                    }
                    "n" if cmd => {
                        cx.emit(ArchiveVmEvent::RequestShowCreate);
                    }
                    "e" if cmd => {
                        cx.emit(ArchiveVmEvent::RequestShowExtract);
                    }
                    "t" if cmd => {
                        let handle = this.state.archive.clone();
                        let repo = this.controller.repo();
                        cx.spawn(async move |_, cx| {
                            if let Some(h) = handle {
                                crate::adapters::views::dialogs::test::TestDialog::open_with_entries(cx, h, repo);
                            }
                        }).detach();
                    }
                    "v" if cmd => {
                        // Preview handled by selection change
                    }
                    "f5" => {
                        if this.state.archive.is_some() {
                            let key = this.state.current_path.clone();
                            this.state.directory_cache.remove(&key);
                        }
                    }
                    "f4" => {
                        if let Some(ref h) = this.state.archive {
                            let repo = this.controller.repo();
                            let mut handle = h.clone();
                            cx.background_spawn(async move {
                                let _ = crate::application::new_file::new_file_and_add(repo, &mut handle, "new_file.txt", None);
                            }).detach();
                        }
                    }
                    "f2" => {
                        if let Some(idx) = this.state.first_selected_index() {
                            cx.emit(ArchiveVmEvent::RequestRename { index: idx, new_name: String::new() });
                        }
                    }
                    "enter" if modifiers.alt => {
                        cx.emit(ArchiveVmEvent::RequestProperties);
                    }
                    "enter" => {
                        if let Some(ref h) = this.state.archive {
                            let idx = this.state.first_selected_index();
                            let repo = this.controller.repo();
                            let handle = h.clone();
                            cx.background_spawn(async move {
                                if let Some(idx_val) = idx {
                                    let uc = crate::application::open_entry::OpenEntryUseCase::new(repo);
                                    let _ = uc.execute(&handle, idx_val);
                                }
                            }).detach();
                        }
                    }
                    "Backspace" | "Delete" => {
                        let handle = this.state.archive.clone();
                        let indices: Vec<u32> = this.state.selection.iter().copied().collect();
                        if !indices.is_empty() {
                            let repo = this.controller.repo();
                            cx.spawn(async move |_, cx| {
                                if let Some(h) = handle {
                                    crate::adapters::views::dialogs::delete::DeleteDialog::open(cx, indices, h, repo);
                                }
                            }).detach();
                        }
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
