use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use bit7z_domain::archive::*;
use bit7z_domain::repository::*;
use bit7z_pres_view_models::app_state::ViewStatus;
use bit7z_pres_view_models::intent::Intent;
use bit7z_pres_view_models::AppState;
use gpui::*;

use crate::archive_browser::ArchiveBrowser;
use crate::archive_file_list::ArchiveFileList;
use crate::preview_panel::PreviewPanel;
use crate::root::RootView;

// TODO(integration): IPC polling moved to runtime/gui

#[derive(Clone)]
pub struct UseCases {
    pub repo: Arc<dyn ArchiveRepository>,
}

impl UseCases {
    pub fn new(repo: Arc<dyn ArchiveRepository>) -> Self { Self { repo } }

    pub fn extract(&self, h: &ArchiveHandle, idx: &[u32], dest: &Path, om: OverwriteMode) -> Result<(), ArchiveError> {
        let req = ExtractRequest { indices: idx.to_vec(), dest: dest.to_path_buf(), overwrite: om };
        self.repo.extract(h, &req, &OpCtx { cancel: CancellationToken::new(), pause: PauseToken::new(), progress: Arc::new(NoopSink) }).map(|_| ())
    }

    pub fn delete(&self, h: &ArchiveHandle, idx: &[u32]) -> Result<(), ArchiveError> {
        let mut cs = ChangeSet::new();
        for &i in idx { cs.delete(i); }
        let plan = self.repo.plan(h, &cs)?;
        self.repo.apply(h, &plan, &WriteOptions { cancel: Arc::new(AtomicBool::new(false)), paused: Arc::new(AtomicBool::new(false)), notifier: Arc::new(NoopNotifier) }, &OpCtx { cancel: CancellationToken::new(), pause: PauseToken::new(), progress: Arc::new(NoopSink) })
    }

    pub fn test(&self, h: &ArchiveHandle) -> Result<TestReport, ArchiveError> {
        let page = self.repo.list(h, 0..1)?;
        let all: Vec<u32> = (0..page.total.unwrap_or(0) as u32).collect();
        self.repo.test(h, &all, &OpCtx { cancel: CancellationToken::new(), pause: PauseToken::new(), progress: Arc::new(NoopSink) })
    }

    pub fn test_selected(&self, h: &ArchiveHandle, idx: &[u32]) -> Result<TestReport, ArchiveError> {
        self.repo.test(h, idx, &OpCtx { cancel: CancellationToken::new(), pause: PauseToken::new(), progress: Arc::new(NoopSink) })
    }

    pub fn open_entry(&self, h: &ArchiveHandle, idx: u32) -> Result<(), ArchiveError> {
        bit7z_app_archive::open_entry::OpenEntryUseCase::new(self.repo.clone()).execute(h, idx)
    }

    pub fn properties(&self, h: &ArchiveHandle) -> Result<ArchiveProperties, ArchiveError> { self.repo.properties(h) }
    pub fn close(&self, h: &ArchiveHandle) { self.repo.close(h); }

    pub fn list_directory(&self, h: &ArchiveHandle, path: &str) -> Result<Vec<ArchiveEntry>, ArchiveError> {
        self.repo.list_dir(h, path, 0..usize::MAX).map(|p| p.items)
    }

    pub fn preview(&self, h: &ArchiveHandle, idx: u32, max: usize) -> Result<bit7z_app_preview::PreviewData, ArchiveError> {
        bit7z_app_preview::PreviewEntryUseCase::new(self.repo.clone()).execute(h, idx, max)
    }

    pub fn new_file(&self, h: &ArchiveHandle) -> Result<(), ArchiveError> {
        let _ = bit7z_app_archive::new_file::new_file_and_add(self.repo.clone(), h, "new_file.txt", None);
        Ok(())
    }
}

impl EventEmitter<Intent> for AppShell {}
impl EventEmitter<bit7z_infra_events::ArchiveVmEvent> for AppShell {}

