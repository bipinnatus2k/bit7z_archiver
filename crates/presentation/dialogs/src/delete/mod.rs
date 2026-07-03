mod view;

use bit7z_app_archive::delete::DeleteEntriesUseCase;
use bit7z_infra_progress::progress_channel;
use bit7z_domain::archive::*;
use bit7z_domain::repository::*;
use crossbeam::channel::{unbounded, Receiver, Sender};
use gpui::*;
use std::sync::Arc;
use view::{DeleteDialogView, DeleteViewIntent};

pub enum DeleteResult {
    Success,
    Error(String),
    Canceled,
}

pub struct DeleteDialog {
    view: Entity<DeleteDialogView>,
    indices: Vec<u32>,
    handle: ArchiveHandle,
    repo: Arc<dyn ArchiveRepository>,
    result_tx: Option<Sender<DeleteResult>>,
}

impl DeleteDialog {
    pub fn open(
        cx: &mut AsyncApp,
        indices: Vec<u32>,
        handle: ArchiveHandle,
        repo: Arc<dyn ArchiveRepository>,
    ) -> Receiver<DeleteResult> {
        let (tx, rx) = unbounded::<DeleteResult>();
        let count = indices.len() as u64;

        cx.spawn(async move |cx| {
            let _ = cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                        point(px(200.), px(200.)),
                        size(px(420.), px(220.)),
                    ))),
                    window_background: WindowBackgroundAppearance::Opaque,
                    window_decorations: Some(WindowDecorations::Client),
                    ..Default::default()
                },
                move |window, cx| {
                    let view = cx.new(|_cx| DeleteDialogView::new(count));
                    let view_handle = view.clone();
                    let container = cx.new(|_cx| Self { view, indices, handle, repo, result_tx: Some(tx) });
                    cx.subscribe::<DeleteDialogView, DeleteViewIntent>(&view_handle, {
                        let container = container.clone();
                        move |_, intent, cx| {
                            container.update(cx, |c, cx| c.handle_intent(intent.clone(), cx));
                        }
                    }).detach();
                    cx.new(|cx| gpui_component::Root::new(container, window, cx))
                },
            );
        }).detach();
        rx
    }

    fn handle_intent(&mut self, intent: DeleteViewIntent, cx: &mut Context<Self>) {
        match intent {
            DeleteViewIntent::Confirm => {
                self.view.update(cx, |v, _| v.set_processing(0, self.indices.len() as u64, "Deleting entries..."));

                let indices = self.indices.clone();
                let mut handle = self.handle.clone();
                let repo = self.repo.clone();
                let (tx, rx) = progress_channel();
                let view = self.view.clone();
                let result_tx = self.result_tx.take();

                let notifier: Option<std::sync::Arc<dyn bit7z_domain::repository::ProgressNotifier>> =
                    Some(std::sync::Arc::new(bit7z_infra_progress::CrossbeamNotifier(tx)));
                cx.background_spawn(async move {
                    let uc = DeleteEntriesUseCase::new(repo);
                    let _ = uc.execute(&mut handle, &indices, notifier);
                }).detach();

                cx.spawn(async move |_, cx| {
                    loop {
                        if let Ok(update) = rx.try_recv() {
                            view.update(cx, |v, _| v.set_processing(update.items_done, update.items_total, &update.current_file.unwrap_or_default()));
                            if update.items_done >= update.items_total || update.error.is_some() {
                                break;
                            }
                        }
                        cx.background_spawn(std::future::ready(())).await;
                    }
                    view.update(cx, |v, _| v.set_complete());
                    if let Some(tx) = result_tx {
                        let _ = tx.send(DeleteResult::Success);
                    }
                }).detach();
            }
            DeleteViewIntent::Cancel | DeleteViewIntent::Close => {
                if let Some(tx) = self.result_tx.take() {
                    let _ = tx.send(DeleteResult::Canceled);
                }
            }
        }
    }
}

impl Render for DeleteDialog {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        self.view.clone()
    }
}
