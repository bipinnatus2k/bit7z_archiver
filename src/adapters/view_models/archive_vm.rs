use crate::domain::archive::*;
use crate::domain::preferences::Preferences;
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
    pub status: ViewStatus,
    pub current_offset: usize,
    pub total_entries: Option<usize>,
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
            status: ViewStatus::Empty,
            current_offset: 0,
            total_entries: None,
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
        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let result = repo.open(&path_buf, password.as_deref());
            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok(handle) => {
                        this.archive = Some(handle);
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

    pub fn select(&mut self, index: u32, modifiers: &Modifiers, cx: &mut Context<Self>) {
        if modifiers.shift {
            // Range select
        } else if modifiers.control {
            if self.selection.contains(&index) {
                self.selection.remove(&index);
            } else {
                self.selection.insert(index);
            }
        } else {
            self.selection.clear();
            self.selection.insert(index);
        }
        cx.notify();
    }

    pub fn close(&mut self, cx: &mut Context<Self>) {
        if let Some(archive) = self.archive.take() {
            self.repo.close(archive);
        }
        self.entries.clear();
        self.selection.clear();
        self.status = ViewStatus::Empty;
        cx.notify();
    }
}


