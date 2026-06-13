use crate::theme::Theme;
use gpui::prelude::FluentBuilder as _;
use gpui::*;

#[derive(Debug, Clone)]
pub enum PasswordDialogEvent {
    Submitted(String),
    Canceled,
}

impl EventEmitter<PasswordDialogEvent> for PasswordDialog {}

pub struct PasswordDialog {
    pub archive_name: String,
    pub password: String,
    pub show_password: bool,
    pub error: Option<String>,
}

impl PasswordDialog {
    pub fn new(archive_name: String, cx: &mut Context<Self>) -> Entity<Self> {
        cx.new(|_cx| Self {
            archive_name,
            password: String::new(),
            show_password: false,
            error: None,
        })
    }
}

impl Render for PasswordDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().flex().flex_col().gap_3().p_4().w(px(360.))
            .child(div().font_weight(FontWeight::BOLD).child("Password Required"))
            .child(div().text_sm().child(format!("The archive \"{}\" is encrypted.", self.archive_name)))
            .child(
                div().flex().flex_row().gap_1().items_center()
                    .child(div().flex_1().px_2().py_1().border_1().border_color(cx.global::<Theme>().border).rounded_md().child("••••••"))
                    .child(div().cursor_pointer().child("👁"))
            )
            .when_some(self.error.as_ref(), |el, err| {
                el.child(div().text_sm().text_color(cx.global::<Theme>().error).child(err.clone()))
            })
            .child(
                div().flex().flex_row().justify_end().gap_2()
                    .child(div().px_3().py_1().rounded_md().cursor_pointer().child("Cancel")
                        .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _e, _window, cx| cx.emit(PasswordDialogEvent::Canceled))))
                    .child(div().px_3().py_1().rounded_md().bg(cx.global::<Theme>().primary).cursor_pointer().child("OK")
                        .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _e, _window, cx| cx.emit(PasswordDialogEvent::Submitted(this.password.clone())))))
            )
    }
}


