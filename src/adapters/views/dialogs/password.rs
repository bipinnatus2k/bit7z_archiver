use crate::theme::Theme;
use crossbeam::channel::{unbounded, Receiver, Sender};
use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::{h_flex, v_flex};
use crate::domain::archive::Password;

#[derive(Debug, Clone)]
pub enum PasswordDialogEvent {
    Submitted(String),
    Canceled,
}

impl EventEmitter<PasswordDialogEvent> for PasswordDialog {}

pub enum PasswordResult {
    Submitted(String),
    Canceled,
}

pub struct PasswordDialog {
    archive_name: String,
    password: Password,
    // show_password: bool,
    input_state: Entity<InputState>,
    pub error: Option<String>,
    _subscription: Subscription,
    result_tx: Option<Sender<PasswordResult>>,
}

impl PasswordDialog {
    pub fn new(archive_name: String, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input_state = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Enter password...")
                .masked(true)
        });
        let subscription = cx.subscribe_in(&input_state, window, {
            let input_state = input_state.clone();
            move |this: &mut PasswordDialog, _emitter, ev: &InputEvent, _window, cx| match ev {
                InputEvent::Change => {
                    let value = input_state.read(cx).value().clone();
                    this.password = Password::new(value);
                }
                _ => {}
            }
        });
        Self {
            archive_name,
            password: Password::new(""),
            // show_password: false,
            input_state,
            error: None,
            _subscription: subscription,
            result_tx: None,
        }
    }

    /// Open as independent window. Returns a receiver for the result.
    pub fn open(archive_name: String, cx: &mut AsyncApp) -> Receiver<PasswordResult> {
        let (tx, rx) = unbounded::<PasswordResult>();
        let tx = std::sync::Mutex::new(Some(tx));
        cx.spawn(async move |cx| {
            let _ = cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                        point(px(200.), px(200.)),
                        size(px(420.), px(240.)),
                    ))),
                    window_background: WindowBackgroundAppearance::Opaque,
                    window_decorations: Some(WindowDecorations::Client),
                    focus: true,
                    kind: WindowKind::PopUp,
                    ..Default::default()
                },
                move |window, cx| {
                    let tx_lock = tx.lock().unwrap().take().unwrap();
                    let dialog = cx.new(|cx| PasswordDialog::new(archive_name, window, cx));
                    dialog.update(cx, |d, _| d.result_tx = Some(tx_lock));
                    cx.new(|cx| gpui_component::Root::new(dialog, window, cx))
                },
            );
        })
        .detach();
        rx
    }

    fn submit(&mut self) {
        if let Some(tx) = self.result_tx.take() {
            let _ = tx.send(PasswordResult::Submitted(self.password.as_str().parse().unwrap()));
        }
    }

    fn cancel(&mut self) {
        if let Some(tx) = self.result_tx.take() {
            let _ = tx.send(PasswordResult::Canceled);
        }
    }

    fn submit_and_close(&mut self, window: &mut Window) {
        self.submit();
        window.remove_window();
    }

    fn cancel_and_close(&mut self, window: &mut Window) {
        self.cancel();
        window.remove_window();
    }
}

impl Render for PasswordDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let has_error = self.error.is_some();
        let err = self.error.clone();
        // let show_password = self.show_password;

        v_flex()
            .p_4()
            .gap_4()
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, _cx| {
                if event.keystroke.key == "enter" {
                    this.submit_and_close(window);
                }
                if event.keystroke.key == "escape" {
                    this.cancel_and_close(window);
                }
            }))
            .child(
                div()
                    .font_weight(FontWeight::BOLD)
                    .text_lg()
                    .child("Password Required"),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child(format!(
                        "The archive \"{}\" is encrypted.",
                        self.archive_name
                    )),
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(Input::new(&self.input_state).mask_toggle().flex_1())
                ,
            )
            .when(has_error, |el| {
                el.child(
                    div()
                        .text_sm()
                        .mt_1()
                        .text_color(theme.error)
                        .child(err.unwrap_or_default()),
                )
            })
            .child(
                h_flex()
                    .justify_end()
                    .gap_2()
                    .child(
                        Button::new("cancel")
                            .outline()
                            .label("Cancel")
                            .on_click(cx.listener(|this, _, window, _| {
                                this.cancel_and_close(window);
                            })),
                    )
                    .child(
                        Button::new("ok")
                            .primary()
                            .label("OK")
                            .on_click(cx.listener(|this, _, window, _| {
                                this.submit_and_close(window);
                            })),
                    ),
            )
    }
}
