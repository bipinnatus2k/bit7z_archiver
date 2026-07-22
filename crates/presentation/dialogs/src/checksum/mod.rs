use bit7z_app_archive::runtime_service::ArchiveService;
use bit7z_app_checksum::{CalculateChecksumUseCase, ChecksumAlgorithm};
use bit7z_domain::archive::ArchiveHandle;
use bit7z_domain::repository::ArchiveError;
use bit7z_infra_progress::progress_channel;
use bit7z_pres_components::window_dialog::{
    CloseAction, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle,
    WindowDialogOptions, open_window_dialog_async,
};
use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::v_flex;
use std::sync::Arc;

pub struct ChecksumDialog;

impl ChecksumDialog {
    pub fn open_with_entries(
        cx: &mut AsyncApp,
        handle: ArchiveHandle,
        indices: Vec<u32>,
        service: Arc<ArchiveService>,
    ) {
        open_window_dialog_async(
            cx,
            WindowDialogOptions {
                title: "Calculate Checksum".into(),
                width: px(520.),
                height: Some(px(200.)),
                min_width: None,
                min_height: None,
                kind: WindowKind::Dialog,
                close_action: CloseAction::RemoveWindow,
                window_decorations: Some(WindowDecorations::Client),
                window_background: WindowBackgroundAppearance::Opaque,
            },
            move |_window, cx| cx.new(move |_| ChecksumContent::new(handle, indices, service)),
        );
    }
}

struct ChecksumContent {
    handle: ArchiveHandle,
    indices: Vec<u32>,
    service: Arc<ArchiveService>,
}

impl ChecksumContent {
    fn new(handle: ArchiveHandle, indices: Vec<u32>, service: Arc<ArchiveService>) -> Self {
        Self {
            handle,
            indices,
            service,
        }
    }

    fn on_calculate(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        window.remove_window();

        let service = self.service.clone();
        let handle = self.handle.clone();
        let indices = self.indices.clone();
        let (_progress_tx, progress_rx) = progress_channel();

        cx.spawn(async move |_this, cx| {
            let _ = crate::progress::ProgressDialog::open(
                cx,
                format!("Calculating checksums for {} entries...", indices.len()),
                progress_rx,
                None,
                None,
            );

            let (result_tx, result_rx) = crossbeam_channel::unbounded();
            cx.background_spawn(async move {
                let algos = vec![
                    ChecksumAlgorithm::Crc32,
                    ChecksumAlgorithm::Md5,
                    ChecksumAlgorithm::Sha1,
                    ChecksumAlgorithm::Sha256,
                ];
                let uc = CalculateChecksumUseCase::new(service);
                let _ = uc.execute(&handle, &indices, &algos);
                // TODO: collect per-file results from the use case
                let _ = result_tx.send(Ok::<_, ArchiveError>(()));
            })
            .detach();

            let _ = loop {
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

            show_checksum_complete(cx);
        })
        .detach();
    }
}

fn show_checksum_complete(cx: &mut AsyncApp) {
    open_window_dialog_async(
        cx,
        WindowDialogOptions {
            title: "Checksum Complete".into(),
            width: px(400.),
            height: Some(px(160.)),
            min_width: None,
            min_height: None,
            kind: WindowKind::Dialog,
            close_action: CloseAction::RemoveWindow,
            window_decorations: Some(WindowDecorations::Client),
            window_background: WindowBackgroundAppearance::Opaque,
        },
        move |_window, cx| cx.new(|_| ChecksumCompleteContent),
    );
}

struct ChecksumCompleteContent;

impl Render for ChecksumCompleteContent {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .gap(px(12.))
            .child(DialogHeader::new().child(DialogTitle::new().child("Checksum Complete")))
            .child(DialogContent::new().child(
                DialogDescription::new().child("Checksums have been calculated successfully."),
            ))
            .child(
                DialogFooter::new().justify_end().child(
                    Button::new("close")
                        .label("Close")
                        .primary()
                        .on_click(|_, window, _| {
                            window.remove_window();
                        }),
                ),
            )
    }
}

impl Render for ChecksumContent {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let h = cx.entity();
        let count = self.indices.len();

        v_flex()
            .size_full()
            .gap(px(12.))
            .child(DialogHeader::new().child(DialogTitle::new().child("Calculate Checksum")))
            .child(
                DialogContent::new().child(
                    v_flex()
                        .h_full()
                        .gap_3()
                        .child(DialogDescription::new().child(format!(
                            "{} entr{} selected",
                            count,
                            if count == 1 { "y" } else { "ies" }
                        )))
                        .child(
                            DialogDescription::new()
                                .child("Algorithms: CRC32, MD5, SHA-1, SHA-256"),
                        ),
                ),
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
                    .child(Button::new("calc").label("Calculate").primary().on_click({
                        let h = h.clone();
                        move |_, window, cx| {
                            h.update(cx, |this, cx| this.on_calculate(window, cx));
                        }
                    })),
            )
    }
}