pub struct AppShell {
    pub state: AppState,
    pub use_cases: Arc<UseCases>,
    pub root_view: Entity<RootView>,
    pub preview_panel: Entity<PreviewPanel>,
    pub sidebar_collapsed: bool,
}

impl AppShell {
    pub fn new(
        window: &mut Window, cx: &mut Context<Self>, repo: Arc<dyn ArchiveRepository>,
        open_path: Option<String>, open_password: Option<String>,
    ) -> Self {
        let state = AppState::new();
        let use_cases = Arc::new(UseCases::new(repo));
        let preview_panel = cx.new(|_| PreviewPanel::new());
        let p2 = preview_panel.clone();
        let archive_browser = cx.new(|cx| ArchiveBrowser::new(window, cx));
        let entry_list = cx.new(|cx| ArchiveFileList::new(window, cx));
        let app_shell_weak = cx.entity().downgrade();
        let root_view = cx.new(move |cx| RootView::new(app_shell_weak, p2, archive_browser, entry_list, cx));

        cx.subscribe::<RootView, Intent>(&root_view, move |this, _, intent, cx| this.on_intent(intent.clone(), cx)).detach();

        Self { state, use_cases, root_view, preview_panel, sidebar_collapsed: false }
    }

    fn on_intent(&mut self, intent: Intent, cx: &mut Context<Self>) {
        match intent {
            Intent::Extract => self.handle_extract(cx),
            Intent::DeleteSelected => self.handle_delete(cx),
            Intent::ShowProperties => self.handle_properties(cx),
            Intent::TestSelected => self.handle_test(cx, true),
            Intent::TestAll => self.handle_test(cx, false),
            Intent::OpenEntry => self.handle_open_entry(cx),
            Intent::RequestNewFolder => cx.emit(bit7z_infra_events::ArchiveVmEvent::RequestNewFolder),
            Intent::RequestNewFile => { if let Some(ref h) = self.state.handle { let uc = self.use_cases.clone(); let handle = h.clone(); cx.background_spawn(async move { let _ = uc.new_file(&handle); }).detach(); } }
            Intent::RequestAddFiles => self.handle_add_files(cx),
            Intent::RequestOpenArchive => { if let Some(path) = bit7z_infra_platform::pick_archive_file() { self.handle_open_archive(&path, None, cx); } }
            Intent::RequestCreateArchive => { cx.spawn(async move |_, cx| { bit7z_pres_dialogs::create::CreateArchiveDialog::open(cx, vec![]); }).detach(); }
            Intent::CloseArchive => self.handle_close(cx),
            Intent::Refresh => { if self.state.handle.is_some() { self.state.directory_cache.remove(&self.state.current_path); } }
            _ => {}
        }
    }

    fn handle_extract(&mut self, cx: &mut Context<Self>) {
        let indices = self.state.selected_indices();
        let entries = self.state.selected_entries();
        if entries.is_empty() || self.state.handle.is_none() { return; }
        let handle = self.state.handle.clone().unwrap();
        let uc = self.use_cases.clone();
        let busy = indices.clone();
        cx.spawn(async move |this, cx| {
            let rx = bit7z_pres_dialogs::extract::ExtractDialog::open(entries, cx);
            use crossbeam_channel::TryRecvError;
            loop {
                match rx.try_recv() {
                    Ok(bit7z_pres_dialogs::extract::ExtractDialogEvent::ExtractRequested { destination, overwrite_mode, .. }) => {
                        let (tx, progress_rx) = bit7z_infra_progress::progress_channel();
                        let cancel = Arc::new(AtomicBool::new(false));
                        let paused = Arc::new(AtomicBool::new(false));
                        bit7z_pres_dialogs::progress::ProgressDialog::open(cx, "Extracting...".into(), progress_rx, Some(cancel.clone()), Some(paused.clone()));
                        let h = handle.clone(); let dest = destination.clone(); let idx = busy.clone(); let u = uc.clone();
                        cx.background_spawn(async move {
                            let _ = u.extract(&h, &idx, &dest, overwrite_mode);
                            let _ = tx.send(ProgressUpdate { file_current: 0, file_total: 0, current_file: None, items_done: 0, items_total: 0, bytes_done: 0, bytes_total: 0, error: None });
                        }).detach();
                        break;
                    }
                    Ok(bit7z_pres_dialogs::extract::ExtractDialogEvent::Canceled) => break,
                    Err(TryRecvError::Empty) => { cx.background_spawn(std::future::ready(())).await; }
                    Err(TryRecvError::Disconnected) => break,
                }
            }
            let _ = this.update(cx, |_, cx| cx.notify());
        }).detach();
    }

