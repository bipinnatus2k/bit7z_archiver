mod view;

use bit7z_app_archive::delete::DeleteEntriesUseCase;
use bit7z_infra_progress::progress_channel;
use bit7z_domain::archive::*;
use bit7z_domain::repository::*;
use gpui::*;
use std::sync::Arc;
use view::{DeleteDialogView, DeleteViewIntent};

use bit7z_pres_components::window_dialog::{
    open_window_dialog_async, WindowDialogOptions, CloseAction,
};

pub struct DeleteDialog {
    view: Entity<DeleteDialogView>,
    indices: Vec<u32>,
    handle: ArchiveHandle,
    repo: Arc<dyn ArchiveRepository>,
}

impl DeleteDialog {
    pub fn open(
        cx: &mut AsyncApp,
        indices: Vec<u32>,
        handle: ArchiveHandle,
        repo: Arc<dyn ArchiveRepository>,
    ) {
        open_window_dialog_async(
            cx,
            WindowDialogOptions {
                title: "Delete Entries".into(),
                width: px(420.),
                height: Some(px(220.)),
                min_width: None,
                min_height: None,
                kind: WindowKind::Dialog,
                close_action: CloseAction::RemoveWindow,
                window_decorations: Some(WindowDecorations::Client),
                window_background: WindowBackgroundAppearance::Opaque,
            },
            |window, cx| {
                let view = cx.new(|_cx| DeleteDialogView::new(indices.len() as u64));
                let view_handle = view.clone();
                let dlg = cx.new(|_cx| Self { view, indices, handle, repo });
                let dlg_handle = dlg.clone();
                cx.subscribe::<DeleteDialogView, DeleteViewIntent>(
                    &view_handle,
                    move |_, intent, cx| {
                        dlg_handle.update(cx, |d, cx| d.handle_intent(intent.clone(), cx));
                    },
                )
                .detach();
                dlg
            },
        );
    }

    fn handle_intent(&mut self, intent: DeleteViewIntent, cx: &mut Context<Self>) {
        match intent {
            DeleteViewIntent::Confirm => {
                self.view.update(cx, |v, _| {
                    v.set_processing(0, self.indices.len() as u64, "Deleting entries...")
                });

                let indices = self.indices.clone();
                let mut handle = self.handle.clone();
                let repo = self.repo.clone();
                let (tx, rx) = progress_channel();
                let view = self.view.clone();

                let notifier: Option<
                    Arc<dyn bit7z_domain::repository::ProgressNotifier>,
                > = Some(Arc::new(bit7z_infra_progress::CrossbeamNotifier(tx)));
                cx.background_spawn(async move {
                    let uc = DeleteEntriesUseCase::new(repo);
                    let _ = uc.execute(&mut handle, &indices, notifier);
                })
                .detach();

                cx.spawn(async move |_, cx| {
                    loop {
                        if let Ok(update) = rx.try_recv() {
                            view.update(cx, |v, _| {
                                v.set_processing(
                                    update.items_done,
                                    update.items_total,
                                    &update.current_file.unwrap_or_default(),
                                )
                            });
                            if update.items_done >= update.items_total || update.error.is_some() {
                                break;
                            }
                        }
                        cx.background_spawn(std::future::ready(())).await;
                    }
                    view.update(cx, |v, _| v.set_complete());
                })
                .detach();
            }
            DeleteViewIntent::Cancel | DeleteViewIntent::Close => {}
        }
    }
}

impl Render for DeleteDialog {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        self.view.clone()
    }
}
