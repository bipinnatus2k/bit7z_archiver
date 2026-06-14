use crate::domain::archive::*;
use crate::domain::repository::*;
use gpui::{EventEmitter, *};
use std::collections::{HashSet, VecDeque};
use std::sync::Arc;

impl EventEmitter<crate::adapters::views::root::ArchiveVmEvent> for ArchiveViewModel {}


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
    /// Anchor for shift-click range selection.
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

        let repo = self.repo.clone();
        let path_buf = path.to_path_buf();
        let path_string = path.to_string_lossy().to_string();
        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let result = repo.open(&path_buf, password.as_deref());
            let archive_properties = result.as_ref().ok().and_then(|handle| {
                repo.get_properties(handle).ok()
            });
            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok(handle) => {
                        this.archive = Some(handle);
                        this.properties = archive_properties;
                        // Add to recent files
                        let mut prefs = cx.global::<crate::domain::preferences::Preferences>().clone();
                        prefs.archive.add_recent(path_string.clone());
                        cx.set_global(prefs);
                        this.load_page(0, cx);
                    }
                    Err(e) => {
                        this.status = ViewStatus::Error(e.to_string());
                        cx.notify();
                    }
                }
            });
        }).detach();
    }

    pub fn load_page(&mut self, offset: usize, cx: &mut Context<Self>) {
        if let Some(ref archive) = self.archive {
            let repo = self.repo.clone();
            let handle_raw = archive.raw;
            cx.spawn(async move |this, cx: &mut AsyncApp| {
                let page = repo.list_page(
                    &ArchiveHandle { raw: handle_raw, is_writer: false },
                    offset, PAGE_SIZE,
                );
                let _ = this.update(cx, |this, cx| {
                    match page {
                        Ok(p) => {
                            this.entries.extend(p.items);
                            this.current_offset = offset;
                            this.total_entries = p.total;
                            // Trim cache
                            while this.entries.len() > PAGE_SIZE * CACHE_PAGES {
                                this.entries.pop_front();
                            }
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

    /// Selection logic without cx (testable directly).
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
            cx.emit(crate::adapters::views::root::ArchiveVmEvent::SelectionChanged(
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
        cx.notify();
    }

    pub fn set_filter(&mut self, text: &str, cx: &mut Context<Self>) {
        self.filter_text = text.to_string();
        self.selection.clear();
        self.selection_anchor = None;
        cx.notify();
    }

    /// Return entries matching the current filter with their original index.
    pub fn displayed_entries(&self) -> Vec<(usize, &ArchiveEntry)> {
        if self.filter_text.is_empty() {
            self.entries.iter().enumerate().collect()
        } else {
            let lower = self.filter_text.to_lowercase();
            self.entries.iter().enumerate()
                .filter(|(_, e)| e.name.to_lowercase().contains(&lower))
                .collect()
        }
    }

    pub fn request_extract(&mut self, cx: &mut Context<Self>) {
        if matches!(self.status, ViewStatus::Ready) && !self.selection.is_empty() {
            cx.emit(crate::adapters::views::root::ArchiveVmEvent::RequestShowExtract);
        }
    }

    pub fn request_create(&mut self, cx: &mut Context<Self>) {
        cx.emit(crate::adapters::views::root::ArchiveVmEvent::RequestShowCreate);
    }

    pub fn request_test(&mut self, cx: &mut Context<Self>) {
        if matches!(self.status, ViewStatus::Ready) {
            cx.emit(crate::adapters::views::root::ArchiveVmEvent::RequestTest);
        }
    }

    pub fn close(&mut self, cx: &mut Context<Self>) {
        if let Some(archive) = self.archive.take() {
            self.repo.close(archive);
        }
        self.entries.clear();
        self.selection.clear();
        self.selection_anchor = None;
        self.filter_text.clear();
        self.status = ViewStatus::Empty;
        cx.notify();
    }
}

