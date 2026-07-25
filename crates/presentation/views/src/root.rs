use crate::archive_browser::{ArchiveBrowser, BrowserIntent};
use crate::archive_file_list::{ArchiveFileList, FileListIntent};
use crate::menu::{self, MenuView};
use crate::preview_panel::PreviewPanelView;
use crate::root_controller::RootController;
use crate::status_bar::StatusBarView;
use bit7z_app_preview::PreviewData;
use bit7z_domain::repository::ArchiveError;
use bit7z_infra_events::ArchiveVmEvent;
use std::sync::Arc;
use bit7z_pres_dialogs::password::PasswordDialog;
use bit7z_pres_settings::SettingsStore;
use bit7z_pres_view_models::archive_state::{ArchiveState, ViewStatus};
use bit7z_rt_app_state::AppState;
use bit7z_rt_ipc::GuiCommand;
use crossbeam_channel::unbounded;
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::button::Button;
use gpui_component::menu::AppMenuBar;
use gpui_component::resizable::{h_resizable, resizable_panel, v_resizable};
use gpui_component::{Disableable, GlobalState, IconName, Root, h_flex, v_flex};
use std::path::Path;
use std::sync::atomic::AtomicBool;

pub struct RootView {
    pub app_state: Arc<AppState>,
    focus_handle: FocusHandle,
    menu_bar: Entity<AppMenuBar>,
    archive_browser: Entity<ArchiveBrowser>,
    entry_list: Entity<ArchiveFileList>,
    preview_data: Option<Arc<PreviewData>>,
    preview_loading: bool,
    pending_password_path: Option<String>,
    state: ArchiveState,
    controller: RootController,
    sidebar_collapsed: bool,
}

impl EventEmitter<ArchiveVmEvent> for RootView {}

