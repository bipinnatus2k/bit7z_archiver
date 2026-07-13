use bit7z_infra_progress::progress_channel;
use bit7z_app_test::TestEntriesUseCase;
use bit7z_domain::archive::TestResult;
use bit7z_domain::archive::*;
use bit7z_domain::repository::*;
use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::v_flex;
use std::sync::Arc;
use bit7z_pres_components::window_dialog::{
    open_window_dialog_async, CloseAction, DialogContent, DialogDescription, DialogFooter,
    DialogHeader, DialogTitle, WindowDialogOptions,
};

pub struct TestDialog;

impl TestDialog {
    pub fn open_with_entries(
        cx: &mut AsyncApp,
        handle: ArchiveHandle,
        indices: Option<Vec<u32>>,
        repo: Arc<dyn ArchiveRepository>,
    ) {
        let total = indices.as_ref().map_or_else(
            || repo.properties(&handle).map(|p| p.files_count() as usize).unwrap_or(0),
            |v| count_expanded(&repo, &handle, v),
        );

        open_window_dialog_async(
            cx,
            WindowDialogOptions {
                title: "Test Archive".into(),
                width: px(520.),
                height: Some(px(200.)),
                min_width: None,
                min_height: None,
                kind: WindowKind::Dialog,
                close_action: CloseAction::RemoveWindow,
                window_decorations: Some(WindowDecorations::Client),
                window_background: WindowBackgroundAppearance::Opaque,
            },
            move |_window, cx| {
                cx.new(move |_| TestContent::new(total, handle, indices, repo))
            },
        );
    }
}

struct TestContent {
    total: usize,
    handle: Option<ArchiveHandle>,
    path: Option<std::path::PathBuf>,
    password: Option<Password>,
    repo: Arc<dyn ArchiveRepository>,
    indices: Option<Vec<u32>>,
}

impl TestContent {
    fn new(
        total: usize,
        handle: ArchiveHandle,
        indices: Option<Vec<u32>>,
        repo: Arc<dyn ArchiveRepository>,
    ) -> Self {
        Self { total, handle: Some(handle), path: None, password: None, repo, indices }
    }

    fn on_start(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        window.remove_window();

        let repo = self.repo.clone();
        let handle = self.handle.clone();
        let path = self.path.clone();
        let password = self.password.clone();
        let indices = self.indices.clone();
        let (progress_tx, progress_rx) = progress_channel();
        let total = self.total;

        struct TestProgressSink(crossbeam_channel::Sender<ProgressUpdate>);
        impl ProgressSink for TestProgressSink {
            fn on_progress(&self, processed: u64, total: u64) {
                let _ = self.0.send(ProgressUpdate {
                    bytes_done: processed,
                    bytes_total: total,
                    ..Default::default()
                });
            }
            fn on_file(&self, path: &str) {
                let _ = self.0.send(ProgressUpdate {
                    current_file: Some(path.to_string()),
                    ..Default::default()
                });
            }
        }

        // Spawn a background task that opens ProgressDialog, runs the test,
        // then opens the result dialog on completion.
        cx.spawn(async move |_this, cx| {
            // Open the progress window (it will poll progress_rx).
            let _ = crate::progress::ProgressDialog::open(
                cx,
                format!("Testing {} entries...", total),
                progress_rx,
                None,
                None,
            );

            // Run the blocking test on a background thread.
            let (result_tx, result_rx) = crossbeam_channel::unbounded();
            cx.background_spawn(async move {
                let dialog_owns_handle = handle.is_none();
                let h = match handle {
                    Some(h) => h,
                    None => path
                        .and_then(|p| repo.open(&p, password.as_ref()).ok())
                        .unwrap_or(ArchiveHandle::new(0)),
                };
                let sink: Arc<dyn ProgressSink> = Arc::new(TestProgressSink(progress_tx));
                let uc = TestEntriesUseCase::new(repo.clone());
                let result = uc.execute(&h, indices.as_deref(), Some(sink));
                if dialog_owns_handle {
                    repo.close(&h);
                }
                let _ = result_tx.send(result);
            })
            .detach();

            // Wait for the result.
            let result = loop {
                match result_rx.try_recv() {
                    Ok(r) => break r,
                    Err(crossbeam_channel::TryRecvError::Empty) => {
                        cx.background_spawn(std::future::ready(())).await;
                    }
                    Err(crossbeam_channel::TryRecvError::Disconnected) => break Err(ArchiveError::Internal("disconnected".into())),
                }
            };

            match result {
                Ok(tr) => show_test_result(cx, tr),
                Err(e) => show_test_error(cx, &e.to_string()),
            }
        })
        .detach();
    }
}