    fn handle_delete(&mut self, cx: &mut Context<Self>) {
        if let (Some(h), false) = (self.state.handle.clone(), self.state.selected_indices().is_empty()) {
            let indices = self.state.selected_indices();
            let uc = self.use_cases.clone();
            cx.spawn(async move |_, cx| { bit7z_pres_dialogs::delete::DeleteDialog::open(cx, indices, h, uc.repo.clone()); }).detach();
        }
    }

    fn handle_properties(&mut self, cx: &mut Context<Self>) {
        let entries = self.state.selected_entries();
        if !entries.is_empty() {
            cx.spawn(async move |_, cx| { bit7z_pres_dialogs::properties::PropertiesDialog::open_entries(entries, cx); }).detach();
        } else if let Some(ref h) = self.state.handle {
            let path_str = h.path().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
            let uc = self.use_cases.clone(); let handle = h.clone();
            cx.spawn(async move |_, cx| { if let Ok(props) = uc.properties(&handle) { bit7z_pres_dialogs::properties::PropertiesDialog::open_archive(path_str, props, cx); } }).detach();
        }
    }

    fn handle_test(&mut self, cx: &mut Context<Self>, selected: bool) {
        if let Some(h) = self.state.handle.clone() {
            let indices = if selected { Some(self.state.selected_indices()) } else { None };
            let uc = self.use_cases.clone();
            cx.spawn(async move |_, cx| { bit7z_pres_dialogs::test::TestDialog::open_with_entries(cx, h, indices, uc.repo.clone()); }).detach();
        }
    }

    fn handle_open_entry(&mut self, cx: &mut Context<Self>) {
        if let (Some(ref h), Some(idx)) = (self.state.handle.clone(), self.state.first_selected_index()) {
            if self.state.displayed_entries().iter().find(|e| e.original_index == idx).map_or(false, |e| e.is_directory) {
                let name = self.state.displayed_entries().iter().find(|e| e.original_index == idx).map(|e| e.display_name.clone()).unwrap();
                self.state.navigate_into(&name); self.sync_root_view(cx); self.load_current_directory(cx);
            } else {
                let uc = self.use_cases.clone(); let handle = h.clone();
                cx.background_spawn(async move { let _ = uc.open_entry(&handle, idx); }).detach();
            }
        }
    }

    fn handle_add_files(&mut self, cx: &mut Context<Self>) {
        if let Some(ref h) = self.state.handle {
            let uc = self.use_cases.clone(); let handle = h.clone();
            cx.spawn(async move |_, cx| { bit7z_pres_dialogs::add_files::AddFilesDialog::open(cx, bit7z_domain::archive::ArchiveFormat::SevenZip, Some(handle), Some(uc.repo.clone()), false); }).detach();
        }
    }

