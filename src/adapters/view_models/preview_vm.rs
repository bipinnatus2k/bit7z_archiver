use crate::application::preview::{PreviewData, PreviewEntryUseCase};
use crate::domain::archive::ArchiveHandle;
use crate::domain::preferences::Preferences;
use crate::domain::repository::{ArchiveRepository, RepoGlobal};
use gpui::*;
use std::sync::Arc;

pub struct PreviewViewModel {
    pub data: Option<PreviewData>,
    pub is_loading: bool,
    max_bytes: usize,
    repo: Arc<dyn ArchiveRepository>,
}

impl PreviewViewModel {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let repo = cx.global::<RepoGlobal>().0.clone();
        let prefs = cx.global::<Preferences>();
        Self {
            data: None,
            is_loading: false,
            max_bytes: prefs.preview.text_max_bytes,
            repo,
        }
    }

    pub fn load(&mut self, archive: ArchiveHandle, index: u32, cx: &mut Context<Self>) {
        self.is_loading = true;
        cx.notify();
        let max_bytes = self.max_bytes;
        let repo = self.repo.clone();
        let handle_for_bg = archive.clone();

        let bg_task = cx.background_spawn(async move {
            let uc = PreviewEntryUseCase::new(repo);
            uc.execute(&handle_for_bg, index, max_bytes)
        });

        cx.spawn(async move |this, cx| {
            let result = bg_task.await;
            let _ = this.update(cx, |this, cx| {
                this.is_loading = false;
                match result {
                    Ok(data) => this.data = Some(data),
                    Err(_) => this.data = None,
                }
                cx.notify();
            });
        }).detach();
    }

    pub fn clear(&mut self, cx: &mut Context<Self>) {
        self.data = None;
        self.is_loading = false;
        cx.notify();
    }
}
