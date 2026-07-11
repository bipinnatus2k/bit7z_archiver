use bit7z_app_archive::delete::DeleteEntriesUseCase;
use bit7z_infra_progress::progress_channel;
use bit7z_domain::archive::*;
use bit7z_domain::repository::*;
use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::v_flex;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use bit7z_pres_components::window_dialog::{
    open_window_dialog_async, CloseAction, DialogContent, DialogDescription, DialogFooter,
    DialogHeader, DialogTitle, WindowDialogOptions,
};

pub struct DeleteDialog;

impl DeleteDialog {
    pub fn open(
        cx: &mut AsyncApp,
        indices: Vec<u32>,
        handle: ArchiveHandle,
        repo: Arc<dyn ArchiveRepository>,
    ) {
        let count = indices.len();
        open_window_dialog_async(
            cx,
            WindowDialogOptions {
                title: "Delete Entries".into(),
                width: px(420.),
                height: Some(px(200.)),
                min_width: None,
                min_height: None,
                kind: WindowKind::Dialog,
                close_action: CloseAction::RemoveWindow,
                window_decorations: Some(WindowDecorations::Client),
                window_background: WindowBackgroundAppearance::Opaque,
            },
            move |_window, cx| {
                cx.new(move |cx| DeleteContent::new(count, indices, handle, repo, cx))
            },
        );
    }
}

struct DeleteContent {
    count: usize,
    indices: Vec<u32>,
    handle: ArchiveHandle,
    repo: Arc<dyn ArchiveRepository>,
}

impl DeleteContent {
    fn new(
        count: usize,
        indices: Vec<u32>,
        handle: ArchiveHandle,
        repo: Arc<dyn ArchiveRepository>,
        _cx: &mut Context<Self>,
    ) -> Self {
        Self { count, indices, handle, repo }
    }

    fn on_delete(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        window.remove_window();

        let indices = self.indices.clone();
        let mut handle = self.handle.clone();
        let repo = self.repo.clone();
        let (tx, rx) = progress_channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let cancel_clone = cancel.clone();
        let progress_title = format!("Deleting {} entries...", indices.len());

        cx.spawn(async move |_this, cx| {
            // Open the progress window first, then run the delete task.
            crate::progress::ProgressDialog::open(cx, progress_title, rx, Some(cancel_clone), None);

            let notifier: Option<Arc<dyn ProgressNotifier>> =
                Some(Arc::new(bit7z_infra_progress::CrossbeamNotifier(tx)));
            let uc = DeleteEntriesUseCase::new(repo);
            let _ = uc.execute(&mut handle, &indices, notifier);
        })
        .detach();
    }
}

impl Render for DeleteContent {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let h = cx.entity();
        let count = self.count;

        v_flex()
            .size_full()
            .gap(px(12.))
            .child(
                DialogHeader::new()
                    .child(DialogTitle::new().child("Delete Entries")),
            )
            .child(
                DialogContent::new().child(
                    v_flex()
                        .h_full()
                        .gap_3()
                        .child(DialogDescription::new().child(format!(
                            "Are you sure you want to delete {} entr{}?",
                            count,
                            if count == 1 { "y" } else { "ies" },
                        )))
                        .child(
                            DialogDescription::new()
                                .child("This action cannot be undone."),
                        ),
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
                        Button::new("delete")
                            .label("Delete")
                            .danger()
                            .on_click({
                                let h = h.clone();
                                move |_, window, cx| {
                                    h.update(cx, |this, cx| this.on_delete(window, cx));
                                }
                            }),
                    ),
            )
    }
}
