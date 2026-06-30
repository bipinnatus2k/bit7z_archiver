mod view;

use crate::application::checksum::{CalculateChecksumUseCase, ChecksumAlgorithm};
use crate::application::progress::{progress_channel, CrossbeamNotifier};
use crate::domain::archive::*;
use crate::domain::repository::*;
use crossbeam::channel::{unbounded, Receiver, Sender};
use gpui::*;
use std::sync::Arc;
use view::{ChecksumDialogView, ChecksumViewIntent};

pub enum ChecksumResultEvent {
    Completed,
    Canceled,
}

#[allow(dead_code)]
pub struct ChecksumDialog {
    view: Entity<ChecksumDialogView>,
    handle: Option<ArchiveHandle>,
    path: Option<std::path::PathBuf>,
    password: Option<Password>,
    indices: Vec<u32>,
    repo: Arc<dyn ArchiveRepository>,
    result_tx: Option<Sender<ChecksumResultEvent>>,
}

impl ChecksumDialog {
    pub fn open_with_entries(
        cx: &mut AsyncApp,
        handle: ArchiveHandle,
        indices: Vec<u32>,
        repo: Arc<dyn ArchiveRepository>,
    ) -> Receiver<ChecksumResultEvent> {
        let (tx, rx) = unbounded::<ChecksumResultEvent>();
        let count = indices.len();
        let opts = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::new(point(px(200.), px(200.)), size(px(560.), px(480.))))),
            window_background: WindowBackgroundAppearance::Opaque,
            window_decorations: Some(WindowDecorations::Client),
            ..Default::default()
        };
        cx.spawn(async move |cx| {
            let _ = cx.open_window(opts, move |window, cx| {
                let view = cx.new(|_cx| ChecksumDialogView::new(count));
                let view_handle = view.clone();
                let c = cx.new(|_cx| Self { view, handle: Some(handle), path: None, password: None, indices, repo, result_tx: Some(tx) });
                let c_sub = c.clone();
                cx.subscribe::<ChecksumDialogView, ChecksumViewIntent>(&view_handle, move |_emitter: Entity<ChecksumDialogView>, intent: &ChecksumViewIntent, cx: &mut App| {
                    c_sub.update(cx, |c, cx| c.handle_intent(intent.clone(), cx));
                }).detach();
                cx.new(|cx| gpui_component::Root::new(c, window, cx))
            });
        }).detach();
        rx
    }

    pub fn open_with_path(
        cx: &mut AsyncApp,
        path: std::path::PathBuf,
        password: Option<Password>,
        repo: Arc<dyn ArchiveRepository>,
    ) -> Receiver<ChecksumResultEvent> {
        let (handle, count) = repo.open(&path, password.as_ref())
            .and_then(|h| { let n = repo.get_properties(&h).map(|p| p.items_count); Ok((h, n.unwrap_or(0) as usize)) })
            .unwrap_or_else(|_| (ArchiveHandle::new_reader(), 0));
        let all_indices: Vec<u32> = (0..count as u32).collect();
        let (tx, rx) = unbounded::<ChecksumResultEvent>();
        let opts = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::new(point(px(200.), px(200.)), size(px(560.), px(480.))))),
            window_background: WindowBackgroundAppearance::Opaque,
            window_decorations: Some(WindowDecorations::Client),
            ..Default::default()
        };
        cx.spawn(async move |cx| {
            let _ = cx.open_window(opts, move |window, cx| {
                let view = cx.new(|_cx| ChecksumDialogView::new(count));
                let view_handle = view.clone();
                let c = cx.new(|_cx| Self { view, handle: Some(handle), path: None, password, indices: all_indices, repo, result_tx: Some(tx) });
                let c_sub = c.clone();
                cx.subscribe::<ChecksumDialogView, ChecksumViewIntent>(&view_handle, move |_emitter: Entity<ChecksumDialogView>, intent: &ChecksumViewIntent, cx: &mut App| {
                    c_sub.update(cx, |c, cx| c.handle_intent(intent.clone(), cx));
                }).detach();
                cx.new(|cx| gpui_component::Root::new(c, window, cx))
            });
        }).detach();
        rx
    }

    fn handle_intent(&mut self, intent: ChecksumViewIntent, cx: &mut Context<Self>) {
        match intent {
            ChecksumViewIntent::Start => {
                self.view.update(cx, |v, _| v.set_processing(0, self.indices.len() as u64, "Starting...", vec![]));

                let view = self.view.clone();
                let repo = self.repo.clone();
                let _result_tx = self.result_tx.take();
                let (progress_tx, progress_rx) = progress_channel();
                let (result_tx2, result_rx2) = unbounded::<Result<(), ArchiveError>>();
                let handle = self.handle.clone().unwrap();
                let indices = self.indices.clone();

                let algos = vec![
                    ChecksumAlgorithm::Crc32,
                    ChecksumAlgorithm::Md5,
                    ChecksumAlgorithm::Sha1,
                    ChecksumAlgorithm::Sha256,
                ];

                cx.background_spawn(async move {
                    let uc = CalculateChecksumUseCase::new(repo.clone());
                    let _ = uc.execute(&handle, &indices, &algos);
                    let _ = result_tx2.send(Ok(()));
                }).detach();

                cx.spawn(async move |_, cx| {
                    let results: Vec<(String, u64, String)> = Vec::new();
                    loop {
                        while let Ok(update) = progress_rx.try_recv() {
                            view.update(cx, |v, _| v.set_processing(update.items_done, update.items_total, &update.current_file.unwrap_or_default(), results.clone()));

                        }
                        if let Ok(_) = result_rx2.try_recv() {
                            view.update(cx, |v, _| v.set_complete(results));

                            break;
                        }
                        cx.background_spawn(std::future::ready(())).await;
                    }
                }).detach();
            }
            ChecksumViewIntent::Cancel | ChecksumViewIntent::Close => {
                if let Some(tx) = self.result_tx.take() {
                    let _ = tx.send(ChecksumResultEvent::Canceled);
                }
            }
        }
    }
}

impl Render for ChecksumDialog {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        self.view.clone()
    }
}