impl RootView {
    pub fn new(
        app_state: Arc<AppState>,
        window: &mut Window,
        cx: &mut App,
        open_path: Option<String>,
        open_password: Option<String>,
    ) -> Entity<Self> {
        cx.new(|cx| {
            let service = app_state.service.clone();

            let menu_bar = AppMenuBar::new(cx);
            let archive_browser = cx.new(|cx| ArchiveBrowser::new(window, cx));
            let entry_list = cx.new(|cx| ArchiveFileList::new(window, cx));

            let deferred_open = open_path.map(|path| {
                let pw = open_password.clone();
                let p = path;
                move |this: &mut RootView, cx: &mut Context<RootView>| {
                    this.handle_open_archive(std::path::Path::new(&p), pw, cx);
                }
            });

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

            cx.subscribe::<ArchiveFileList, FileListIntent>(&entry_list, {
                move |this: &mut RootView, _emitter, intent: &FileListIntent, cx| {
                    match intent {
                        FileListIntent::SelectionChanged(indices) => {
                            this.state.selection = indices.iter().copied().collect();
                            this.state.selection_anchor = None;
                            this.sync_children(cx);
                            if let Some(idx) = indices.first() {
                                if let Some(ref archive) = this.state.archive {
                                    let repo = this.controller.service();
                                    let h = archive.clone();
                                    let idx = *idx;
                                    let this_entity = cx.entity();
                                    cx.spawn(async move |_, cx| {
                                        this_entity.update(cx, |this, cx| {
                                            this.preview_loading = true;
                                            cx.notify();
                                        });
                                        let uc = bit7z_app_preview::PreviewEntryUseCase::new(repo);
                                        match uc.execute(&h, idx, 1_048_576) {
                                            Ok(data) => {
                                                this_entity.update(cx, |this, cx| {
                                                    this.preview_data = Some(Arc::new(data));
                                                    this.preview_loading = false;
                                                    cx.notify();
                                                });
                                            }
                                            Err(_) => {
                                                this_entity.update(cx, |this, cx| {
                                                    this.preview_data = None;
                                                    this.preview_loading = false;
                                                    cx.notify();
                                                });
                                            }
                                        }
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
                        FileListIntent::OpenEntry(name) => {
                            if let Some(ref h) = this.state.archive {
                                if let Some(entry) = this.state.displayed_entries()
                                    .iter()
                                    .find(|e| e.display_name == *name)
                                {
                                    if entry.is_directory {
                                        this.state.navigate_into(&name);
                                        this.sync_children(cx);
                                        this.load_current_directory(cx);
                                    } else {
                                        let idx = entry.original_index;
                                        let repo = this.controller.service();
                                        let handle = h.clone();
                                        cx.background_spawn(async move {
                                            let uc = bit7z_app_archive::open_entry::OpenEntryUseCase::new(repo);
                                            let _ = uc.execute(&handle, idx);
                                        }).detach();
                                    }
                                }
                            }
                        }
                        FileListIntent::PreviewEntry => {
                        }
                        FileListIntent::ExtractSelected => {
                            let (indices, entries) = this.selected_entries_data();
                            if !entries.is_empty() {
                                if let Some(ref handle) = this.state.archive {
                                    let handle = handle.clone();
                                    let controller = this.controller.clone();
                                    let busy = indices.clone();
                                    cx.spawn(async move |_, cx| {
                                        let rx = bit7z_pres_dialogs::extract::ExtractDialog::open(entries, cx);
                                        use crossbeam_channel::TryRecvError;
                                        loop {
                                            match rx.try_recv() {
                                                Ok(bit7z_pres_dialogs::extract::ExtractDialogEvent::ExtractRequested { destination, overwrite_mode, .. }) => {
                                                    let (tx, progress_rx) = bit7z_infra_progress::progress_channel();
                                                    let cancel = Arc::new(AtomicBool::new(false));
                                                    let paused = Arc::new(AtomicBool::new(false));
                                                    bit7z_pres_dialogs::progress::ProgressDialog::open(cx, format!("Extracting..."), progress_rx, Some(cancel.clone()), Some(paused.clone()));
                                                    let ctrl = controller.clone();
                                                    let h = handle.clone();
                                                    let dest = destination.clone();
                                                    let idx = busy.clone();
                                                    cx.background_spawn(async move {
                                                        let _ = ctrl.extract(&h, &idx, &dest, overwrite_mode, Some(tx), Some(cancel), Some(paused));
                                                    }).detach();
                                                    break;
                                                }
                                                Ok(bit7z_pres_dialogs::extract::ExtractDialogEvent::Canceled) => break,
                                                Err(TryRecvError::Empty) => {
                                                    cx.background_spawn(std::future::ready(())).await;
                                                }
                                                Err(TryRecvError::Disconnected) => break,
                                            }
                                        }
                                    }).detach();
                                }
                            }
                        }
                        FileListIntent::TestSelected => {
                            let indices: Vec<u32> = this.state.selected_indices();
                            let handle = this.state.archive.clone();
                            let repo = this.controller.service();
                            if let Some(ref h) = handle {
                                let h_clone = h.clone();
                                cx.spawn(async move |_, cx| {
                                    bit7z_pres_dialogs::test::TestDialog::open_with_entries(cx, h_clone, Some(indices), repo);
                                }).detach();
                            }
                        }
                        FileListIntent::RenameEntry(idx) => {
                            let actual_idx = idx.unwrap_or_else(|| this.state.first_selected_index().unwrap_or(0));
                            cx.emit(ArchiveVmEvent::RequestRename { index: actual_idx, new_name: String::new() });
                        }
                        FileListIntent::DeleteSelected => {
                            let handle = this.state.archive.clone();
                            let indices: Vec<u32> = this.state.selected_indices();
                            if !indices.is_empty() {
                                let repo = this.controller.service();
                                cx.spawn(async move |_, cx| {
                                    bit7z_pres_dialogs::delete::DeleteDialog::open(cx, indices, handle.unwrap(), repo);
                                }).detach();
                            }
                        }
                        FileListIntent::Checksum(_algo) => {
                            let handle = this.state.archive.clone();
                            let indices: Vec<u32> = this.state.selected_indices();
                            if !indices.is_empty() {
                                let repo = this.controller.service();
                                cx.spawn(async move |_, cx| {
                                    bit7z_pres_dialogs::checksum::ChecksumDialog::open_with_entries(cx, handle.unwrap(), indices, repo);
                                }).detach();
                            }
                        }
                        FileListIntent::ShowProperties => {
                            let (_indices, entries) = this.selected_entries_data();
                            if !entries.is_empty() {
                                cx.spawn(async move |_, cx| {
                                    bit7z_pres_dialogs::properties::PropertiesDialog::open_entries(entries, cx);
                                }).detach();
                            } else if let Some(ref handle) = this.state.archive {
                                let path_str = handle.path.as_ref()
                                    .map(|p| p.to_string_lossy().to_string())
                                    .unwrap_or_default();
                                let controller = this.controller.clone();
                                let h = handle.clone();
                                cx.spawn(async move |_, cx| {
                                    if let Ok(props) = controller.get_properties(&h) {
                                        bit7z_pres_dialogs::properties::PropertiesDialog::open_archive(path_str, props, cx);
                                    }
                                }).detach();
                            }
                        }
                    }
                }
            }).detach();

            let (ipc_cmd_tx, ipc_cmd_rx) = unbounded::<GuiCommand>();
            let ipc_receiver_arc = app_state.ipc_receiver.clone();

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
                    cx.background_spawn(std::future::ready(())).await;
                }
            }).detach();

            let focus_handle = cx.focus_handle();

            let mut root = Self {
                app_state,
                focus_handle,
                menu_bar,
                archive_browser,
                entry_list,
                preview_data: None,
                preview_loading: false,
                pending_password_path: None,
                state: ArchiveState::new(),
                controller: RootController::new(service),
                sidebar_collapsed: false,
            };

            let menus = menu::build_menus(false, false, vec![]);
            let owned: Vec<OwnedMenu> = menus.into_iter().map(|m| m.owned()).collect();
            GlobalState::global_mut(cx).set_app_menus(owned);
            root.menu_bar.update(cx, |bar, cx| bar.reload(cx));

            if let Some(cb) = deferred_open {
                cb(&mut root, cx);
            }
            root
        })
    }

    pub fn view(
        window: &mut Window,
        cx: &mut App,
        path: Option<String>,
        password: Option<String>,
    ) -> Entity<Self> {
        let app_state = bit7z_rt_app_state::AppState::global(cx);
        Self::new(app_state, window, cx, path, password)
    }

    fn selected_entries_data(&self) -> (Vec<u32>, Vec<bit7z_domain::archive::ArchiveEntry>) {
        let indices = self.state.selected_indices();
        let entries = self
            .state
            .directory_cache
            .get(&self.state.current_path)
            .map(|all| {
                all.iter()
                    .filter(|e| indices.contains(&e.original_index))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        (indices, entries)
    }

    fn sync_children(&mut self, cx: &mut Context<Self>) {
        let entries = self.state.displayed_entries().to_vec();
        let status = self.state.status.clone();
        let path = self.state.current_path.clone();
        let is_open = self.state.archive.is_some();
        let has_sel = self.state.has_selection();
        let subdirs = self.state.filtered_subdirs();

        self.entry_list
            .update(cx, |c, cx| c.set_state(entries, status, path, cx));
        self.archive_browser.update(cx, |c, cx| {
            c.set_collapsed(self.sidebar_collapsed, cx);
            c.set_state(subdirs, vec![]);
        });

        let menus = menu::build_menus(is_open, has_sel, vec![]);
        let owned: Vec<OwnedMenu> = menus.into_iter().map(|m| m.owned()).collect();
        GlobalState::global_mut(cx).set_app_menus(owned);
        self.menu_bar.update(cx, |bar, cx| bar.reload(cx));
    }

    fn handle_open_archive(
        &mut self,
        path: &Path,
        password: Option<String>,
        cx: &mut Context<Self>,
    ) {
        self.state.status = ViewStatus::Loading;
        self.sync_children(cx);
        cx.emit(ArchiveVmEvent::SelectionChanged(None));

        let service = self.controller.service();
        let path_buf = path.to_path_buf();
        let path_string = path.to_string_lossy().to_string();
        let use_case = bit7z_app_archive::open::OpenArchiveUseCase::new(service);
        let pw = password.map(|s| bit7z_domain::archive::Password::new(s));
        let pw_clone = pw.clone();

        let bg_task = cx.background_spawn(async move { use_case.execute(&path_buf, pw.as_ref()) });

        cx.spawn(async move |this, cx| {
            let result = bg_task.await;
            let _ = this.update(cx, |this, cx| match result {
                Ok(output) => {
                    this.state.archive = Some(output.handle);
                    this.state.properties = Some(output.properties);
                    this.state.archive_password = pw_clone;
                    this.state.current_path = String::new();
                    this.state.path_history.clear();
                    this.state.directory_cache.clear();
                    let path_str = path_string.clone();
                    SettingsStore::get_mut(cx).update_and_save(|p| {
                        p.archive.add_recent(path_str);
                    });
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
            });
        })
        .detach();
    }

    fn load_current_directory(&mut self, cx: &mut Context<Self>) {
        let handle = self.state.archive.clone();
        let path = self.state.current_path.clone();
        let controller = self.controller.service();

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
        })
        .detach();
    }

    fn handle_save_archive(&mut self, _cx: &mut Context<Self>) {
        if let Some(ref h) = self.state.archive {
            let _ = self.controller.commit_archive(h);
        }
    }

    fn handle_undo(&mut self, cx: &mut Context<Self>) {
        if let Some(ref h) = self.state.archive {
            let result = self.controller.undo_archive(h);
            if let Ok(true) = result {
                self.state.directory_cache.clear();
                self.load_current_directory(cx);
            } else {
                cx.notify();
            }
        }
    }

    fn handle_redo(&mut self, cx: &mut Context<Self>) {
        if let Some(ref h) = self.state.archive {
            let result = self.controller.redo_archive(h);
            if let Ok(true) = result {
                self.state.directory_cache.clear();
                self.load_current_directory(cx);
            } else {
                cx.notify();
            }
        }
    }

    fn handle_close_archive(&mut self, cx: &mut Context<Self>) {
        let h = match self.state.archive.take() {
            Some(h) => h,
            None => return,
        };
        if self.controller.has_unsaved_changes(&h) {
            let _ = self.controller.discard_pending(&h);
        }
        self.controller.close_archive(h);
        self.state = ArchiveState::new();
        self.sync_children(cx);
    }

    fn menu_checksum(&self, cx: &mut Context<Self>) {
        let handle = self.state.archive.clone();
        let indices: Vec<u32> = self.state.selection.iter().copied().collect();
        let repo = self.controller.service();
        cx.spawn(async move |_, cx| {
            if let Some(h) = handle {
                bit7z_pres_dialogs::checksum::ChecksumDialog::open_with_entries(
                    cx, h, indices, repo,
                );
            }
        })
        .detach();
    }
}

impl Focusable for RootView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for RootView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_open = self.state.archive.is_some();
        let is_ready = self.state.is_ready();
        let has_sel = self.state.has_selection();
        let status_text = self.state.status_text();

        let (has_unsaved, can_undo, can_redo) = self
            .state
            .archive
            .as_ref()
            .map(|h| {
                (
                    self.controller.has_unsaved_changes(h),
                    self.controller.can_undo(h),
                    self.controller.can_redo(h),
                )
            })
            .unwrap_or((false, false, false));
        let compact = window.bounds().size.width < px(640.0);

        let dialog_layer = Root::render_dialog_layer(window, cx);

        if let Some(path) = self.pending_password_path.take() {
            let p = path.clone();
            cx.spawn(async move |this, cx| {
                let rx = PasswordDialog::open(path, cx);
                use crossbeam_channel::TryRecvError;
                loop {
                    match rx.try_recv() {
                        Ok(result) => {
                            use bit7z_pres_dialogs::password::PasswordResult;
                            match result {
                                PasswordResult::Submitted(pw) => {
                                    let p_buf = std::path::PathBuf::from(&p);
                                    this.update(cx, |this, cx| {
                                        this.handle_open_archive(&p_buf, Some(pw), cx);
                                    })
                                    .expect("TODO: panic message");
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
            })
            .detach();
        }

        v_flex().size_full().relative()
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
                        if let Some(path) = bit7z_infra_platform::pick_archive_file() {
                            this.handle_open_archive(&path, None, cx);
                        }
                    }
                    "n" if cmd && shift => {
                        cx.emit(ArchiveVmEvent::RequestNewFolder);
                    }
                    "n" if cmd => {
                        cx.spawn(async move |_, cx| {
                            bit7z_pres_dialogs::create::CreateArchiveDialog::open(cx, vec![]);
                        }).detach();
                    }
                    "e" if cmd => {
                        let (indices, entries) = this.selected_entries_data();
                        if !entries.is_empty() {
                            if let Some(ref handle) = this.state.archive {
                                let handle = handle.clone();
                                let controller = this.controller.clone();
                                let busy = indices.clone();
                                cx.spawn(async move |_, cx| {
                                    let rx = bit7z_pres_dialogs::extract::ExtractDialog::open(entries, cx);
                                    use crossbeam_channel::TryRecvError;
                                    loop {
                                        match rx.try_recv() {
                                                Ok(bit7z_pres_dialogs::extract::ExtractDialogEvent::ExtractRequested { destination, overwrite_mode, .. }) => {
                                                    let (tx, progress_rx) = bit7z_infra_progress::progress_channel();
                                                    let cancel = Arc::new(AtomicBool::new(false));
                                                    let paused = Arc::new(AtomicBool::new(false));
                                                    bit7z_pres_dialogs::progress::ProgressDialog::open(cx, format!("Extracting..."), progress_rx, Some(cancel.clone()), Some(paused.clone()));
                                                    let ctrl = controller.clone();
                                                    let h = handle.clone();
                                                    let dest = destination.clone();
                                                    let idx = busy.clone();
                                                    cx.background_spawn(async move {
                                                        let _ = ctrl.extract(&h, &idx, &dest, overwrite_mode, Some(tx), Some(cancel), Some(paused));
                                                    }).detach();
                                                    break;
                                                }
                                            Ok(bit7z_pres_dialogs::extract::ExtractDialogEvent::Canceled) => break,
                                            Err(TryRecvError::Empty) => {
                                                cx.background_spawn(std::future::ready(())).await;
                                            }
                                            Err(TryRecvError::Disconnected) => break,
                                        }
                                    }
                                }).detach();
                            }
                        }
                    }
                    "s" if cmd => {
                        this.handle_save_archive(cx);
                    }
                    "z" if cmd && !shift => {
                        this.handle_undo(cx);
                    }
                    "y" if cmd => {
                        this.handle_redo(cx);
                    }
                    "z" if cmd && shift => {
                        this.handle_redo(cx);
                    }
                    "t" if cmd => {
                        let handle = this.state.archive.clone();
                        let repo = this.controller.service();
                        cx.spawn(async move |_, cx| {
                            if let Some(h) = handle {
                                bit7z_pres_dialogs::test::TestDialog::open_with_entries(cx, h, None, repo);
                            }
                        }).detach();
                    }
                    "v" if cmd => {
                    }
                    "f5" => {
                        if this.state.archive.is_some() {
                            let key = this.state.current_path.clone();
                            this.state.directory_cache.remove(&key);
                        }
                    }
                    "f4" => {
                        if let Some(ref h) = this.state.archive {
                            let repo = this.controller.service();
                            let handle = h.clone();
                            cx.background_spawn(async move {
                                let _ = bit7z_app_archive::new_file::new_file_and_add(repo, &handle, "new_file.txt", None);
                            }).detach();
                        }
                    }
                    "f2" => {
                        if let Some(idx) = this.state.first_selected_index() {
                            cx.emit(ArchiveVmEvent::RequestRename { index: idx, new_name: String::new() });
                        }
                    }
                    "enter" if modifiers.alt => {
                        let (_indices, entries) = this.selected_entries_data();
                        if !entries.is_empty() {
                            cx.spawn(async move |_, cx| {
                                bit7z_pres_dialogs::properties::PropertiesDialog::open_entries(entries, cx);
                            }).detach();
                        } else if let Some(ref handle) = this.state.archive {
                            let path_str = handle.path.as_ref()
                                .map(|p| p.to_string_lossy().to_string())
                                .unwrap_or_default();
                            let controller = this.controller.clone();
                            let h = handle.clone();
                            cx.spawn(async move |_, cx| {
                                if let Ok(props) = controller.get_properties(&h) {
                                    bit7z_pres_dialogs::properties::PropertiesDialog::open_archive(path_str, props, cx);
                                }
                            }).detach();
                        }
                    }
                    "Backspace" | "Delete" => {
                        let handle = this.state.archive.clone();
                        let indices: Vec<u32> = this.state.selected_indices();
                        if !indices.is_empty() {
                            let repo = this.controller.service();
                            cx.spawn(async move |_, cx| {
                                if let Some(h) = handle {
                                    bit7z_pres_dialogs::delete::DeleteDialog::open(cx, indices, h, repo);
                                }
                            }).detach();
                        }
                    }
                    "Escape" => {}
                    _ => {}
                }
            }))
            .on_action(cx.listener(|this: &mut RootView, _: &menu::OpenArchive, _window, cx| {
                if let Some(path) = bit7z_infra_platform::pick_archive_file() {
                    this.handle_open_archive(&path, None, cx);
                }
            }))
            .on_action(cx.listener(|_: &mut RootView, _: &menu::CreateArchive, _window, cx| {
                cx.spawn(async move |_, cx| {
                    bit7z_pres_dialogs::create::CreateArchiveDialog::open(cx, vec![]);
                }).detach();
            }))
            .on_action(cx.listener(|this: &mut RootView, _: &menu::AddFiles, _window, cx| {
                if let Some(ref handle) = this.state.archive {
                    let repo = this.controller.service();
                    let h = handle.clone();
                    cx.spawn(async move |_, cx| {
                        bit7z_pres_dialogs::add_files::AddFilesDialog::open(cx, bit7z_domain::archive::ArchiveFormat::SevenZip, Some(h), Some(repo), false);
                    }).detach();
                }
            }))
            .on_action(cx.listener(|this: &mut RootView, _: &menu::TestSelected, _window, cx| {
                let indices: Vec<u32> = this.state.selected_indices();
                let handle = this.state.archive.clone();
                let repo = this.controller.service();
                cx.spawn(async move |_, cx| {
                    if let Some(h) = handle {
                        bit7z_pres_dialogs::test::TestDialog::open_with_entries(cx, h, Some(indices), repo);
                    }
                }).detach();
            }))
            .on_action(cx.listener(|this: &mut RootView, _: &menu::TestAll, _window, cx| {
                let handle = this.state.archive.clone();
                let repo = this.controller.service();
                cx.spawn(async move |_, cx| {
                    if let Some(h) = handle {
                        bit7z_pres_dialogs::test::TestDialog::open_with_entries(cx, h, None, repo);
                    }
                }).detach();
            }))
            .on_action(cx.listener(|this: &mut RootView, _: &menu::CloseArchive, _window, cx| {
                if let Some(h) = this.state.archive.take() { this.controller.close_archive(h); }
                this.state = ArchiveState::new();
                this.sync_children(cx);
            }))
            .on_action(cx.listener(|this: &mut RootView, _: &menu::ShowProperties, _window, cx| {
                let (_indices, entries) = this.selected_entries_data();
                if !entries.is_empty() {
                    cx.spawn(async move |_, cx| {
                        bit7z_pres_dialogs::properties::PropertiesDialog::open_entries(entries, cx);
                    }).detach();
                } else if let Some(ref handle) = this.state.archive {
                    let path_str = handle.path.as_ref()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_default();
                    let controller = this.controller.clone();
                    let h = handle.clone();
                    cx.spawn(async move |_, cx| {
                        if let Ok(props) = controller.get_properties(&h) {
                            bit7z_pres_dialogs::properties::PropertiesDialog::open_archive(path_str, props, cx);
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
                        let repo = this.controller.service(); let handle = h.clone(); let indices: Vec<u32> = this.state.selected_indices();
                        cx.spawn(async move |_, cx| { bit7z_pres_dialogs::delete::DeleteDialog::open(cx, indices, handle, repo); }).detach();
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
                    bit7z_pres_dialogs::settings::SettingsDialog::open(cx);
                }).detach();
            }))
            .on_action(cx.listener(|_: &mut RootView, _: &menu::About, _window, cx| {
                bit7z_pres_dialogs::about::AboutDialog::open(cx);
            }))
            .on_action(cx.listener(|this: &mut RootView, _: &menu::ToggleSidebar, _window, cx| {
                this.sidebar_collapsed = !this.sidebar_collapsed;
                this.sync_children(cx);
                cx.notify();
            }))
            .on_action(cx.listener(|this: &mut RootView, _: &menu::SaveArchive, _window, cx| {
                this.handle_save_archive(cx);
            }))
            .on_action(cx.listener(|this: &mut RootView, _: &menu::UndoArchive, _window, cx| {
                this.handle_undo(cx);
            }))
            .on_action(cx.listener(|this: &mut RootView, _: &menu::RedoArchive, _window, cx| {
                this.handle_redo(cx);
            }))
            .on_action(cx.listener(|this: &mut RootView, _: &menu::ExtractArchive, _window, cx| {
                let (indices, entries) = this.selected_entries_data();
                if !entries.is_empty() {
                    if let Some(ref handle) = this.state.archive {
                        let handle = handle.clone();
                        let controller = this.controller.clone();
                        let busy = indices.clone();
                        cx.spawn(async move |_, cx| {
                            let rx = bit7z_pres_dialogs::extract::ExtractDialog::open(entries, cx);
                            use crossbeam_channel::TryRecvError;
                            loop {
                                match rx.try_recv() {
                                    Ok(bit7z_pres_dialogs::extract::ExtractDialogEvent::ExtractRequested { destination, overwrite_mode, .. }) => {
                                        let (tx, progress_rx) = bit7z_infra_progress::progress_channel();
                                        let cancel = Arc::new(AtomicBool::new(false));
                                        let paused = Arc::new(AtomicBool::new(false));
                                        bit7z_pres_dialogs::progress::ProgressDialog::open(cx, format!("Extracting..."), progress_rx, Some(cancel.clone()), Some(paused.clone()));
                                        let ctrl = controller.clone();
                                        let h = handle.clone();
                                        let dest = destination.clone();
                                        let idx = busy.clone();
                                        cx.background_spawn(async move {
                                            let _ = ctrl.extract(&h, &idx, &dest, overwrite_mode, Some(tx), Some(cancel), Some(paused));
                                        }).detach();
                                        break;
                                    }
                                    Ok(bit7z_pres_dialogs::extract::ExtractDialogEvent::Canceled) => break,
                                    Err(TryRecvError::Empty) => {
                                        cx.background_spawn(std::future::ready(())).await;
                                    }
                                    Err(TryRecvError::Disconnected) => break,
                                }
                            }
                        }).detach();
                    }
                }
            }))
            .child(MenuView::new(self.menu_bar.clone(), self.sidebar_collapsed))
            .child(
                h_flex().gap_2().p_2().w_full()
                    .child(
                        Button::new("open")
                            .icon(IconName::FolderOpen)
                            .tooltip("Open archive (Ctrl+O)")
                            .when(!compact, |b| b.label("Open"))
                            .on_click(|_, window, cx| window.dispatch_action(Box::new(menu::OpenArchive), cx))
                    )
                    .child(
                        Button::new("create")
                            .icon(IconName::Plus)
                            .tooltip("Create new archive (Ctrl+N)")
                            .when(!compact, |b| b.label("Create"))
                            .on_click(|_, window, cx| window.dispatch_action(Box::new(menu::CreateArchive), cx))
                    )
                    .child(
                        Button::new("add")
                            .icon(IconName::Plus)
                            .tooltip("Add files to archive")
                            .when(!compact, |b| b.label("Add"))
                            .disabled(!is_open)
                            .on_click(|_, window, cx| window.dispatch_action(Box::new(menu::AddFiles), cx))
                    )
                    .child(
                        Button::new("extract")
                            .icon(IconName::ChevronDown)
                            .tooltip("Extract selected files (Ctrl+E)")
                            .when(!compact, |b| b.label("Extract"))
                            .disabled(!is_ready || !has_sel)
                            .on_click(|_, window, cx| window.dispatch_action(Box::new(menu::ExtractArchive), cx))
                    )
                    .child(
                        Button::new("test")
                            .icon(IconName::PanelRightClose)
                            .tooltip("Test archive integrity (Ctrl+T)")
                            .when(!compact, |b| b.label("Test"))
                            .disabled(!is_open)
                            .on_click(|_, window, cx| window.dispatch_action(Box::new(menu::TestAll), cx))
                    )
                    .child(
                        Button::new("close")
                            .icon(IconName::Close)
                            .tooltip("Close archive")
                            .when(!compact, |b| b.label("Close"))
                            .disabled(!is_open)
                            .on_click(|_, window, cx| window.dispatch_action(Box::new(menu::CloseArchive), cx))
                    )
                    .when(is_open, |row| {
                        row.child(div().w(px(4.)))
                            .child(
                                Button::new("save")
                                    .icon(IconName::Check)
                                    .tooltip("Save changes (Ctrl+S)")
                                    .when(!compact, |b| b.label("Save"))
                                    .disabled(!has_unsaved)
                                    .on_click(|_, window, cx| window.dispatch_action(Box::new(menu::SaveArchive), cx))
                            )
                            .child(
                                Button::new("undo")
                                    .icon(IconName::Undo2)
                                    .tooltip("Undo (Ctrl+Z)")
                                    .when(!compact, |b| b.label("Undo"))
                                    .disabled(!can_undo)
                                    .on_click(|_, window, cx| window.dispatch_action(Box::new(menu::UndoArchive), cx))
                            )
                            .child(
                                Button::new("redo")
                                    .icon(IconName::Redo2)
                                    .tooltip("Redo (Ctrl+Y)")
                                    .when(!compact, |b| b.label("Redo"))
                                    .disabled(!can_redo)
                                    .on_click(|_, window, cx| window.dispatch_action(Box::new(menu::RedoArchive), cx))
                            )
                    })
                    .child(div().flex_1())
                    .child(
                        Button::new("settings")
                            .icon(IconName::Settings)
                            .tooltip("Settings")
                            .on_click(|_, window, cx| window.dispatch_action(Box::new(menu::ShowSettings), cx))
                    )
            )
            .child(div().flex_1().child(
                h_resizable("main-hz")
                    .child(
                        resizable_panel()
                        .when_else(self.sidebar_collapsed, |x|{
                            x.size(px(45.)).size_range(px(45.)..px(45.))
                        }, |x| {
                            x.size(px(255.)).size_range(px(200.)..px(320.))
                        })
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
                                    .child(PreviewPanelView::new(self.preview_data.clone(), self.preview_loading))
                            )
                    )
            ))
            .child(StatusBarView::new(status_text))
            .children(dialog_layer)
    }
}
