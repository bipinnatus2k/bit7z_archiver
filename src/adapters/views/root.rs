use crate::adapters::view_models::archive_state::{ArchiveState, ViewStatus};
use crate::adapters::views::archive_browser::{ArchiveBrowser, BrowserIntent};
use crate::adapters::views::archive_file_list::{ArchiveFileList, FileListIntent};
use crate::adapters::views::menu::{self, Menu};
use crate::adapters::views::preview_panel::PreviewPanel;
use crate::adapters::views::status_bar::StatusBar;
use crate::adapters::views::toolbar::{Toolbar, ToolbarIntent};
use crate::adapters::views::root_controller::RootController;
use crate::adapters::views::dialogs::password::PasswordDialog;
use crate::adapters::events::ArchiveVmEvent;

impl EventEmitter<ArchiveVmEvent> for RootView {}
use crate::domain::repository::{ArchiveError};
use crate::gui::IpcReceiver;
use crate::ipc::GuiCommand;
use crossbeam::channel::unbounded;
use gpui::*;
use std::path::Path;
use gpui_component::resizable::{h_resizable, resizable_panel, v_resizable};

pub struct RootView {
    menu: Entity<Menu>,
    toolbar: Entity<Toolbar>,
    archive_browser: Entity<ArchiveBrowser>,
    entry_list: Entity<ArchiveFileList>,
    preview_panel: Entity<PreviewPanel>,
    status_bar: Entity<StatusBar>,
    pending_password_path: Option<String>,
    // repo: Arc<dyn ArchiveRepository>,
    state: ArchiveState,
    controller: RootController,
}

