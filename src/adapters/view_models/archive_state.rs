use crate::domain::archive::*;
use crate::domain::repository::ArchiveProperties;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, Default)]
pub struct KeyModifiers {
    pub shift: bool,
    pub control: bool,
    pub platform: bool,
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

#[derive(Debug, Clone, PartialEq)]
pub enum ViewStatus {
    Empty,
    Loading,
    Ready,
    Error(String),
}

pub struct ArchiveState {
    pub archive: Option<ArchiveHandle>,
    pub properties: Option<ArchiveProperties>,
    pub directory_cache: HashMap<String, Vec<ArchiveEntry>>,
    pub selection: HashSet<u32>,
    pub filter_text: String,
    pub sort_column: u32,
    pub sort_ascending: bool,
    pub status: ViewStatus,
    pub level_entries: Vec<LevelEntry>,
    pub current_path: String,
    pub path_history: Vec<String>,
    pub selection_anchor: Option<u32>,
    pub archive_password: Option<Password>,
}

impl ArchiveState {
    pub fn new() -> Self {
        Self {
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
            archive_password: None,
        }
    }

    pub fn update_selection(&mut self, row: usize) {
        let index = self.level_entries.get(row).map(|e| e.original_index).unwrap_or(0);
        self.selection.clear();
        self.selection.insert(index);
        self.selection_anchor = Some(index);
    }

    pub fn select_all(&mut self) {
        for entry in &self.level_entries {
            self.selection.insert(entry.original_index);
        }
        self.selection_anchor = None;
    }

    pub fn clear_selection(&mut self) {
        self.selection.clear();
        self.selection_anchor = None;
    }

    pub fn invert_selection(&mut self) {
        let all: HashSet<u32> = self.level_entries.iter().map(|e| e.original_index).collect();
        for idx in &all {
            if self.selection.contains(idx) {
                self.selection.remove(idx);
            } else {
                self.selection.insert(*idx);
            }
        }
        self.selection_anchor = None;
    }

    pub fn sort_by_column(&mut self, column: u32) {
        if self.sort_column == column {
            self.sort_ascending = !self.sort_ascending;
        } else {
            self.sort_column = column;
            self.sort_ascending = true;
        }
        self.reapply_filter_and_sort();
    }

    pub fn apply_sort(&mut self, column: u32, ascending: bool) {
        self.sort_column = column;
        self.sort_ascending = ascending;
        self.reapply_filter_and_sort();
    }

    pub fn set_filter(&mut self, text: &str) {
        self.filter_text = text.to_string();
        self.selection.clear();
        self.selection_anchor = None;
        self.reapply_filter_and_sort();
    }

    pub fn navigate_into(&mut self, dir_name: &str) {
        self.path_history.push(self.current_path.clone());
        self.current_path = if self.current_path.is_empty() {
            format!("{}/", dir_name)
        } else {
            format!("{}{}/", self.current_path, dir_name)
        };
        self.selection.clear();
        self.selection_anchor = None;
    }

    pub fn navigate_up(&mut self) {
        if let Some(prev) = self.path_history.pop() {
            self.current_path = prev;
        } else {
            self.current_path = String::new();
        }
        self.selection.clear();
        self.selection_anchor = None;
    }

    pub fn navigate_root(&mut self) {
        self.path_history.clear();
        self.current_path = String::new();
        self.selection.clear();
        self.selection_anchor = None;
    }

    pub(crate) fn reapply_filter_and_sort(&mut self) {
        let snapshot = self.directory_cache.get(&self.current_path).cloned().unwrap_or_default();
        self.apply_filter_and_sort(&snapshot);
    }

    fn apply_filter_and_sort(&mut self, entries: &[ArchiveEntry]) {
        let filter_lower = self.filter_text.to_lowercase();
        let mut items: Vec<LevelEntry> = Vec::new();

        for entry in entries {
            if !self.filter_text.is_empty() && !entry.path.to_lowercase().contains(&filter_lower) {
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
        else { (((1.0 - e.compressed_size as f64 / e.size as f64) * 10000.0).max(0.0)) as u64 }
    }

    pub fn displayed_entries(&self) -> &[LevelEntry] {
        &self.level_entries
    }

    pub fn selected_entries(&self) -> Vec<ArchiveEntry> {
        if self.selection.is_empty() { return vec![]; }
        self.directory_cache.values()
            .flat_map(|entries| entries.iter())
            .filter(|e| self.selection.contains(&e.original_index))
            .cloned()
            .collect()
    }

    pub fn first_selected_index(&self) -> Option<u32> {
        self.selection.iter().next().copied()
    }

    pub fn has_selection(&self) -> bool {
        !self.selection.is_empty()
    }

    pub fn is_ready(&self) -> bool {
        self.status == ViewStatus::Ready
    }

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

    pub fn filtered_subdirs(&self) -> Vec<String> {
        if self.filter_text.is_empty() {
            return self.current_subdirs();
        }
        let filter_lower = self.filter_text.to_lowercase();
        self.current_subdirs().into_iter()
            .filter(|name| name.to_lowercase().contains(&filter_lower))
            .collect()
    }

    pub fn status_text(&self) -> String {
        match &self.status {
            ViewStatus::Empty => String::new(),
            ViewStatus::Loading => "Loading...".into(),
            ViewStatus::Ready => {
                let total = self.level_entries.len();
                let selected = self.selection.len();
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