    pub fn handle_open_archive(&mut self, path: &Path, password: Option<String>, cx: &mut Context<Self>) {
        self.state.status = ViewStatus::Loading;
        self.sync_root_view(cx);
        let uc = self.use_cases.clone();
        let path_buf = path.to_path_buf();
        let path_string = path.to_string_lossy().to_string();
        let pw = password.map(|s| Password::new(s));
        let pw_clone = pw.clone();
        let path_for_password = path_string.clone();

        let bg_task = cx.background_spawn(async move { uc.repo.open(&path_buf, pw.as_ref()) });
        cx.spawn(async move |this, cx| {
            let this_strong = this.clone().upgrade();
            match bg_task.await {
                Ok(handle) => {
                    this.update(cx, |this, cx| {
                        this.state.handle = Some(handle); this.state.archive_password = pw_clone;
                        this.state.current_path = String::new(); this.state.path_history.clear(); this.state.directory_cache.clear();
                        bit7z_pres_settings::SettingsStore::get_mut(cx).update_and_save(|p| p.archive.add_recent(path_string.clone()));
                        this.load_current_directory(cx);
                    }).ok();
                }
                Err(ArchiveError::EncryptedArchiveRequiresPassword) => {
                    this.update(cx, |this, cx| { this.state.status = ViewStatus::Empty; }).ok();
                    if let Some(this_strong) = this_strong.clone() {
                        Self::password_prompt(path_for_password, this_strong, cx);
                    }
                }
                Err(e) => { this.update(cx, |this, _| { this.state.status = ViewStatus::Error(e.to_string()); }).ok(); }
            }
        }).detach();
    }

    fn password_prompt(path: String, this: Entity<Self>, cx: &mut AsyncApp) {
        cx.spawn(async move |cx| {
            let rx = bit7z_pres_dialogs::password::PasswordDialog::open(path.clone(), cx);
            use crossbeam_channel::TryRecvError;
            loop {
                match rx.try_recv() {
                    Ok(result) => {
                        use bit7z_pres_dialogs::password::PasswordResult;
                        if let PasswordResult::Submitted(pw) = result {
                            this.update(cx, |this, cx| this.handle_open_archive(Path::new(&path), Some(pw), cx));
                        }
                        break;
                    }
                    Err(TryRecvError::Empty) => { cx.background_spawn(std::future::ready(())).await; }
                    Err(TryRecvError::Disconnected) => break,
                }
            }
        }).detach();
    }

    pub fn load_current_directory(&mut self, cx: &mut Context<Self>) {
        if let (Some(ref h), _) = (self.state.handle.clone(), ()) {
            let handle = h.clone(); let path = self.state.current_path.clone(); let uc = self.use_cases.clone();
            cx.spawn(async move |this, cx| {
                match uc.list_directory(&handle, &path) {
                    Ok(entries) => { let _ = this.update(cx, |this, cx| { this.state.directory_cache.insert(path, entries); this.state.reapply_filter_and_sort(); this.state.status = ViewStatus::Ready; this.sync_root_view(cx); cx.notify(); }); }
                    Err(e) => { let _ = this.update(cx, |this, _| { this.state.status = ViewStatus::Error(e.to_string()); }); }
                }
            }).detach();
        }
    }

    pub fn navigate_into(&mut self, dir: &str, cx: &mut Context<Self>) { self.state.navigate_into(dir); self.sync_root_view(cx); self.load_current_directory(cx); }
    pub fn set_filter(&mut self, text: &str, cx: &mut Context<Self>) { self.state.set_filter(text); self.sync_root_view(cx); }
    pub fn toggle_sidebar(&mut self, cx: &mut Context<Self>) { self.sidebar_collapsed = !self.sidebar_collapsed; self.sync_root_view(cx); cx.notify(); }

    fn handle_close(&mut self, cx: &mut Context<Self>) {
        if let Some(ref h) = self.state.handle { self.use_cases.close(h); }
        self.state.handle = None; self.state.properties = None; self.state.directory_cache.clear(); self.state.level_entries.clear();
        self.state.current_path = String::new(); self.state.path_history.clear(); self.state.status = ViewStatus::Empty;
        self.sync_root_view(cx);
    }

    pub fn sync_root_view(&self, cx: &mut Context<Self>) {
        self.root_view.update(cx, |rv, cx| rv.sync_state(&self.state, self.sidebar_collapsed, cx));
    }
}
