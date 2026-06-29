use crate::application::progress::{ProgressReceiver, ProgressUpdate};
use crate::theme::Theme;
use crossbeam::channel::{TryRecvError, unbounded, Receiver};
use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::button::Button;
use gpui_component::progress::Progress;
use gpui_component::{h_flex, v_flex, Sizable};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

pub enum ProgressEvent {
    Canceled,
}

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
    pub fn new(title: String, rx: Receiver<ProgressUpdate>) -> Self {
        Self { title, message: String::new(), current: 0, total: 1, file_current: 0, file_total: 0, is_complete: false, error: None, rx, cancel: None, paused: None, is_paused: false }
    }

    pub fn open(cx: &mut AsyncApp, title: String, rx: ProgressReceiver, cancel: Option<Arc<AtomicBool>>, paused: Option<Arc<AtomicBool>>) -> Receiver<ProgressEvent> {
        let (_tx, event_rx) = unbounded::<ProgressEvent>();
        cx.spawn(async move |cx| {
            let bounds = cx.update(|app| {
                WindowBounds::centered(size(px(440.), px(260.)), app)
            });
            let _ = cx.open_window(
                WindowOptions {
                    window_bounds: Some(bounds),
                    window_background: WindowBackgroundAppearance::Opaque,
                    window_decorations: Some(WindowDecorations::Client),
                    focus: true,
                    titlebar: Some(TitlebarOptions {
                        title: Some(SharedString::from(title.clone())),
                        appears_transparent: false,
                        ..Default::default()
                    }),
                    is_minimizable: true,
                    ..Default::default()
                },
                move |window, cx| {
                    let dialog = cx.new(|_cx| ProgressDialog::new(title, rx));
                    if let Some(c) = cancel.clone() {
                        dialog.update(cx, |d, _| d.set_cancel(c));
                    }
                    if let Some(p) = paused.clone() {
                        dialog.update(cx, |d, _| d.set_paused(p));
                    }
                    let poll_dialog = dialog.clone();
                    cx.spawn(async move |cx| {
                        loop {
                            cx.background_spawn(async move {
                                std::thread::sleep(std::time::Duration::from_millis(80));
                            }).await;
                            let done = poll_dialog.update(cx, |d, cx| {
                                d.poll_updates();
                                cx.notify();
                                d.is_complete
                            });
                            if done { break; }
                        }
                    }).detach();
                    cx.new(|cx| gpui_component::Root::new(dialog, window, cx))
                },
            );
        }).detach();
        event_rx
    }

    pub fn set_cancel(&mut self, cancel: Arc<AtomicBool>) {
        self.cancel = Some(cancel);
    }

    pub fn set_paused(&mut self, paused: Arc<AtomicBool>) {
        self.paused = Some(paused);
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

    pub fn poll_updates(&mut self) {
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

    pub fn update(&mut self, message: &str, current: u64, total: u64, err: Option<String>) {
        self.message = message.to_string();
        self.current = current;
        self.total = total;
        self.is_complete = err.is_some() || current >= total;
        self.error = err;
    }
}

impl Render for ProgressDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.poll_updates();

        let theme = cx.global::<Theme>();
        let total_pct = if self.total > 0 { (self.current as f64 / self.total as f64 * 100.0) as f32 } else { 0.0 };
        let file_pct = if self.file_total > 0 { (self.file_current as f64 / self.file_total as f64 * 100.0) as f32 } else { 0.0 };
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
            .p_3()
            .gap_3()
            .w_full()
            .child(div().font_weight(FontWeight::BOLD).text_lg().child(self.title.clone()))
            .child(div().text_sm().child(msg))
            // Total progress
            .child(
                v_flex()
                    .gap_1()
                    .child(
                        h_flex()
                            .justify_between()
                            .child(div().text_sm().text_color(theme.muted).child(format!("Total: {} / {}", format_bytes(self.current), format_bytes(self.total))))
                            .child(div().text_sm().text_color(theme.muted).child(format!("{:.0}%", total_pct))),
                    )
                    .child(Progress::new("total").value(total_pct)),
            )
            // Current file progress
            .when(!self.is_complete, |el| {
                el.child(
                    v_flex()
                        .gap_1()
                        .child(
                            h_flex()
                                .justify_between()
                                .child(div().text_sm().text_color(theme.muted).child(format!("File: {} / {}", format_bytes(self.file_current), format_bytes(self.file_total))))
                                .child(div().text_sm().text_color(theme.muted).child(format!("{:.0}%", file_pct))),
                        )
                        .child(Progress::new("file").value(file_pct).small().color(theme.selection)),
                )
            })
            // Cancel / Pause buttons
            .when(!self.is_complete && self.cancel.is_some(), |el| {
                el.child(
                    h_flex()
                        .gap_2()
                        .justify_end()
                        .child(Button::new("pause").label(if self.is_paused { "Resume" } else { "Pause" }).on_click(cx.listener(|this, _, _, _| this.toggle_pause())))
                        .child(Button::new("cancel").label("Cancel").on_click(cx.listener(|this, _, _, _| this.do_cancel()))),
                )
            })
            .when(self.error.is_some(), |el| {
                el.child(div().text_sm().mt_1().text_color(theme.error).child(self.error.clone().unwrap_or_default()))
            })
    }
}
