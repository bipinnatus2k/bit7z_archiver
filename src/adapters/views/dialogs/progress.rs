use crate::theme::Theme;
use gpui::prelude::FluentBuilder as _;
use gpui::*;

pub struct ProgressDialog {
    pub message: String,
    pub current: u64,
    pub total: u64,
    pub is_canceled: bool,
    pub is_complete: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum ProgressDialogEvent {
    Cancel,
    HideToTray,
    Close,
}

impl EventEmitter<ProgressDialogEvent> for ProgressDialog {}

impl ProgressDialog {
    pub fn new(message: impl Into<String>, cx: &mut Context<Self>) -> Entity<Self> {
        cx.new(|_cx| Self {
            message: message.into(),
            current: 0,
            total: 0,
            is_canceled: false,
            is_complete: false,
            error: None,
        })
    }

    pub fn update(&mut self, current: u64, total: u64, message: String, cx: &mut Context<Self>) {
        self.current = current;
        self.total = total;
        self.message = message;
        cx.notify();
    }

    pub fn set_complete(&mut self, message: String, cx: &mut Context<Self>) {
        self.is_complete = true;
        self.message = message;
        cx.notify();
    }

    pub fn set_error(&mut self, error: String, cx: &mut Context<Self>) {
        self.error = Some(error);
        cx.notify();
    }
}

impl Render for ProgressDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let pct = if self.total > 0 { (self.current as f32 / self.total as f32) * 100.0 } else { 0.0 };
        div().flex().flex_col().gap_3().p_4().w(px(420.))
            .child(div().font_weight(FontWeight::BOLD).child(if self.is_complete { "Complete" } else { "Operation in Progress" }))
            .child(div().text_sm().child(self.message.clone()))
            .child(
                div().w_full().h(px(6.)).bg(cx.global::<Theme>().muted).rounded_full()
                    .child(div().h_full().w(relative(pct / 100.0)).bg(cx.global::<Theme>().primary).rounded_full())
            )
            .child(div().text_sm().text_color(cx.global::<Theme>().muted).child(format!("{:.0}%", pct)))
            .when_some(self.error.as_ref(), |el, err| {
                el.child(div().text_sm().text_color(cx.global::<Theme>().error).child(err.clone()))
            })
            .child(
                div().flex().flex_row().justify_end().gap_2()
                    .when(!self.is_complete, |el| {
                        el.child(div().px_3().py_1().rounded_md().cursor_pointer().child("Hide to Tray")
                            .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _e, _window, cx| cx.emit(ProgressDialogEvent::HideToTray))))
                    })
                    .when(!self.is_complete && !self.is_canceled, |el| {
                        el.child(div().px_3().py_1().rounded_md().cursor_pointer().child("Cancel")
                            .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _e, _window, cx| cx.emit(ProgressDialogEvent::Cancel))))
                    })
                    .when(self.is_complete || self.is_canceled, |el| {
                        el.child(div().px_3().py_1().rounded_md().bg(cx.global::<Theme>().primary).cursor_pointer().child("Close")
                            .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _e, _window, cx| cx.emit(ProgressDialogEvent::Close))))
                    })
            )
    }
}