fn show_test_result(cx: &mut AsyncApp, result: TestResult) {
    // AsyncApp derefs to App, so we can pass it directly to open_window_dialog.
    crate::test_results::TestResultsDialog::open(cx, result);
}

fn show_test_error(cx: &mut AsyncApp, msg: &str) {
    let msg = msg.to_string();
    open_window_dialog_async(
        cx,
        WindowDialogOptions {
            title: "Test Error".into(),
            width: px(420.),
            height: Some(px(160.)),
            min_width: None,
            min_height: None,
            kind: WindowKind::Dialog,
            close_action: CloseAction::RemoveWindow,
            window_decorations: Some(WindowDecorations::Client),
            window_background: WindowBackgroundAppearance::Opaque,
        },
        move |_window, cx| cx.new(move |_| TestErrorContent(msg.clone())),
    );
}

impl Render for TestContent {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let h = cx.entity();
        let total = self.total;

        v_flex()
            .size_full()
            .gap(px(12.))
            .child(
                DialogHeader::new()
                    .child(DialogTitle::new().child("Test Archive")),
            )
            .child(
                DialogContent::new().child(
                    v_flex()
                        .h_full()
                        .gap_3()
                        .child(DialogDescription::new().child(format!(
                            "Test {} entr{} for integrity.",
                            total,
                            if total == 1 { "y" } else { "ies" }
                        ))),
                ),
            )
            .child(
                DialogFooter::new().justify_end().gap_2()
                    .child(
                        Button::new("cancel")
                            .label("Cancel")
                            .on_click(move |_, window, _| {
                                window.remove_window();
                            }),
                    )
                    .child(
                        Button::new("start")
                            .label("Start Test")
                            .primary()
                            .on_click({
                                let h = h.clone();
                                move |_, window, cx| {
                                    h.update(cx, |this, cx| this.on_start(window, cx));
                                }
                            }),
                    ),
            )
    }
}

struct TestErrorContent(String);

impl Render for TestErrorContent {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let msg = self.0.clone();
        v_flex()
            .size_full()
            .gap(px(12.))
            .child(
                DialogHeader::new()
                    .child(DialogTitle::new().child("Test Error")),
            )
            .child(
                DialogContent::new().child(
                    div().text_sm().child(msg),
                ),
            )
            .child(
                DialogFooter::new().justify_end()
                    .child(
                        Button::new("close")
                            .label("Close")
                            .primary()
                            .on_click(move |_, window, _| {
                                window.remove_window();
                            }),
                    ),
            )
    }
}

fn failed_entry(f: &bit7z_domain::archive::TestFailure) -> impl IntoElement {
    let reason = match &f.reason {
        bit7z_domain::archive::TestFailureReason::CrcMismatch { expected, actual } => {
            format!("CRC mismatch: expected {:08X}, got {:08X}", expected, actual)
        }
        bit7z_domain::archive::TestFailureReason::ReadError(msg) => {
            format!("Read error: {}", msg)
        }
        bit7z_domain::archive::TestFailureReason::UnsupportedOperation => {
            "Unsupported operation".to_string()
        }
    };
    v_flex().gap_0().py_1()
        .child(div().text_sm().font_weight(FontWeight::MEDIUM).child(format!("#{} {}", f.index, f.path)))
        .child(div().text_xs().child(reason))
}

// ---------------------------------------------------------------------------
// Helpers (from old view.rs)
// ---------------------------------------------------------------------------

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
    if let Ok(page) = repo.list_dir(archive, &path, 0..usize::MAX) {
        for child in &page.items {
            if child.is_directory() {
                count += count_files_recursive(repo, archive, child.path());
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
        if let Ok(page) = repo.list(archive, idx as usize..idx as usize + 1) {
            if let Some(entry) = page.items.first() {
                if entry.is_directory() {
                    count += count_files_recursive(repo, archive, entry.path());
                    continue;
                }
            }
        }
        count += 1;
    }
    count
}
