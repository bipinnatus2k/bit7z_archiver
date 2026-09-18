use std::path::PathBuf;
use std::sync::Arc;
use gpui::{div, px, AppContext, AsyncApp, Context, IntoElement, ParentElement, Render, Styled, Window, WindowBackgroundAppearance, WindowDecorations, WindowKind};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::dialog::{DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle};
use gpui_component::v_flex;
use bit7z_app_archive::runtime_service::ArchiveService;
use bit7z_domain::archive::ArchiveHandle;
use bit7z_domain::archive::error::ArchiveError;
use bit7z_domain::archive::test::TestResult;
use bit7z_domain::password::Password;
use bit7z_pres_components::window_dialog::{open_window_dialog_async, CloseAction, WindowDialogOptions};
use crate::progress::ProgressDialog;
use crate::test::test_results;

struct TestContent {
    total: usize,
    handle: Option<ArchiveHandle>,
    path: Option<PathBuf>,
    password: Option<Password>,
    service: Arc<ArchiveService>,
    indices: Option<Vec<u32>>,
}

impl TestContent {
    fn new(
        total: usize,
        handle: ArchiveHandle,
        indices: Option<Vec<u32>>,
        service: Arc<ArchiveService>,
    ) -> Self {
        Self {
            total,
            handle: Some(handle),
            path: None,
            password: None,
            service,
            indices,
        }
    }

    fn on_start(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        window.remove_window();

        let service = self.service.clone();
        let handle = self.handle.clone();
        let path = self.path.clone();
        let password = self.password.clone();
        let indices = self.indices.clone();
        let (progress_tx, progress_rx) = progress_channel();
        let total = self.total;

        // Spawn a background task that opens ProgressDialog, runs the test,
        // then opens the result dialog on completion.
        cx.spawn(async move |_this, cx| {
            // Open the progress window (it will poll progress_rx).
            let _ = ProgressDialog::open(
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
                        .and_then(|p| service.open(&p, password.as_ref()).ok())
                        .unwrap_or(ArchiveHandle::new_reader()),
                };
                let uc = TestEntriesUseCase::new(service.clone());
                let result = uc.execute(&h, indices.as_deref(), Some(progress_tx));
                if dialog_owns_handle {
                    service.close(&h);
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
                    Err(crossbeam_channel::TryRecvError::Disconnected) => {
                        break Err(ArchiveError::Internal("disconnected".into()));
                    }
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
    test_results::TestResultsDialog::open(cx, result);
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
            .child(DialogHeader::new().child(DialogTitle::new().child("Test Archive")))
            .child(
                DialogContent::new().child(v_flex().h_full().gap_3().child(
                    DialogDescription::new().child(format!(
                        "Test {} entr{} for integrity.",
                        total,
                        if total == 1 { "y" } else { "ies" }
                    )),
                )),
            )
            .child(
                DialogFooter::new()
                    .justify_end()
                    .gap_2()
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
            .child(DialogHeader::new().child(DialogTitle::new().child("Test Error")))
            .child(DialogContent::new().child(div().text_sm().child(msg)))
            .child(
                DialogFooter::new().justify_end().child(
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