use bit7z_domain::archive::progress::ProgressUpdate;
use bit7z_infra_progress::ProgressReceiver;
use bit7z_infra_tray::{TrayCommand, TrayGlobal, TrayManager};
use bit7z_pres_components::window_dialog::{
    CloseAction, DialogContent, DialogHeader, DialogTitle, WindowDialogOptions,
    open_window_dialog_async,
};
use crossbeam_channel::{Receiver, TryRecvError};
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::button::Button;
use gpui_component::dialog::{DialogClose, DialogFooter};
use gpui_component::progress::Progress;
use gpui_component::{Disableable, Sizable, h_flex, v_flex};
use humansize::{BINARY, format_size};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

pub struct ProgressDialog {
    pub title: String,
    pub message: String,
    pub current: u64,
    pub total: u64,
    pub file_current: u64,
    pub file_total: u64,
    pub is_all_complete: bool,
    pub is_cancel: bool,
    pub error: Option<String>,
    rx: Receiver<ProgressUpdate>,
    canceled: Option<Arc<AtomicBool>>,
    paused: Option<Arc<AtomicBool>>,
    is_paused: bool,
    is_background: bool,
    tray: Option<Arc<TrayManager>>,
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
            is_all_complete: false,
            is_cancel: false,
            error: None,
            rx,
            canceled: None,
            paused: None,
            is_paused: false,
            is_background: false,
            tray: None,
        }
    }

    pub fn open(
        cx: &mut AsyncApp,
        title: String,
        rx: ProgressReceiver,
        canceled: Option<Arc<AtomicBool>>,
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
                let dlg = cx.new(|_cx| ProgressDialog::new(title, rx));
                if let Some(c) = canceled {
                    dlg.update(cx, |d, _| {
                        d.is_cancel = true;
                        d.canceled = Some(c)
                    });
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
                            d.update_tray();
                            cx.notify();
                            d.is_all_complete
                        });
                        if done {
                            let _ = poll.update(cx, |d, _| d.finalize_tray());
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
        self.is_cancel = true;
        if let Some(ref canceled) = self.canceled {
            canceled.store(true, Ordering::Relaxed);
        }
    }

    fn send_to_background(&mut self, cx: &mut App) {
        self.tray = cx.try_global::<TrayGlobal>().map(|g| g.0.clone());
        self.is_background = true;
    }

    fn update_tray(&self) {
        if !self.is_background {
            return;
        }
        if let Some(ref tray) = self.tray {
            let pct = if self.total > 0 {
                (self.current as f64 / self.total as f64 * 100.0) as f32
            } else {
                0.0
            };
            tray.send(TrayCommand::UpdateProgress {
                message: self.message.clone(),
                percent: pct,
            });
        }
    }

    fn finalize_tray(&self) {
        if let Some(ref tray) = self.tray {
            tray.send(TrayCommand::Idle);
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
                        self.is_all_complete = true;
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.is_all_complete = true;
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
        let msg = if self.is_all_complete {
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
            .child(DialogHeader::new().child(DialogTitle::new().child(self.title.clone())))
            .child(
                DialogContent::new().child(
                    v_flex()
                        .gap_3()
                        .child(
                            h_flex()
                                .justify_between()
                                .child(div().text_sm().child(msg))
                                .child(div().text_sm().child(format!("{:.0}%", total_pct))),
                        )
                        .child(Progress::new("total").value(total_pct).w_full())
                        .child(
                            h_flex()
                                .justify_between()
                                .child(div().text_sm().child(format!(
                                    "File: {} / {}",
                                    format_size(self.file_current, BINARY),
                                    format_size(self.file_total, BINARY)
                                )))
                                .child(div().text_sm().child(format!("{:.0}%", file_pct))),
                        )
                        .child(Progress::new("file").value(file_pct).small()),
                ),
            )
            .child(
                DialogFooter::new()
                    .child(
                        Button::new("pause")
                            .disabled(self.is_all_complete || self.is_cancel)
                            .label(if self.is_paused { "Resume" } else { "Pause" })
                            .on_click(cx.listener(|this, _, _, _| this.toggle_pause())),
                    )
                    .child(
                        Button::new("background")
                            .label("Background")
                            // .disabled(self.is_all_complete || self.is_cancel || self.is_background)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.send_to_background(cx);
                                window.remove_window();
                            })),
                    )
                    .child(DialogClose::new().when_else(
                        !self.is_all_complete && !self.is_cancel,
                        |el| {
                            el.child(
                                Button::new("cancel")
                                    .label("Cancel")
                                    .disabled(self.is_cancel)
                                    .on_click(cx.listener(|this, _, _, _| this.do_cancel())),
                            )
                        },
                        |el| {
                            el.child(Button::new("finish").label("Finish").on_click(
                                cx.listener(|_, _click_event, window, _| window.remove_window()),
                            ))
                        },
                    )),
            )
    }
}
