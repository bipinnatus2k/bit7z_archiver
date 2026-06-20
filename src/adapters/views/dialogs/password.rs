use crate::theme::Theme;
use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::input::{Input, InputEvent, InputState};

#[derive(Debug, Clone)]
pub enum PasswordDialogEvent {
    Submitted(String),
    Canceled,
}

impl EventEmitter<PasswordDialogEvent> for PasswordDialog {}

pub struct PasswordDialog {
    pub archive_name: String,
    password: String,
    input_state: Entity<InputState>,
    pub error: Option<String>,
    _subscription: Subscription,
}

impl PasswordDialog {
    pub fn new(archive_name: String, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input_state = cx.new(|cx| InputState::new(window, cx).placeholder("Enter password..."));
        let subscription = cx.subscribe_in(&input_state, window, {
            let input_state = input_state.clone();
            move |this: &mut PasswordDialog, _emitter, ev: &InputEvent, _window, cx| match ev {
                InputEvent::Change => {
                    let value = input_state.read(cx).value();
                    this.password = value.to_string();
                }
                _ => {}
            }
        });
        Self {
            archive_name,
            password: String::new(),
            input_state,
            error: None,
            _subscription: subscription,
        }
    }
}

impl Render for PasswordDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        div().flex().flex_col().gap_3().p_4().w(px(360.))
            .child(div().font_weight(FontWeight::BOLD).child("Password Required"))
            .child(div().text_sm().child(format!("The archive \"{}\" is encrypted.", self.archive_name)))
            .child(
                Input::new(&self.input_state)
                    .flex_1()
            )
            .when_some(self.error.as_ref(), |el, err| {
                el.child(div().text_sm().mt_1().text_color(theme.error).child(err.clone()))
            })
            .child(
                div().flex().flex_row().justify_end().gap_2()
                    .child(div().px_3().py_1().rounded_md().cursor_pointer().child("Cancel")
                        .on_mouse_down(gpui::MouseButton::Left, cx.listener(|_this, _e, _window, cx| cx.emit(PasswordDialogEvent::Canceled))))
                    .child(div().px_3().py_1().rounded_md().bg(theme.primary).cursor_pointer().child("OK")
                        .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _e, _window, cx| cx.emit(PasswordDialogEvent::Submitted(this.password.clone())))))
            )
    }
}
