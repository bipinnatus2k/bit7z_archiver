mod view;

use bit7z_infra_progress::progress_channel;
use bit7z_app_test::TestEntriesUseCase;
use bit7z_domain::archive::*;
use bit7z_domain::repository::*;
use crossbeam::channel::{unbounded, Receiver, Sender};
use gpui::*;
use std::sync::Arc;
use view::{TestDialogView, TestViewIntent};

fn count_files_recursive(
    repo: &Arc<dyn ArchiveRepository>,
    archive: &ArchiveHandle,
    dir_path: &str,
) -> usize {
    let path = if dir_path.ends_with('/') {
        dir_path.to_string()
    } else {
        format!("{}/", dir_path)
    };
    let mut count = 0;
    if let Ok(children) = repo.list_directory(archive, &path) {
        for child in &children {
            if child.is_directory {
                count += count_files_recursive(repo, archive, &child.path);
            } else {
                count += 1;
            }
        }
    }
    count
}

fn count_expanded(
    repo: &Arc<dyn ArchiveRepository>,
    archive: &ArchiveHandle,
    indices: &[u32],
) -> usize {
    let mut count = 0;
    for &idx in indices {
        if let Ok(page) = repo.list_page(archive, idx as usize, 1) {
            if let Some(entry) = page.items.first() {
                if entry.is_directory {
                    count += count_files_recursive(repo, archive, &entry.path);
                    continue;
                }
            }
        }
        count += 1;
    }
    count
}

pub enum TestResultEvent {
    Completed(TestResult),
    Canceled,
}

pub struct TestDialog {
    view: Entity<TestDialogView>,
    handle: Option<ArchiveHandle>,
    path: Option<std::path::PathBuf>,
    password: Option<Password>,
    repo: Arc<dyn ArchiveRepository>,
    result_tx: Option<Sender<TestResultEvent>>,
    total_items: u64,
    indices: Option<Vec<u32>>,
}

impl TestDialog {
    pub fn open_with_entries(
        cx: &mut AsyncApp,
        handle: ArchiveHandle,
        indices: Option<Vec<u32>>,
        repo: Arc<dyn ArchiveRepository>,
    ) -> Receiver<TestResultEvent> {
        let (tx, rx) = unbounded::<TestResultEvent>();
        let total = indices.as_ref().map_or_else(
            || repo.get_properties(&handle).map(|p| p.files_count as usize).unwrap_or(0),
            |v| count_expanded(&repo, &handle, v),
        );
        let opts = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::new(point(px(200.), px(200.)), size(px(560.), px(480.))))),
            window_background: WindowBackgroundAppearance::Opaque,
            window_decorations: Some(WindowDecorations::Client),
            ..Default::default()
        };
        cx.spawn(async move |cx| {
            let _ = cx.open_window(opts, move |window, cx| {
                let view = cx.new(|_cx| TestDialogView::new(total));
                let view_handle = view.clone();
                let c = cx.new(|_cx| Self { view, handle: Some(handle), path: None, password: None, repo, result_tx: Some(tx), total_items: total as u64, indices });
                let c_sub = c.clone();
                cx.subscribe::<TestDialogView, TestViewIntent>(&view_handle, move |_emitter: Entity<TestDialogView>, intent: &TestViewIntent, cx: &mut App| {
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
    ) -> Receiver<TestResultEvent> {
        let count = repo.open(&path, password.as_ref())
            .and_then(|h| { let n = repo.get_properties(&h).map(|p| p.files_count).unwrap_or(0); repo.close(&h); Ok(n) })
            .unwrap_or(0) as usize;
        let (tx, rx) = unbounded::<TestResultEvent>();
        let opts = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::new(point(px(200.), px(200.)), size(px(560.), px(480.))))),
            window_background: WindowBackgroundAppearance::Opaque,
            window_decorations: Some(WindowDecorations::Client),
            ..Default::default()
        };
        cx.spawn(async move |cx| {
            let _ = cx.open_window(opts, move |window, cx| {
                let view = cx.new(|_cx| TestDialogView::new(count));
                let view_handle = view.clone();
                let c = cx.new(|_cx| Self { view, handle: None, path: Some(path.to_path_buf()), password, repo, result_tx: Some(tx), total_items: count as u64, indices: None });
                let c_sub = c.clone();
                cx.subscribe::<TestDialogView, TestViewIntent>(&view_handle, move |_emitter: Entity<TestDialogView>, intent: &TestViewIntent, cx: &mut App| {
                    c_sub.update(cx, |c, cx| c.handle_intent(intent.clone(), cx));
                }).detach();
                cx.new(|cx| gpui_component::Root::new(c, window, cx))
            });
        }).detach();
        rx
    }

    fn handle_intent(&mut self, intent: TestViewIntent, cx: &mut Context<Self>) {
        match intent {
            TestViewIntent::Start => {
                self.view.update(cx, |v, cx| {
                    v.set_processing(0, self.total_items, "Starting...");
                    cx.notify();
                });

                let view = self.view.clone();
                let repo = self.repo.clone();
                let _result_tx = self.result_tx.take();
                let (progress_tx, progress_rx) = progress_channel();
                let (result_tx2, result_rx2) = unbounded::<Result<TestResult, ArchiveError>>();
                let handle = self.handle.clone();
                let path = self.path.clone();
                let password = self.password.clone();
                let indices = self.indices.clone();

                cx.background_spawn(async move {
                    let dialog_owns_handle = handle.is_none();
                    let h = match handle {
                        Some(h) => h,
                        None => path.and_then(|p| repo.open(&p, password.as_ref()).ok()).unwrap_or(ArchiveHandle::new_reader()),
                    };
                    let uc = TestEntriesUseCase::new(repo.clone());
                    let result = uc.execute(&h, indices.as_deref(), Some(progress_tx));
                    if dialog_owns_handle {
                        repo.close(&h);
                    }
                    let _ = result_tx2.send(result);
                }).detach();

                cx.spawn(async move |_, cx| {
                    loop {
                        while let Ok(update) = progress_rx.try_recv() {
                            view.update(cx, |v, cx| {
                                v.set_processing(update.items_done, update.items_total, &update.current_file.unwrap_or_default());
                                cx.notify();
                            });
                        }
                        if let Ok(result) = result_rx2.try_recv() {
                            match result {
                                Ok(tr) => {
                                    view.update(cx, |v, cx| {
                                        v.set_complete(tr.passed, tr.failed);
                                        cx.notify();
                                    });
                                }
                                Err(e) => {
                                    view.update(cx, |v, cx| {
                                        v.set_error(&e.to_string());
                                        cx.notify();
                                    });
                                }
                            }
                            break;
                        }
                        cx.background_spawn(std::future::ready(())).await;
                    }
                }).detach();
            }
            TestViewIntent::Cancel | TestViewIntent::Close => {
                if let Some(tx) = self.result_tx.take() {
                    let _ = tx.send(TestResultEvent::Canceled);
                }
            }
        }
    }
}

impl Render for TestDialog {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        self.view.clone()
    }
}
