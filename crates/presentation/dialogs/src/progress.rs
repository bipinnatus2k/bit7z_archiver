use bit7z_domain::repository::ProgressUpdate;
use bit7z_infra_progress::ProgressReceiver;
use crossbeam_channel::{TryRecvError, Receiver};
use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::progress::Progress;
use gpui_component::{h_flex, v_flex, Sizable};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use humansize::{format_size, BINARY};
use bit7z_pres_components::window_dialog::{
    open_window_dialog_async, CloseAction, DialogContent, DialogHeader, DialogTitle,
    WindowDialogOptions,
};

pub struct ProgressDialog {
    pub title: String,
    pub message: String,
    pub current: u64,
    pub total: u64,
    pub file_current: u64,
    pub file_total: u64,
    pub is_complete: bool,
    pub error: Option<String>,
    rx: Receiver<ProgressUpdate>,
    cancel: Option<Arc<AtomicBool>>,
    paused: Option<Arc<AtomicBool>>,
    is_paused: bool,
}

impl ProgressDialog {
    fn new(title: String, rx: Receiver<ProgressUpdate>) -> Self {
        Self {
            title,
            message: String::new(),
            current: 0,
            total: 1,
            file_current: 0,
            file_total: 0,
            is_complete: false,
            error: None,
            rx,
            cancel: None,
            paused: None,
            is_paused: false,
        }
    }

    pub fn open(
        cx: &mut AsyncApp,
        title: String,
        rx: ProgressReceiver,
        cancel: Option<Arc<AtomicBool>>,
        paused: Option<Arc<AtomicBool>>,
    ) {
        open_window_dialog_async(
            cx,
            WindowDialogOptions {
                title: title.clone().into(),
                width: px(440.),
                height: Some(px(260.)),
                min_width: None,
                min_height: None,
                kind: WindowKind::Dialog,
                close_action: CloseAction::RemoveWindow,
                window_decorations: Some(WindowDecorations::Client),
                window_background: WindowBackgroundAppearance::Opaque,
            },
            move |_window, cx| {
                let mut dlg = cx.new(|_cx| ProgressDialog::new(title, rx));
                if let Some(c) = cancel {
                    dlg.update(cx, |d, _| d.cancel = Some(c));
                }
                if let Some(p) = paused {
                    dlg.update(cx, |d, _| d.paused = Some(p));
                }
                let poll = dlg.clone();
                cx.spawn(async move |cx| {
                    loop {
                        cx.background_spawn(async move {
                            std::thread::sleep(std::time::Duration::from_millis(80));
                        })
                        .await;
                        let done = poll.update(cx, |d, cx| {
                            d.poll_updates();
                            cx.notify();
                            d.is_complete
                        });
                        if done {
                            break;
                        }
                    }
                })
                .detach();
                dlg
            },
        );
    }

    fn toggle_pause(&mut self) {
        self.is_paused = !self.is_paused;
        if let Some(ref paused) = self.paused {
            paused.store(self.is_paused, Ordering::Relaxed);
        }
    }

    fn do_cancel(&mut self) {
        if let Some(ref cancel) = self.cancel {
            cancel.store(true, Ordering::Relaxed);
        }
    }

    fn poll_updates(&mut self) {
        loop {
            match self.rx.try_recv() {
                Ok(update) => {
                    self.current = update.bytes_done;
                    self.total = update.bytes_total;
                    self.file_current = update.file_current;
                    self.file_total = update.file_total;
                    self.message = update.current_file.unwrap_or_else(|| {
                        if update.items_total > 0 {
                            format!("{}/{} items", update.items_done, update.items_total)
                        } else {
                            String::new()
                        }
                    });
                    if let Some(ref err) = update.error {
                        self.error = Some(err.clone());
                        self.is_complete = true;
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.is_complete = true;
                    break;
                }
            }
        }
    }
}

impl Render for ProgressDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.poll_updates();

        let total_pct = if self.total > 0 {
            (self.current as f64 / self.total as f64 * 100.0) as f32
        } else {
            0.0
        };
        let file_pct = if self.file_total > 0 {
            (self.file_current as f64 / self.file_total as f64 * 100.0) as f32
        } else {
            0.0
        };
        let msg = if self.is_complete {
            if self.error.is_some() {
                "Failed".to_string()
            } else {
                "Complete".to_string()
            }
        } else {
            self.message.clone()
        };

        v_flex()
            .size_full()
            .gap(px(8.))
            .child(
                DialogHeader::new()
                    .child(DialogTitle::new().child(self.title.clone())),
            )
            .child(
                DialogContent::new().child(
                    v_flex()
                        .gap_3()
                        .child(
                            v_flex()
                                .gap_1()
                                .child(
                                    h_flex()
                                        .justify_between()
                                        .child(div().text_sm().child(msg))
                                        .child(div().text_sm().child(format!("{:.0}%", total_pct))),
                                )
                                .child(Progress::new("total").value(total_pct)),
                        )
                        .when(!self.is_complete, |el| {
                            el.child(
                                v_flex()
                                    .gap_1()
                                    .child(
                                        h_flex()
                                            .justify_between()
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .child(format!("File: {} / {}", format_size(self.file_current, BINARY), format_size(self.file_total, BINARY))),
                                            )
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .child(format!("{:.0}%", file_pct)),
                                            ),
                                    )
                                    .child(Progress::new("file").value(file_pct).small()),
                            )
                        })
                        .when(!self.is_complete && self.cancel.is_some(), |el| {
                            el.child(
                                h_flex()
                                    .gap_2()
                                    .justify_end()
                                    .child(
                                        Button::new("pause")
                                            .label(if self.is_paused { "Resume" } else { "Pause" })
                                            .on_click(
                                                cx.listener(|this, _, _, _| this.toggle_pause()),
                                            ),
                                    )
                                    .child(
                                        Button::new("cancel")
                                            .label("Cancel")
                                            .on_click(
                                                cx.listener(|this, _, _, _| this.do_cancel()),
                                            ),
                                    ),
                            )
                        })
                        .when(self.error.is_some(), |el| {
                            el.child(
                                div()
                                    .text_sm()
                                    .child(self.error.clone().unwrap_or_default()),
                            )
                        }),
                ),
            )

    }
}
