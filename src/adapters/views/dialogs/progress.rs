use crate::adapters::view_models::progress_vm::ProgressState;
use crate::theme::Theme;
use gpui::prelude::FluentBuilder as _;
use gpui::*;

pub struct ProgressDialog {
    pub is_complete: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum ProgressDialogEvent {
    Pause,
    Resume,
    Cancel,
    Hide,
    Close,
}

impl EventEmitter<ProgressDialogEvent> for ProgressDialog {}

impl ProgressDialog {
    pub fn new(cx: &mut Context<Self>) -> Entity<Self> {
        cx.new(|_cx| Self {
            is_complete: false,
            error: None,
        })
    }
}

impl Render for ProgressDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = cx.global::<ProgressState>().clone();
        let theme = cx.global::<Theme>().clone();

        let file_pct = if state.file_total > 0 {
            (state.file_current as f32 / state.file_total as f32) * 100.0
        } else {
            0.0
        };
        let overall_pct = state.percent() * 100.0;
        let is_done = state.is_complete;
        let is_paused = state.is_paused;
        let is_active = state.is_active;

        div().flex().flex_col().gap_3().p_4().w(px(420.))
            .child(div().font_weight(FontWeight::BOLD).text_lg().child(
                if is_done {
                    if state.error.is_some() { "Operation Failed" } else { "Complete" }
                } else if is_paused {
                    "Paused"
                } else {
                    "Operation in Progress"
                }
            ))
            .child(div().text_sm().child(state.message.clone()))
            // Current file label
            .when_some(state.current_file.as_ref(), |el, fname| {
                el.child(div().text_sm().text_color(theme.muted).child(format!("Current file: {}", fname)))
            })
            // File progress bar (top)
            .child(
                div().flex().flex_col().gap_1()
                    .child(div().text_xs().text_color(theme.muted).child(format!(
                        "File: {} / {} ({:.0}%)",
                        state.file_current, state.file_total, file_pct
                    )))
                    .child(
                        div().w_full().h(px(6.)).bg(theme.muted).rounded_full()
                            .child(div().h_full().w(relative(file_pct / 100.0)).bg(theme.primary).rounded_full())
                    )
            )
            // Overall progress bar (bottom)
            .child(
                div().flex().flex_col().gap_1()
                    .child(div().text_xs().text_color(theme.muted).child(format!(
                        "Overall: {}/{} items, {} / {} ({:.0}%)",
                        state.items_done, state.items_total,
                        state.bytes_done, state.bytes_total, overall_pct
                    )))
                    .child(
                        div().w_full().h(px(6.)).bg(theme.muted).rounded_full()
                            .child(div().h_full().w(relative(overall_pct / 100.0)).bg(theme.primary).rounded_full())
                    )
            )
            .when_some(state.error.as_ref(), |el, err| {
                el.child(div().text_sm().text_color(theme.error).child(err.clone()))
            })
            // Buttons
            .child(
                div().flex().flex_row().justify_end().gap_2()
                    // Done state: Close button
                    .when(is_done, |el| {
                        el.child(
                            div().px_3().py_1().rounded_md().bg(theme.primary).cursor_pointer().child("Close")
                                .on_mouse_down(MouseButton::Left, cx.listener(|_this, _e, _window, cx| {
                                    cx.emit(ProgressDialogEvent::Close);
                                }))
                        )
                    })
                    // Active state: Pause/Resume, Hide, Cancel
                    .when(!is_done && is_active, |el| {
                        el.child(
                            div().px_3().py_1().rounded_md().cursor_pointer()
                                .hover(|mut s| { s.background = Some(theme.hover.into()); s })
                                .child(if is_paused { "Resume" } else { "Pause" })
                                .on_mouse_down(MouseButton::Left, cx.listener(|_this, _e, _window, cx| {
                                    if cx.global::<ProgressState>().is_paused {
                                        cx.emit(ProgressDialogEvent::Resume);
                                    } else {
                                        cx.emit(ProgressDialogEvent::Pause);
                                    }
                                }))
                        )
                        .child(
                            div().px_3().py_1().rounded_md().cursor_pointer()
                                .hover(|mut s| { s.background = Some(theme.hover.into()); s })
                                .child("Hide")
                                .on_mouse_down(MouseButton::Left, cx.listener(|_this, _e, _window, cx| {
                                    cx.update_global::<ProgressState, _>(|state, _cx| {
                                        state.is_hidden_to_tray = true;
                                    });
                                    cx.emit(ProgressDialogEvent::Hide);
                                }))
                        )
                        .child(
                            div().px_3().py_1().rounded_md().cursor_pointer()
                                .hover(|mut s| { s.background = Some(theme.hover.into()); s })
                                .child("Cancel")
                                .on_mouse_down(MouseButton::Left, cx.listener(|_this, _e, _window, cx| {
                                    cx.emit(ProgressDialogEvent::Cancel);
                                }))
                        )
                    })
            )
    }
}
