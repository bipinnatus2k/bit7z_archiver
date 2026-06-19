use crate::application::checksum::{CalculateChecksumUseCase, ChecksumAlgorithm};
use crate::application::events::{ArchiveVmEvent, ChecksumAlgorithm as EventChecksumAlgorithm};
use crate::application::new_folder::new_folder;
use crate::application::new_file::new_file_and_add;
use crate::application::open::OpenArchiveUseCase;
use crate::application::open_entry::OpenEntryUseCase;
use crate::application::progress::progress_channel;
use crate::application::{add_to::AddToArchiveUseCase, delete::DeleteEntriesUseCase, rename::RenameEntryUseCase, test::TestEntriesUseCase};
use crate::domain::archive::*;
use crate::domain::preferences::{Preferences, PreferencesRepoGlobal};
use crate::domain::repository::*;
use crate::adapters::view_models::progress_vm::ProgressState;
use gpui::{EventEmitter, *};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};


/// A single item at the current navigation level.
#[derive(Clone, Debug)]
pub struct LevelEntry {
    pub display_name: String,
    pub is_directory: bool,
    /// Original archive index (for preview / extraction).
    pub original_index: u32,
    pub size: u64,
    pub compressed_size: u64,
    pub modified: Option<chrono::DateTime<chrono::Utc>>,
}


impl EventEmitter<ArchiveVmEvent> for ArchiveViewModel {}

pub struct ArchiveViewModel {
    repo: Arc<dyn ArchiveRepository>,
    pub archive: Option<ArchiveHandle>,
    pub properties: Option<ArchiveProperties>,
    /// Per-directory cache: path -> entries at that level.
    directory_cache: HashMap<String, Vec<ArchiveEntry>>,
    pub selection: HashSet<u32>,
    pub filter_text: String,
    pub sort_column: u32,
    pub sort_ascending: bool,
    pub status: ViewStatus,
    pub level_entries: Vec<LevelEntry>,
    pub current_path: String,
    pub path_history: Vec<String>,
    selection_anchor: Option<u32>,
}

pub enum ViewStatus {
    Empty,
    Loading,
    Ready,
    Error(String),
}

