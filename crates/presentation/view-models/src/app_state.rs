use bit7z_domain::archive::*;
use bit7z_domain::repository::ArchiveProperties;
use gpui_signals::prelude::*;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq)]
pub enum ViewStatus {
    Empty,
    Loading,
    Ready,
    Error(String),
}

#[derive(Clone, Debug)]
pub struct LevelEntry {
    pub display_name: String,
    pub is_directory: bool,
    pub original_index: u32,
    pub size: u64,
    pub compressed_size: u64,
    pub modified: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone)]
pub struct OpHandle {
    pub can_cancel: bool,
}

#[derive(Debug, Clone)]
pub struct ProgressSnapshot {
    pub current: u64,
    pub total: u64,
    pub message: String,
}

#[derive(Clone)]
pub struct AppState {
    pub handle: Signal<Option<ArchiveHandle>>,
    pub properties: Signal<Option<ArchiveProperties>>,
    pub directory_cache: Signal<HashMap<String, Vec<ArchiveEntry>>>,
    pub selection: Signal<HashSet<u32>>,
    pub filter_text: Signal<String>,
    pub sort_column: Signal<u32>,
    pub sort_ascending: Signal<bool>,
    pub status: Signal<ViewStatus>,
    pub level_entries: Signal<Vec<LevelEntry>>,
    pub current_path: Signal<String>,
    pub path_history: Signal<Vec<String>>,
    pub selection_anchor: Signal<Option<u32>>,
    pub archive_password: Signal<Option<Password>>,
    pub show_preview: Signal<bool>,
    pub operation: Signal<Option<OpHandle>>,
    pub progress: Signal<Option<ProgressSnapshot>>,
    pub sidebar_collapsed: Signal<bool>,
}