impl RootView {
    pub fn new(window: &mut Window, cx: &mut App, open_path: Option<String>, open_password: Option<String>) -> Entity<Self> {
        cx.new(|cx| {
            let repo = cx.global::<crate::gui::RepoGlobal>().0.clone();

            let menu = cx.new(|cx| Menu::new(cx));
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
                        ToolbarIntent::CreateArchive => {
                            cx.spawn(async move |_, cx| {
                                crate::adapters::views::dialogs::create::CreateArchiveDialog::open(cx, vec![]);
                            }).detach();
                        }
                        ToolbarIntent::AddFiles => {
                            if let Some(ref handle) = this.state.archive {
                                let repo = this.controller.repo();
                                let h = handle.clone();
                                cx.spawn(async move |_, cx| {
                                    crate::adapters::views::dialogs::add_files::AddFilesDialog::open(cx, crate::domain::archive::ArchiveFormat::SevenZip, Some(h), Some(repo), false);
                                }).detach();
                            }
                        }
                        ToolbarIntent::ExtractSelected => {
                            let indices: Vec<u32> = this.state.selection.iter().copied().collect();
                            let entries: Vec<crate::domain::archive::ArchiveEntry> = this.state.directory_cache
                                .get(&this.state.current_path)
                                .map(|all| all.iter().filter(|e| indices.contains(&e.original_index)).cloned().collect())
                                .unwrap_or_default();
                            if !entries.is_empty() {
                                cx.spawn(async move |_, cx| {
                                    crate::adapters::views::dialogs::extract::ExtractDialog::open(entries, cx);
                                }).detach();
                            }
                        }
                        ToolbarIntent::TestArchive => {
                            let indices = if this.state.selection.is_empty() {
                                None
                            } else {
                                Some(this.state.selection.iter().copied().collect::<Vec<u32>>())
                            };
                            let handle = this.state.archive.clone();
                            let repo = this.controller.repo();
                            cx.spawn(async move |_, cx| {
                                if let Some(h) = handle {
                                    crate::adapters::views::dialogs::test::TestDialog::open_with_entries(cx, h, indices, repo);
                                }
                            }).detach();
                        }
                        ToolbarIntent::CloseArchive => {
                            if let Some(h) = this.state.archive.take() { this.controller.close_archive(h); }
                            this.state = ArchiveState::new();
                            this.sync_children(cx);
                        }
                        ToolbarIntent::ShowSettings => {
                            cx.spawn(async move |_, cx| {
                                crate::adapters::views::dialogs::settings::SettingsDialog::open(cx);
                            }).detach();
                        }
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
                        FileListIntent::SelectionChanged(indices) => {
                            this.state.selection = indices.iter().copied().collect();
                            this.state.selection_anchor = None;
                            this.sync_children(cx);
                            // Trigger preview for the first selected entry
                            if let Some(idx) = indices.first() {
                                if let Some(ref archive) = this.state.archive {
                                    let repo = this.controller.repo();
                                    let panel = this.preview_panel.clone();
                                    let h = archive.clone();
                                    let idx = *idx;
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
                            this.entry_list.update(cx, |c, cx| c.select_all_entries(cx));
                            this.sync_children(cx);
                        }
                        FileListIntent::ClearSelection => {
                            this.state.clear_selection();
                            this.entry_list.update(cx, |c, cx| c.clear_selection(cx));
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
                                if let Some(idx) = this.state.first_selected_index() {
                                    let is_dir = this.state.displayed_entries()
                                        .iter()
                                        .find(|e| e.original_index == idx)
                                        .map_or(false, |e| e.is_directory);
                                    if is_dir {
                                        let name = this.state.displayed_entries()
                                            .iter()
                                            .find(|e| e.original_index == idx)
                                            .map(|e| e.display_name.clone())
                                            .unwrap();
                                        this.state.navigate_into(&name);
                                        this.sync_children(cx);
                                        this.load_current_directory(cx);
                                    } else {
                                        let repo = this.controller.repo();
                                        let handle = h.clone();
                                        cx.background_spawn(async move {
                                            let uc = crate::application::open_entry::OpenEntryUseCase::new(repo);
                                            let _ = uc.execute(&handle, idx);
                                        }).detach();
                                    }
                                }
                            }
                        }
                        FileListIntent::PreviewEntry => {
                            // Handled by SelectionChanged -> inline preview load
                        }
                        FileListIntent::ExtractSelected => {
                            let indices: Vec<u32> = this.state.selection.iter().copied().collect();
                            let entries: Vec<crate::domain::archive::ArchiveEntry> = this.state.directory_cache
                                .get(&this.state.current_path)
                                .map(|all| all.iter().filter(|e| indices.contains(&e.original_index)).cloned().collect())
                                .unwrap_or_default();
                            if !entries.is_empty() {
                                cx.spawn(async move |_, cx| {
                                    crate::adapters::views::dialogs::extract::ExtractDialog::open(entries, cx);
                                }).detach();
                            }
                        }
                        FileListIntent::TestSelected => {
                            let indices: Vec<u32> = this.state.selection.iter().copied().collect();
                            let handle = this.state.archive.clone();
                            let repo = this.controller.repo();
                            if let Some(ref h) = handle {
                                let h_clone = h.clone();
                                cx.spawn(async move |_, cx| {
                                    crate::adapters::views::dialogs::test::TestDialog::open_with_entries(cx, h_clone, Some(indices), repo);
                                }).detach();
                            }
                        }
                        FileListIntent::RenameEntry(idx) => {
                            let actual_idx = idx.unwrap_or_else(|| this.state.first_selected_index().unwrap_or(0));
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
                            let indices: Vec<u32> = this.state.selection.iter().copied().collect();
                            let entries: Vec<crate::domain::archive::ArchiveEntry> = this.state.directory_cache
                                .get(&this.state.current_path)
                                .map(|all| all.iter().filter(|e| indices.contains(&e.original_index)).cloned().collect())
                                .unwrap_or_default();
                            if !entries.is_empty() {
                                cx.spawn(async move |_, cx| {
                                    crate::adapters::views::dialogs::properties::PropertiesDialog::open_entries(entries, cx);
                                }).detach();
                            } else if let Some(ref handle) = this.state.archive {
                                let path_str = handle.path.as_ref()
                                    .map(|p| p.to_string_lossy().to_string())
                                    .unwrap_or_default();
                                let repo = this.controller.repo();
                                let h = handle.clone();
                                cx.spawn(async move |_, cx| {
                                    if let Ok(props) = repo.get_properties(&h) {
                                        crate::adapters::views::dialogs::properties::PropertiesDialog::open_archive(path_str, props, cx);
                                    }
                                }).detach();
                            }
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
                            .and_then(|rx| rx.try_recv().ok());
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
                                let _ = this.update(cx, |this, cx| {
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
                // repo,
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
        let _selection = self.state.selection.clone();
        let status = self.state.status.clone();
        let path = self.state.current_path.clone();
        let is_ready = self.state.is_ready();
        let has_sel = self.state.has_selection();
        let single = self.state.selection.len() == 1;
        let is_open = self.state.archive.is_some();
        let subdirs = self.state.filtered_subdirs();
        let status_text = self.state.status_text();

        self.entry_list.update(cx, |c, cx| c.set_state(entries, status, path, cx));
        self.toolbar.update(cx, |c, _| c.set_state(is_open, is_ready, has_sel));
        self.menu.update(cx, |c, cx| c.set_state(is_open, has_sel, single, cx));
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
                                this.pending_password_path = Some(path_string);
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

        cx.spawn(async move |this, cx| {
            if let Some(ref h) = handle {
                match controller.list_directory(h, &path) {
                    Ok(entries) => {
                        let _ = this.update(cx, |this, cx| {
                            this.state.directory_cache.insert(path.clone(), entries);
                            this.state.reapply_filter_and_sort();
                            this.state.status = ViewStatus::Ready;
                            this.sync_children(cx);
                            cx.notify();
                        });
                    }
                    Err(e) => {
                        let _ = this.update(cx, |this, cx| {
                            this.state.status = ViewStatus::Error(e.to_string());
                            cx.notify();
                        });
                    }
                }
            } else {
                let _ = this.update(cx, |this, cx| {
                    this.state.status = ViewStatus::Ready;
                    cx.notify();
                });
            }
        }).detach();
    }

    fn menu_checksum(&self, cx: &mut Context<Self>) {
        let handle = self.state.archive.clone();
        let indices: Vec<u32> = self.state.selection.iter().copied().collect();
        let repo = self.controller.repo();
        cx.spawn(async move |_, cx| {
            if let Some(h) = handle {
                crate::adapters::views::dialogs::checksum::ChecksumDialog::open_with_entries(cx, h, indices, repo);
            }
        }).detach();
    }
}

impl Render for RootView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
                                    }).expect("TODO: panic message");
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
                        this.entry_list.update(cx, |c, cx| c.select_all_entries(cx));
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
                        cx.spawn(async move |_, cx| {
                            crate::adapters::views::dialogs::create::CreateArchiveDialog::open(cx, vec![]);
                        }).detach();
                    }
                    "e" if cmd => {
                        let indices: Vec<u32> = this.state.selection.iter().copied().collect();
                        let entries: Vec<crate::domain::archive::ArchiveEntry> = this.state.directory_cache
                            .get(&this.state.current_path)
                            .map(|all| all.iter().filter(|e| indices.contains(&e.original_index)).cloned().collect())
                            .unwrap_or_default();
                        if !entries.is_empty() {
                            cx.spawn(async move |_, cx| {
                                crate::adapters::views::dialogs::extract::ExtractDialog::open(entries, cx);
                            }).detach();
                        }
                    }
                    "t" if cmd => {
                        let handle = this.state.archive.clone();
                        let repo = this.controller.repo();
                        cx.spawn(async move |_, cx| {
                            if let Some(h) = handle {
                                crate::adapters::views::dialogs::test::TestDialog::open_with_entries(cx, h, None, repo);
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
                        let indices: Vec<u32> = this.state.selection.iter().copied().collect();
                        let entries: Vec<crate::domain::archive::ArchiveEntry> = this.state.directory_cache
                            .get(&this.state.current_path)
                            .map(|all| all.iter().filter(|e| indices.contains(&e.original_index)).cloned().collect())
                            .unwrap_or_default();
                        if !entries.is_empty() {
                            cx.spawn(async move |_, cx| {
                                crate::adapters::views::dialogs::properties::PropertiesDialog::open_entries(entries, cx);
                            }).detach();
                        } else if let Some(ref handle) = this.state.archive {
                            let path_str = handle.path.as_ref()
                                .map(|p| p.to_string_lossy().to_string())
                                .unwrap_or_default();
                            let repo = this.controller.repo();
                            let h = handle.clone();
                            cx.spawn(async move |_, cx| {
                                if let Ok(props) = repo.get_properties(&h) {
                                    crate::adapters::views::dialogs::properties::PropertiesDialog::open_archive(path_str, props, cx);
                                }
                            }).detach();
                        }
                    }
                    "enter" => {
                        if let Some(ref h) = this.state.archive {
                            if let Some(idx) = this.state.first_selected_index() {
                                let is_dir = this.state.displayed_entries()
                                    .iter()
                                    .find(|e| e.original_index == idx)
                                    .map_or(false, |e| e.is_directory);
                                if is_dir {
                                    let name = this.state.displayed_entries()
                                        .iter()
                                        .find(|e| e.original_index == idx)
                                        .map(|e| e.display_name.clone())
                                        .unwrap();
                                    this.state.navigate_into(&name);
                                    this.sync_children(cx);
                                    this.load_current_directory(cx);
                                } else {
                                    let repo = this.controller.repo();
                                    let handle = h.clone();
                                    cx.background_spawn(async move {
                                        let uc = crate::application::open_entry::OpenEntryUseCase::new(repo);
                                        let _ = uc.execute(&handle, idx);
                                    }).detach();
                                }
                            }
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
            .on_action(cx.listener(|this: &mut RootView, _: &menu::OpenArchive, _window, cx| {
                if let Some(path) = crate::adapters::platform::pick_archive_file() {
                    this.handle_open_archive(&path, None, cx);
                }
            }))
            .on_action(cx.listener(|_: &mut RootView, _: &menu::CreateArchive, _window, cx| {
                cx.spawn(async move |_, cx| {
                    crate::adapters::views::dialogs::create::CreateArchiveDialog::open(cx, vec![]);
                }).detach();
            }))
            .on_action(cx.listener(|this: &mut RootView, _: &menu::AddFiles, _window, cx| {
                if let Some(ref handle) = this.state.archive {
                    let repo = this.controller.repo();
                    let h = handle.clone();
                    cx.spawn(async move |_, cx| {
                        crate::adapters::views::dialogs::add_files::AddFilesDialog::open(cx, crate::domain::archive::ArchiveFormat::SevenZip, Some(h), Some(repo), false);
                    }).detach();
                }
            }))
            .on_action(cx.listener(|this: &mut RootView, _: &menu::TestSelected, _window, cx| {
                let indices: Vec<u32> = this.state.selection.iter().copied().collect();
                let handle = this.state.archive.clone();
                let repo = this.controller.repo();
                cx.spawn(async move |_, cx| {
                    if let Some(h) = handle {
                        crate::adapters::views::dialogs::test::TestDialog::open_with_entries(cx, h, Some(indices), repo);
                    }
                }).detach();
            }))
            .on_action(cx.listener(|this: &mut RootView, _: &menu::TestAll, _window, cx| {
                let handle = this.state.archive.clone();
                let repo = this.controller.repo();
                cx.spawn(async move |_, cx| {
                    if let Some(h) = handle {
                        crate::adapters::views::dialogs::test::TestDialog::open_with_entries(cx, h, None, repo);
                    }
                }).detach();
            }))
            .on_action(cx.listener(|this: &mut RootView, _: &menu::CloseArchive, _window, cx| {
                if let Some(h) = this.state.archive.take() { this.controller.close_archive(h); }
                this.state = ArchiveState::new();
                this.sync_children(cx);
            }))
            .on_action(cx.listener(|this: &mut RootView, _: &menu::ShowProperties, _window, cx| {
                let indices: Vec<u32> = this.state.selection.iter().copied().collect();
                let entries: Vec<crate::domain::archive::ArchiveEntry> = this.state.directory_cache
                    .get(&this.state.current_path)
                    .map(|all| all.iter().filter(|e| indices.contains(&e.original_index)).cloned().collect())
                    .unwrap_or_default();
                if !entries.is_empty() {
                    cx.spawn(async move |_, cx| {
                        crate::adapters::views::dialogs::properties::PropertiesDialog::open_entries(entries, cx);
                    }).detach();
                } else if let Some(ref handle) = this.state.archive {
                    let path_str = handle.path.as_ref()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_default();
                    let repo = this.controller.repo();
                    let h = handle.clone();
                    cx.spawn(async move |_, cx| {
                        if let Ok(props) = repo.get_properties(&h) {
                            crate::adapters::views::dialogs::properties::PropertiesDialog::open_archive(path_str, props, cx);
                        }
                    }).detach();
                }
            }))
            .on_action(cx.listener(|this: &mut RootView, _: &menu::SelectAll, _window, cx| {
                this.state.select_all();
                this.entry_list.update(cx, |c, cx| c.select_all_entries(cx));
                this.sync_children(cx);
            }))
            .on_action(cx.listener(|this: &mut RootView, _: &menu::InvertSelection, _window, cx| {
                this.state.invert_selection();
                this.sync_children(cx);
            }))
            .on_action(cx.listener(|this: &mut RootView, _: &menu::DeleteSelected, _window, cx| {
                if !this.state.selection.is_empty() {
                    if let Some(ref h) = this.state.archive {
                        let repo = this.controller.repo(); let handle = h.clone(); let indices: Vec<u32> = this.state.selection.iter().copied().collect();
                        cx.spawn(async move |_, cx| { crate::adapters::views::dialogs::delete::DeleteDialog::open(cx, indices, handle, repo); }).detach();
                    }
                }
            }))
            .on_action(cx.listener(|this: &mut RootView, _: &menu::RenameSelected, _window, cx| {
                if let Some(idx) = this.state.first_selected_index() {
                    cx.emit(ArchiveVmEvent::RequestRename { index: idx, new_name: String::new() });
                }
            }))
            .on_action(cx.listener(|this: &mut RootView, _: &menu::ChecksumCrc32, _window, cx| {
                this.menu_checksum(cx);
            }))
            .on_action(cx.listener(|this: &mut RootView, _: &menu::ChecksumMd5, _window, cx| {
                this.menu_checksum(cx);
            }))
            .on_action(cx.listener(|this: &mut RootView, _: &menu::ChecksumSha1, _window, cx| {
                this.menu_checksum(cx);
            }))
            .on_action(cx.listener(|this: &mut RootView, _: &menu::ChecksumSha256, _window, cx| {
                this.menu_checksum(cx);
            }))
            .on_action(cx.listener(|_: &mut RootView, _: &menu::ShowSettings, _window, cx| {
                cx.spawn(async move |_, cx| {
                    crate::adapters::views::dialogs::settings::SettingsDialog::open(cx);
                }).detach();
            }))
            .on_action(cx.listener(|_: &mut RootView, _: &menu::About, _window, cx| {
                cx.spawn(async move |_, cx| {
                    crate::adapters::views::dialogs::about::AboutDialog::open(cx);
                }).detach();
            }))
            .child(self.menu.clone())
            .child(self.toolbar.clone())
            .child(div().flex_1().child(
                h_resizable("main-hz")
                    .child(
                        resizable_panel()
                            .size(px(255.))           // 初始宽度
                            .size_range(px(200.)..px(320.))  // 最小/最大宽度限制
                            // .flex_none()
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
                                    // .flex_none()
                                    .child(self.preview_panel.clone())
                            )
                    )
            ))
            .child(self.status_bar.clone())


    }
}
