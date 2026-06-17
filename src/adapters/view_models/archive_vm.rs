use crate::application::events::ArchiveVmEvent;
use crate::application::open::OpenArchiveUseCase;
use crate::domain::archive::*;
use crate::domain::preferences::Preferences;
use crate::domain::repository::*;
use gpui::{EventEmitter, *};
use std::collections::{HashSet, VecDeque};
use std::sync::Arc;


/// A single item at the current navigation level (file or directory).
/// Directories are aggregated from multiple underlying entries.
#[derive(Clone, Debug)]
pub struct LevelEntry {
    pub display_name: String,
    pub is_directory: bool,
    pub original_idx: Option<usize>,
    pub size: u64,
    pub compressed_size: u64,
    pub modified: Option<chrono::DateTime<chrono::Utc>>,
}


impl EventEmitter<ArchiveVmEvent> for ArchiveViewModel {}

const PAGE_SIZE: usize = 200;
const CACHE_PAGES: usize = 3;

pub struct ArchiveViewModel {
    repo: Arc<dyn ArchiveRepository>,
    pub archive: Option<ArchiveHandle>,
    pub properties: Option<ArchiveProperties>,
    pub entries: VecDeque<ArchiveEntry>,
    pub selection: HashSet<u32>,
    pub filter_text: String,
    pub sort_column: u32,
    pub sort_ascending: bool,
    pub status: ViewStatus,
    pub current_offset: usize,
    pub total_entries: Option<usize>,
    /// Pre-computed folder names for the sidebar tree (avoids iterating all entries on render).
    pub cached_folders: Vec<String>,
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
            entries: VecDeque::new(),
            selection: HashSet::new(),
            filter_text: String::new(),
            sort_column: 0,
            sort_ascending: true,
            status: ViewStatus::Empty,
            current_offset: 0,
            total_entries: None,
            cached_folders: Vec::new(),
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
        // Emit immediately to trigger root re-render.
        cx.emit(ArchiveVmEvent::SelectionChanged(None));

        let repo = self.repo.clone();
        let path_buf = path.to_path_buf();
        let path_string = path.to_string_lossy().to_string();

        // Use OpenArchiveUseCase for combined open + properties + first page.
        let use_case = OpenArchiveUseCase::new(repo);
        let bg_task = cx.background_spawn(async move {
            use_case.execute(&path_buf, password.as_deref())
        });