impl ArchiveViewModel {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let repo = cx.global::<RepoGlobal>().0.clone();
        Self {
            repo,
            archive: None,
            properties: None,
            directory_cache: HashMap::new(),
            selection: HashSet::new(),
            filter_text: String::new(),
            sort_column: 0,
            sort_ascending: true,
            status: ViewStatus::Empty,
            level_entries: Vec::new(),
            current_path: String::new(),
            path_history: Vec::new(),
            selection_anchor: None,
        }
    }

    pub fn open_archive(
        &mut self,
        path: &std::path::Path,
        password: Option<String>,
        cx: &mut Context<Self>,
    ) {
        self.status = ViewStatus::Loading;
        cx.notify();
        cx.emit(ArchiveVmEvent::SelectionChanged(None));

        let repo = self.repo.clone();
        let path_buf = path.to_path_buf();
        let path_string = path.to_string_lossy().to_string();

        let use_case = OpenArchiveUseCase::new(repo);
        let pw = password.map(Password::new);
        let bg_task = cx.background_spawn(async move {
            use_case.execute(&path_buf, pw.as_ref())
        });

        cx.spawn(async move |this, cx| {
            let result = bg_task.await;
            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok(output) => {
                        this.archive = Some(output.handle);
                        this.properties = Some(output.properties);
                        this.current_path = String::new();
                        this.path_history.clear();
                        this.directory_cache.clear();
                        // Save recent files
                        let mut prefs = cx.global::<Preferences>().clone();
                        prefs.archive.add_recent(path_string);
                        cx.set_global(prefs);
                        if let Err(e) = cx.global::<PreferencesRepoGlobal>().0.save(&cx.global::<Preferences>()) {
                            log::warn!("Failed to persist preferences: {}", e);
                        }
                        // Load root directory
                        this.load_current_directory(cx);
                    }
                    Err(e) => {
                        this.status = ViewStatus::Error(e.to_string());
                        cx.notify();
                    }
                }
            });
        }).detach();
    }

    /// Ensure entries for the current path are loaded, then filter/sort.
    fn load_current_directory(&mut self, cx: &mut Context<Self>) {
        let key = self.current_path.clone();
        if let Some(entries) = self.directory_cache.get(&key) {
            let snapshot: Vec<ArchiveEntry> = entries.clone();
            self.status = ViewStatus::Ready;
            self.apply_filter_and_sort(&snapshot);
            cx.emit(ArchiveVmEvent::SelectionChanged(None));
            cx.notify();
            return;
        }

        // Cache miss — load from repository
        self.status = ViewStatus::Loading;
        cx.notify();

        let repo = self.repo.clone();
        let handle = self.archive.clone().expect("no open archive");
        let path = self.current_path.clone();

        let bg_task = cx.background_spawn(async move {
            repo.list_directory(&handle, &path)
        });

        let key2 = key.clone();
        cx.spawn(async move |this, cx| {
            let result = bg_task.await;
            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok(mut entries) => {
                        // Sort directory entries: folders first, then alphabetical
                        entries.sort_by(|a, b| {
                            if a.is_directory != b.is_directory {
                                return b.is_directory.cmp(&a.is_directory);
                            }
                            a.name.to_lowercase().cmp(&b.name.to_lowercase())
                        });
                        this.directory_cache.insert(key2, entries);
                        if let Some(cached) = this.directory_cache.get(&this.current_path) {
                            let snapshot: Vec<ArchiveEntry> = cached.clone();
                            this.status = ViewStatus::Ready;
                            this.apply_filter_and_sort(&snapshot);
                        }
                    }
                    Err(e) => {
                        // Navigate back on failure
                        this.navigate_up_internal();
                        this.status = ViewStatus::Error(e.to_string());
                    }
                }
                cx.emit(ArchiveVmEvent::SelectionChanged(None));
                cx.notify();
            });
        }).detach();
    }

    /// Apply current filter and sort to a set of directory entries.
    fn apply_filter_and_sort(&mut self, entries: &[ArchiveEntry]) {
        let filter_lower = self.filter_text.to_lowercase();
        let mut items: Vec<LevelEntry> = Vec::new();

        for entry in entries {
            if !self.filter_text.is_empty()
                && !entry.path.to_lowercase().contains(&filter_lower)
            {
                continue;
            }

            items.push(LevelEntry {
                display_name: entry.name.clone(),
                is_directory: entry.is_directory,
                original_index: entry.original_index,
                size: entry.size,
                compressed_size: entry.compressed_size,
                modified: entry.modified,
            });
        }

        items.sort_by(|a, b| {
            if a.is_directory != b.is_directory {
                return if self.sort_ascending { b.is_directory.cmp(&a.is_directory) } else { a.is_directory.cmp(&b.is_directory) };
            }
            let c = match self.sort_column {
                1 => a.size.cmp(&b.size),
                2 => a.compressed_size.cmp(&b.compressed_size),
                3 => Self::ratio_key(a).cmp(&Self::ratio_key(b)),
                _ => a.display_name.to_lowercase().cmp(&b.display_name.to_lowercase()),
            };
            if self.sort_ascending { c } else { c.reverse() }
        });
        self.level_entries = items;
    }

    fn ratio_key(e: &LevelEntry) -> u64 {
        if e.size == 0 { 0 }
        else { ((1.0 - e.compressed_size as f64 / e.size as f64) * 10000.0) as u64 }
    }

    // Helper: pop history without triggering load
    fn navigate_up_internal(&mut self) {
        if let Some(prev) = self.path_history.pop() {
            self.current_path = prev;
        } else {
            self.current_path = String::new();
        }
    }

    pub fn navigate_into(&mut self, dir_name: &str, cx: &mut Context<Self>) {
        self.path_history.push(self.current_path.clone());
        self.current_path = if self.current_path.is_empty() {
            format!("{}/", dir_name)
        } else {
            format!("{}{}/", self.current_path, dir_name)
        };
        self.selection.clear();
        self.selection_anchor = None;
        self.load_current_directory(cx);
    }

    pub fn navigate_up(&mut self, cx: &mut Context<Self>) {
        if !self.path_history.is_empty() {
            self.current_path = self.path_history.pop().unwrap();
            self.selection.clear();
            self.selection_anchor = None;
            self.load_current_directory(cx);
        }
    }

    pub fn navigate_root(&mut self, cx: &mut Context<Self>) {
        self.path_history.clear();
        self.current_path = String::new();
        self.selection.clear();
        self.selection_anchor = None;
        self.load_current_directory(cx);
    }

    pub fn handle_level_click(&mut self, level_idx: usize, modifiers: &Modifiers, cx: &mut Context<Self>) {
        if let Some(entry) = self.level_entries.get(level_idx) {
            if entry.is_directory {
                let name = entry.display_name.clone();
                self.navigate_into(&name, cx);
            } else {
                self.update_selection(entry.original_index, modifiers);
                if let Some(archive) = &self.archive {
                    let first = self.selection.iter().next().copied();
                    cx.emit(ArchiveVmEvent::SelectionChanged(
                        first.map(|idx| (archive.clone(), idx)),
                    ));
                }
                cx.notify();
            }
        }
    }

    pub fn select_all(&mut self, cx: &mut Context<Self>) {
        self.selection.clear();
        for entry in &self.level_entries {
            self.selection.insert(entry.original_index);
        }
        self.selection_anchor = None;
        if let Some(archive) = &self.archive {
            let first = self.selection.iter().next().copied();
            cx.emit(ArchiveVmEvent::SelectionChanged(
                first.map(|idx| (archive.clone(), idx)),
            ));
        }
        cx.notify();
    }

    pub fn clear_selection(&mut self, cx: &mut Context<Self>) {
        self.selection.clear();
        self.selection_anchor = None;
        cx.emit(ArchiveVmEvent::SelectionChanged(None));
        cx.notify();
    }

    pub fn update_selection(&mut self, index: u32, modifiers: &Modifiers) {
        if modifiers.shift {
            let anchor = self.selection_anchor.unwrap_or(index);
            let min = anchor.min(index);
            let max = anchor.max(index);
            if self.selection.len() == 1 && self.selection.contains(&min) {
                let existing = *self.selection.iter().next().unwrap();
                self.selection.clear();
                for i in min..=max {
                    self.selection.insert(i);
                }
                if !self.selection.contains(&existing) {
                    self.selection.insert(existing);
                }
            } else {
                self.selection.clear();
                for i in min..=max {
                    self.selection.insert(i);
                }
            }
        } else if modifiers.control {
            if self.selection.contains(&index) {
                self.selection.remove(&index);
            } else {
                self.selection.insert(index);
            }
            self.selection_anchor = Some(index);
        } else {
            self.selection.clear();
            self.selection.insert(index);
            self.selection_anchor = Some(index);
        }
    }

    pub fn select(&mut self, index: u32, modifiers: &Modifiers, cx: &mut Context<Self>) {
        self.update_selection(index, modifiers);
        if let Some(archive) = &self.archive {
            let first = self.selection.iter().next().copied();
            cx.emit(ArchiveVmEvent::SelectionChanged(
                first.map(|idx| (archive.clone(), idx)),
            ));
        }
        cx.notify();
    }

    fn reapply_filter_and_sort(&mut self) {
        let snapshot = self.directory_cache.get(&self.current_path)
            .cloned().unwrap_or_default();
        self.apply_filter_and_sort(&snapshot);
    }

    pub fn sort_by(&mut self, column: u32, cx: &mut Context<Self>) {
        if self.sort_column == column {
            self.sort_ascending = !self.sort_ascending;
        } else {
            self.sort_column = column;
            self.sort_ascending = true;
        }
        self.reapply_filter_and_sort();
        cx.notify();
    }

    pub fn apply_sort(&mut self, column: u32, ascending: bool) {
        self.sort_column = column;
        self.sort_ascending = ascending;
        self.reapply_filter_and_sort();
    }

    pub fn set_filter(&mut self, text: &str, cx: &mut Context<Self>) {
        self.filter_text = text.to_string();
        self.selection.clear();
        self.selection_anchor = None;
        self.reapply_filter_and_sort();
        cx.notify();
    }

    /// Cached subdirectories for the current path (sidebar).
    pub fn current_subdirs(&self) -> Vec<String> {
        self.directory_cache.get(&self.current_path)
            .map(|entries| {
                let mut dirs: Vec<String> = entries.iter()
                    .filter(|e| e.is_directory)
                    .map(|e| e.name.clone())
                    .collect();
                dirs.sort();
                dirs
            })
            .unwrap_or_default()
    }

    pub fn displayed_entries(&self) -> &[LevelEntry] {
        &self.level_entries
    }

    pub fn request_extract(&mut self, cx: &mut Context<Self>) {
        if matches!(self.status, ViewStatus::Ready) && !self.selection.is_empty() {
            cx.emit(ArchiveVmEvent::RequestShowExtract);
        }
    }

    pub fn request_create(&mut self, cx: &mut Context<Self>) {
        cx.emit(ArchiveVmEvent::RequestShowCreate);
    }

    pub fn request_test(&mut self, cx: &mut Context<Self>) {
        if matches!(self.status, ViewStatus::Ready) {
            cx.emit(ArchiveVmEvent::RequestTest);
        }
    }

    pub fn request_add_files(&mut self, cx: &mut Context<Self>) {
        cx.emit(ArchiveVmEvent::RequestShowAdd);
    }

    pub fn request_show_settings(&mut self, cx: &mut Context<Self>) {
        cx.emit(ArchiveVmEvent::RequestShowSettings);
    }

    pub fn invert_selection(&mut self, cx: &mut Context<Self>) {
        let indices: Vec<u32> = self.level_entries.iter().map(|e| e.original_index).collect();
        for idx in indices {
            if self.selection.contains(&idx) {
                self.selection.remove(&idx);
            } else {
                self.selection.insert(idx);
            }
        }
        self.selection_anchor = None;
        if let Some(archive) = &self.archive {
            let first = self.selection.iter().next().copied();
            cx.emit(ArchiveVmEvent::SelectionChanged(
                first.map(|idx| (archive.clone(), idx)),
            ));
        }
        cx.notify();
    }

    /// Collect ArchiveEntry for all selected (original) indices.
    pub fn selected_entries(&self) -> Vec<ArchiveEntry> {
        if self.selection.is_empty() { return vec![]; }
        self.directory_cache.values()
            .flat_map(|entries| entries.iter())
            .filter(|e| self.selection.contains(&e.original_index))
            .cloned()
            .collect()
    }

    pub fn close(&mut self, cx: &mut Context<Self>) {
        if let Some(archive) = self.archive.take() {
            self.repo.close(archive);
        }
        self.directory_cache.clear();
        self.selection.clear();
        self.selection_anchor = None;
        self.filter_text.clear();
        self.level_entries.clear();
        self.current_path.clear();
        self.path_history.clear();
        self.status = ViewStatus::Empty;
        cx.notify();
    }

    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        if self.archive.is_some() {
            self.directory_cache.remove(&self.current_path);
            self.load_current_directory(cx);
        }
    }

    pub fn delete_selected(&mut self, cx: &mut Context<Self>) {
        if !matches!(self.status, ViewStatus::Ready) || self.selection.is_empty() {
            return;
        }
        let indices: Vec<u32> = self.selection.iter().copied().collect();
        cx.emit(ArchiveVmEvent::RequestDelete);
        let repo = self.repo.clone();
        let mut handle = self.archive.clone().unwrap();
        let count = indices.len() as u64;

        let (tx, rx) = progress_channel();
        cx.update_global::<ProgressState, _>(|state, _cx| {
            state.is_active = true;
            state.is_complete = false;
            state.is_paused = false;
            state.receiver = Some(Arc::new(Mutex::new(rx)));
            state.message = format!("Deleting {} entries...", count);
            state.current = 0;
            state.total = count;
            state.error = None;
        });

        cx.background_spawn(async move {
            let uc = DeleteEntriesUseCase::new(repo);
            uc.execute(&mut handle, &indices, Some(tx))
        }).detach();

        cx.spawn(async move |this, cx| {
            loop {
                let done = cx.update_global::<ProgressState, _>(|state, _| {
                    let _ = state.poll();
                    state.is_complete
                });
                if done {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(80)).await;
            }
            let _ = this.update(cx, |this, cx| {
                this.refresh(cx);
                cx.emit(ArchiveVmEvent::RefreshListing);
            });
        }).detach();
    }

    pub fn add_files(&mut self, cx: &mut Context<Self>) {
        if !matches!(self.status, ViewStatus::Ready) {
            return;
        }
        let paths = match crate::adapters::platform::pick_files() {
            Some(p) if !p.is_empty() => p,
            _ => return,
        };
        let count = paths.len() as u64;
        cx.emit(ArchiveVmEvent::RequestAddFiles);
        let repo = self.repo.clone();
        let mut handle = self.archive.clone().unwrap();

        let (tx, rx) = progress_channel();
        cx.update_global::<ProgressState, _>(|state, _cx| {
            state.is_active = true;
            state.is_complete = false;
            state.is_paused = false;
            state.receiver = Some(Arc::new(Mutex::new(rx)));
            state.message = format!("Adding {} files...", count);
            state.current = 0;
            state.total = count;
            state.error = None;
        });

        cx.background_spawn(async move {
            let uc = AddToArchiveUseCase::new(repo);
            uc.execute(&mut handle, &paths, Some(tx))
        }).detach();

        cx.spawn(async move |this, cx| {
            loop {
                let done = cx.update_global::<ProgressState, _>(|state, _| {
                    let _ = state.poll();
                    state.is_complete
                });
                if done {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(80)).await;
            }
            let _ = this.update(cx, |this, cx| {
                this.refresh(cx);
                cx.emit(ArchiveVmEvent::RefreshListing);
            });
        }).detach();
    }

    pub fn rename_entry(&mut self, index: u32, new_name: &str, cx: &mut Context<Self>) {
        if !matches!(self.status, ViewStatus::Ready) {
            return;
        }
        let repo = self.repo.clone();
        let mut handle = self.archive.clone().unwrap();
        let name = new_name.to_string();
        let idx = index;

        cx.background_spawn(async move {
            let uc = RenameEntryUseCase::new(repo);
            uc.execute(&mut handle, idx, &name)
        }).detach();

        cx.spawn(async move |this, cx| {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            let _ = this.update(cx, |this, cx| {
                this.refresh(cx);
            });
        }).detach();
    }

    pub fn test_selected(&mut self, cx: &mut Context<Self>) {
        if !matches!(self.status, ViewStatus::Ready) {
            return;
        }
        let indices: Option<Vec<u32>> = if self.selection.is_empty() {
            None
        } else {
            Some(self.selection.iter().copied().collect())
        };
        let count = indices.as_ref().map(|v| v.len()).unwrap_or(0) as u64;
        cx.emit(ArchiveVmEvent::RequestTestEntries { selected_only: true });
        let repo = self.repo.clone();
        let handle = self.archive.clone().unwrap();

        let (tx, rx) = progress_channel();
        cx.update_global::<ProgressState, _>(|state, _cx| {
            state.is_active = true;
            state.is_complete = false;
            state.is_paused = false;
            state.receiver = Some(Arc::new(Mutex::new(rx)));
            state.message = String::from("Testing archive...");
            state.current = 0;
            state.total = count;
            state.error = None;
        });

        cx.background_spawn(async move {
            let uc = TestEntriesUseCase::new(repo);
            uc.execute(&handle, indices.as_deref(), Some(tx))
        }).detach();

        cx.spawn(async move |this, cx| {
            loop {
                let done = cx.update_global::<ProgressState, _>(|state, _| {
                    let _ = state.poll();
                    state.is_complete
                });
                if done {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(80)).await;
            }
            let _ = this.update(cx, |this, cx| {
                this.refresh(cx);
                cx.emit(ArchiveVmEvent::RefreshListing);
            });
        }).detach();
    }

    pub fn first_selected_index(&self) -> Option<u32> {
        self.selection.iter().next().copied()
    }

    pub fn open_entry(&mut self, cx: &mut Context<Self>) {
        let idx = match self.first_selected_index() {
            Some(i) => i,
            None => {
                log::info!("Open entry: no selection");
                return;
            }
        };
        let handle = match self.archive.as_ref() {
            Some(h) => h.clone(),
            None => {
                log::info!("Open entry: no archive open");
                return;
            }
        };
        let repo = self.repo.clone();
        cx.background_spawn(async move {
            let uc = OpenEntryUseCase::new(repo);
            if let Err(e) = uc.execute(&handle, idx) {
                log::error!("Failed to open entry: {}", e);
            }
        }).detach();
    }

    pub fn preview_entry(&mut self, cx: &mut Context<Self>) {
        let idx = match self.first_selected_index() {
            Some(i) => i,
            None => {
                log::info!("View entry: no selection");
                return;
            }
        };
        let handle = match self.archive.as_ref() {
            Some(h) => h.clone(),
            None => {
                log::info!("View entry: no archive open");
                return;
            }
        };
        let repo = self.repo.clone();
        cx.background_spawn(async move {
            let uc = OpenEntryUseCase::new(repo);
            if let Err(e) = uc.execute(&handle, idx) {
                log::error!("Failed to view entry: {}", e);
            }
        }).detach();
    }

    pub fn edit_entry(&mut self, cx: &mut Context<Self>) {
        let idx = match self.first_selected_index() {
            Some(i) => i,
            None => {
                log::info!("Edit entry: no selection");
                return;
            }
        };
        let handle = match self.archive.as_ref() {
            Some(h) => h.clone(),
            None => {
                log::info!("Edit entry: no archive open");
                return;
            }
        };
        let repo = self.repo.clone();
        let mut handle = handle;
        cx.background_spawn(async move {
            if let Err(e) = new_file_and_add(repo, &mut handle, "new_file.txt") {
                log::error!("Failed to edit entry: {}", e);
            }
        }).detach();
    }

    pub fn show_properties(&mut self, cx: &mut Context<Self>) {
        let entries = self.selected_entries();
        if entries.is_empty() {
            log::info!("Properties: no selection");
            return;
        }
        cx.emit(ArchiveVmEvent::RequestProperties);
    }

    pub fn test_all(&mut self, cx: &mut Context<Self>) {
        self.test_selected(cx);
    }

    pub fn request_new_folder(&mut self, cx: &mut Context<Self>) {
        cx.emit(ArchiveVmEvent::RequestNewFolder);
    }

    pub fn request_new_file(&mut self, cx: &mut Context<Self>) {
        cx.emit(ArchiveVmEvent::RequestNewFile);
    }

    pub fn request_checksum(&mut self, cx: &mut Context<Self>, algorithm: EventChecksumAlgorithm) {
        if !matches!(self.status, ViewStatus::Ready) || self.selection.is_empty() {
            return;
        }
        let indices: Vec<u32> = self.selection.iter().copied().collect();
        let count = indices.len() as u64;
        cx.emit(ArchiveVmEvent::RequestChecksum { algorithm });
        let repo = self.repo.clone();
        let handle = self.archive.clone().unwrap();

        let (tx, rx) = progress_channel();
        cx.update_global::<ProgressState, _>(|state, _cx| {
            state.is_active = true;
            state.is_complete = false;
            state.is_paused = false;
            state.receiver = Some(Arc::new(Mutex::new(rx)));
            state.message = format!("Calculating {}...", match algorithm {
                EventChecksumAlgorithm::Crc32 => "CRC32",
                EventChecksumAlgorithm::Md5 => "MD5",
                EventChecksumAlgorithm::Sha1 => "SHA1",
                EventChecksumAlgorithm::Sha256 => "SHA256",
            });
            state.current = 0;
            state.total = count;
            state.error = None;
        });

        cx.background_spawn(async move {
            let uc = CalculateChecksumUseCase::new(repo);
            let algos = match algorithm {
                EventChecksumAlgorithm::Crc32 => vec![ChecksumAlgorithm::Crc32],
                EventChecksumAlgorithm::Md5 => vec![ChecksumAlgorithm::Md5],
                EventChecksumAlgorithm::Sha1 => vec![ChecksumAlgorithm::Sha1],
                EventChecksumAlgorithm::Sha256 => vec![ChecksumAlgorithm::Sha256],
            };
            let _ = uc.execute(&handle, &indices, &algos);
        }).detach();

        cx.spawn(async move |this, cx| {
            loop {
                let done = cx.update_global::<ProgressState, _>(|state, _| {
                    let _ = state.poll();
                    state.is_complete
                });
                if done {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(80)).await;
            }
            let _ = this.update(cx, |this, cx| {
                this.refresh(cx);
                cx.emit(ArchiveVmEvent::RefreshListing);
            });
        }).detach();
    }
}