impl AppState {
    pub fn new<T: 'static>(cx: &mut gpui::Context<T>) -> Self {
        Self {
            handle: cx.create_signal(None),
            properties: cx.create_signal(None),
            directory_cache: cx.create_signal(HashMap::new()),
            selection: cx.create_signal(HashSet::new()),
            filter_text: cx.create_signal(String::new()),
            sort_column: cx.create_signal(0),
            sort_ascending: cx.create_signal(true),
            status: cx.create_signal(ViewStatus::Empty),
            level_entries: cx.create_signal(Vec::new()),
            current_path: cx.create_signal(String::new()),
            path_history: cx.create_signal(Vec::new()),
            selection_anchor: cx.create_signal(None),
            archive_password: cx.create_signal(None),
            show_preview: cx.create_signal(false),
            operation: cx.create_signal(None),
            progress: cx.create_signal(None),
            sidebar_collapsed: cx.create_signal(false),
        }
    }

    pub fn update_selection(&self, row: usize) {
        let entries = self.level_entries.get();
        let index = entries.get(row).map(|e| e.original_index).unwrap_or(0);
        self.selection.update(|s| {
            s.clear();
            s.insert(index);
        });
        self.selection_anchor.set(Some(index));
    }

    pub fn select_all(&self) {
        let entries = self.level_entries.get();
        self.selection.update(|s| {
            for entry in &entries {
                s.insert(entry.original_index);
            }
        });
        self.selection_anchor.set(None);
    }

    pub fn clear_selection(&self) {
        self.selection.update(|s| s.clear());
        self.selection_anchor.set(None);
    }

    pub fn invert_selection(&self) {
        let entries = self.level_entries.get();
        let all: HashSet<u32> = entries.iter().map(|e| e.original_index).collect();
        self.selection.update(|s| {
            for idx in &all {
                if s.contains(idx) {
                    s.remove(idx);
                } else {
                    s.insert(*idx);
                }
            }
        });
        self.selection_anchor.set(None);
    }

    pub fn sort_by_column(&self, column: u32) {
        if self.sort_column.get() == column {
            self.sort_ascending.update(|a| *a = !*a);
        } else {
            self.sort_column.set(column);
            self.sort_ascending.set(true);
        }
        self.reapply_filter_and_sort();
    }

    pub fn apply_sort(&self, column: u32, ascending: bool) {
        self.sort_column.set(column);
        self.sort_ascending.set(ascending);
        self.reapply_filter_and_sort();
    }

    pub fn set_filter(&self, text: &str) {
        self.filter_text.set(text.to_string());
        self.clear_selection();
        self.reapply_filter_and_sort();
    }

    pub fn navigate_into(&self, dir_name: &str) {
        let current = self.current_path.get();
        self.path_history.update(|h| h.push(current.clone()));
        let new_path = if current.is_empty() {
            format!("{}/", dir_name)
        } else {
            format!("{}{}/", current, dir_name)
        };
        self.current_path.set(new_path);
        self.clear_selection();
    }

    pub fn navigate_up(&self) {
        let prev = self.path_history.update_with(|h| h.pop()).flatten();
        self.current_path.set(prev.unwrap_or_default());
        self.clear_selection();
    }

    pub fn navigate_root(&self) {
        self.path_history.update(|h| h.clear());
        self.current_path.set(String::new());
        self.clear_selection();
    }

    pub fn reapply_filter_and_sort(&self) {
        let cache = self.directory_cache.get();
        let path = self.current_path.get();
        let snapshot = cache.get(&path).cloned().unwrap_or_default();
        self.apply_filter_and_sort(&snapshot);
    }

    fn apply_filter_and_sort(&self, entries: &[ArchiveEntry]) {
        let filter_lower = self.filter_text.get().to_lowercase();
        let sort_column = self.sort_column.get();
        let sort_ascending = self.sort_ascending.get();
        let mut items: Vec<LevelEntry> = Vec::new();

        for entry in entries {
            if !filter_lower.is_empty() && !entry.path().to_lowercase().contains(&filter_lower) {
                continue;
            }
            items.push(LevelEntry {
                display_name: entry.name().to_string(),
                is_directory: entry.is_directory(),
                original_index: entry.original_index(),
                size: entry.size(),
                compressed_size: entry.compressed_size(),
                modified: entry.mtime().and_then(|ts| chrono::DateTime::from_timestamp(ts, 0)),
            });
        }

        items.sort_by(|a, b| {
            if a.is_directory != b.is_directory {
                return if sort_ascending { b.is_directory.cmp(&a.is_directory) } else { a.is_directory.cmp(&b.is_directory) };
            }
            let c = match sort_column {
                1 => a.size.cmp(&b.size),
                2 => a.compressed_size.cmp(&b.compressed_size),
                3 => Self::ratio_key(a).cmp(&Self::ratio_key(b)),
                _ => a.display_name.to_lowercase().cmp(&b.display_name.to_lowercase()),
            };
            if sort_ascending { c } else { c.reverse() }
        });
        self.level_entries.set(items);
    }

    fn ratio_key(e: &LevelEntry) -> u64 {
        if e.size == 0 { 0 }
        else { (((1.0 - e.compressed_size as f64 / e.size as f64) * 10000.0).max(0.0)) as u64 }
    }

    pub fn displayed_entries(&self) -> Vec<LevelEntry> {
        self.level_entries.get()
    }

    pub fn selected_entries(&self) -> Vec<ArchiveEntry> {
        let sel = self.selection.get();
        if sel.is_empty() { return vec![]; }
        let cache = self.directory_cache.get();
        cache.values()
            .flat_map(|entries| entries.iter())
            .filter(|e| sel.contains(&e.original_index()))
            .cloned()
            .collect()
    }

    pub fn first_selected_index(&self) -> Option<u32> {
        self.selection.get().iter().next().copied()
    }

    pub fn selected_indices(&self) -> Vec<u32> {
        self.selection.get().iter().copied().collect()
    }

    pub fn has_selection(&self) -> bool {
        !self.selection.get().is_empty()
    }

    pub fn selection_len(&self) -> usize {
        self.selection.get().len()
    }

    pub fn is_ready(&self) -> bool {
        self.status.get() == ViewStatus::Ready
    }

    pub fn is_open(&self) -> bool {
        self.handle.get().is_some()
    }

    pub fn current_subdirs(&self) -> Vec<String> {
        let cache = self.directory_cache.get();
        let path = self.current_path.get();
        cache.get(&path)
            .map(|entries| {
                let mut dirs: Vec<String> = entries.iter()
                    .filter(|e| e.is_directory())
                    .map(|e| e.name().to_string())
                    .collect();
                dirs.sort();
                dirs
            })
            .unwrap_or_default()
    }

    pub fn filtered_subdirs(&self) -> Vec<String> {
        let filter = self.filter_text.get();
        if filter.is_empty() {
            return self.current_subdirs();
        }
        let filter_lower = filter.to_lowercase();
        self.current_subdirs().into_iter()
            .filter(|name| name.to_lowercase().contains(&filter_lower))
            .collect()
    }

    pub fn status_text(&self) -> String {
        match self.status.get() {
            ViewStatus::Empty => String::new(),
            ViewStatus::Loading => "Loading...".into(),
            ViewStatus::Ready => {
                let total = self.level_entries.get().len();
                let selected = self.selection.get().len();
                if selected > 0 {
                    format!("{} items ({} selected)", total, selected)
                } else {
                    format!("{} items", total)
                }
            }
            ViewStatus::Error(msg) => format!("Error: {}", msg),
        }
    }
}