        cx.spawn(async move |this, cx| {
            let result = bg_task.await;
            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok(output) => {
                        this.archive = Some(output.handle);
                        this.properties = Some(output.properties);
                        // OpenArchiveUseCase already loaded the first page.
                        this.entries = VecDeque::from(output.first_page.items);
                        this.current_offset = 0;
                        this.total_entries = output.first_page.total;
                        this.cached_folders = this.entries.iter()
                            .filter(|e| e.is_directory)
                            .map(|e| e.name.clone())
                            .collect();
                        let mut prefs = cx.global::<Preferences>().clone();
                        prefs.archive.add_recent(path_string);
                        cx.set_global(prefs);
                        this.current_path = String::new();
                        this.path_history.clear();
                        this.re_filter();
                        this.status = ViewStatus::Ready;
                        cx.emit(ArchiveVmEvent::SelectionChanged(None));
                    }
                    Err(e) => {
                        this.status = ViewStatus::Error(e.to_string());
                    }
                }
                cx.notify();
            });
        }).detach();
    }

    pub fn load_page(&mut self, offset: usize, cx: &mut Context<Self>) {
        if let Some(ref archive) = self.archive {
            let repo = self.repo.clone();
            let handle = archive.clone();

            // Run blocking FFI on a background thread.
            let bg_task = cx.background_spawn(async move {
                repo.list_page(&handle, offset, PAGE_SIZE)
            });

            cx.spawn(async move |this, cx| {
                let page = bg_task.await;
                let _ = this.update(cx, |this, cx| {
                    match page {
                        Ok(p) => {
                            this.entries.extend(p.items);
                            this.current_offset = offset;
                            this.total_entries = p.total;
                            while this.entries.len() > PAGE_SIZE * CACHE_PAGES {
                                this.entries.pop_front();
                            }
                            // Rebuild folder cache (avoids iterating all entries on render).
                            this.cached_folders = this.entries.iter()
                                .filter(|e| e.is_directory)
                                .map(|e| e.name.clone())
                                .collect();
                            this.re_filter();
                            this.status = ViewStatus::Ready;
                        }
                        Err(e) => {
                            this.status = ViewStatus::Error(e.to_string());
                        }
                    }
                    cx.notify();
                });
            }).detach();
        }
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

    pub fn sort_by(&mut self, column: u32, cx: &mut Context<Self>) {
        if self.sort_column == column {
            self.sort_ascending = !self.sort_ascending;
        } else {
            self.sort_column = column;
            self.sort_ascending = true;
        }
        let asc = self.sort_ascending;
        let slice = self.entries.make_contiguous();
        slice.sort_by(|a, b| {
            let cmp = match column {
                1 => a.size.cmp(&b.size),
                2 => a.compressed_size.cmp(&b.compressed_size),
                3 => ((a.compression_ratio() * 100.0) as u64)
                     .cmp(&((b.compression_ratio() * 100.0) as u64)),
                _ => a.name.cmp(&b.name),
            };
            if asc { cmp } else { cmp.reverse() }
        });
        self.re_filter();
        cx.notify();
    }

    pub fn set_filter(&mut self, text: &str, cx: &mut Context<Self>) {
        self.filter_text = text.to_string();
        self.selection.clear();
        self.selection_anchor = None;
        self.re_filter();
        cx.notify();
    }

    /// Compute entries visible at the current navigation level.
    fn compute_level_entries(&self) -> Vec<LevelEntry> {
        let mut items: Vec<LevelEntry> = Vec::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        let filter_lower = self.filter_text.to_lowercase();

        for (idx, entry) in self.entries.iter().enumerate() {
            if !self.filter_text.is_empty()
                && !entry.path.to_lowercase().contains(&filter_lower)
            {
                continue;
            }

            let rel = if self.current_path.is_empty() {
                &entry.path
            } else {
                if !entry.path.starts_with(&self.current_path) {
                    continue;
                }
                &entry.path[self.current_path.len()..]
            };
            if rel.is_empty() { continue; }

            let slash = rel.find('/');
            let (name, has_deeper) = match slash {
                Some(p) => (&rel[..p], true),
                None => (rel, false),
            };
            if name.is_empty() || !seen.insert(name.to_string()) { continue; }

            if has_deeper || entry.is_directory {
                items.push(LevelEntry {
                    display_name: name.to_string(),
                    is_directory: true,
                    original_idx: None,
                    size: 0,
                    compressed_size: 0,
                    modified: None,
                });
            } else {
                items.push(LevelEntry {
                    display_name: name.to_string(),
                    is_directory: false,
                    original_idx: Some(idx),
                    size: entry.size,
                    compressed_size: entry.compressed_size,
                    modified: entry.modified,
                });
            }
        }

        items.sort_by(|a, b| {
            if a.is_directory != b.is_directory {
                return if self.sort_ascending { b.is_directory.cmp(&a.is_directory) } else { a.is_directory.cmp(&b.is_directory) };
            }
            let c = a.display_name.to_lowercase().cmp(&b.display_name.to_lowercase());
            if self.sort_ascending { c } else { c.reverse() }
        });
        items
    }

    fn re_filter(&mut self) {
        self.level_entries = self.compute_level_entries();
    }

    /// Apply sort state from DataTable and re-sort entries.
    pub fn apply_sort(&mut self, column: u32, ascending: bool) {
        self.sort_column = column;
        self.sort_ascending = ascending;
        let asc = ascending;
        let slice = self.entries.make_contiguous();
        slice.sort_by(|a, b| {
            let cmp = match column {
                1 => a.size.cmp(&b.size),
                2 => a.compressed_size.cmp(&b.compressed_size),
                3 => ((a.compression_ratio() * 100.0) as u64)
                     .cmp(&((b.compression_ratio() * 100.0) as u64)),
                _ => a.name.cmp(&b.name),
            };
            if asc { cmp } else { cmp.reverse() }
        });
        self.re_filter();
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
        self.re_filter();
        cx.emit(ArchiveVmEvent::SelectionChanged(None));
    }

    pub fn navigate_up(&mut self, cx: &mut Context<Self>) {
        if let Some(prev) = self.path_history.pop() {
            self.current_path = prev;
            self.selection.clear();
            self.selection_anchor = None;
            self.re_filter();
            cx.emit(ArchiveVmEvent::SelectionChanged(None));
        }
    }

    pub fn navigate_root(&mut self, cx: &mut Context<Self>) {
        self.path_history.clear();
        self.current_path = String::new();
        self.selection.clear();
        self.selection_anchor = None;
        self.re_filter();
        cx.emit(ArchiveVmEvent::SelectionChanged(None));
    }

    pub fn handle_level_click(&mut self, level_idx: usize, modifiers: &Modifiers, cx: &mut Context<Self>) {
        if let Some(entry) = self.level_entries.get(level_idx) {
            if entry.is_directory {
                let name = entry.display_name.clone();
                self.navigate_into(&name, cx);
                cx.notify();
            } else if let Some(orig_idx) = entry.original_idx {
                self.update_selection(orig_idx as u32, modifiers);
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

    /// Return entries at the current navigation level.
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

    /// Build tree_entries by parsing entry.path into directory hierarchy.
    pub fn close(&mut self, cx: &mut Context<Self>) {
        if let Some(archive) = self.archive.take() {
            self.repo.close(archive);
        }
        self.entries.clear();
        self.selection.clear();
        self.selection_anchor = None;
        self.filter_text.clear();
        self.cached_folders.clear();
        self.level_entries.clear();
        self.current_path.clear();
        self.path_history.clear();
        self.status = ViewStatus::Empty;
        cx.notify();
    }
}
