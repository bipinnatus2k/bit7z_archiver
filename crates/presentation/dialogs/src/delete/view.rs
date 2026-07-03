use bit7z_pres_theme::Theme;
use gpui::*;
use gpui_component::h_flex;

#[derive(Debug, Clone, PartialEq)]
pub enum DeleteViewIntent {
    Confirm,
    Cancel,
    Close,
}

impl EventEmitter<DeleteViewIntent> for DeleteDialogView {}

pub enum DeletePhase {
    Idle { count: u64 },
    Processing { current: u64, total: u64, message: String },
    Complete,
    #[allow(dead_code)]
    Error(String),
}

pub struct DeleteDialogView {
    phase: DeletePhase,
}

impl DeleteDialogView {
    pub fn new(count: u64) -> Self {
        Self { phase: DeletePhase::Idle { count } }
    }

    pub fn set_processing(&mut self, current: u64, total: u64, message: &str) {
        self.phase = DeletePhase::Processing { current, total, message: message.to_string() };
    }

    pub fn set_complete(&mut self) {
        self.phase = DeletePhase::Complete;
    }

    #[allow(dead_code)]
    pub fn set_error(&mut self, msg: &str) {
        self.phase = DeletePhase::Error(msg.to_string());
    }
}

impl Render for DeleteDialogView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div().flex().flex_col().gap_3().p_4().w(px(400.))
            .child(match &self.phase {
                DeletePhase::Idle { count } => {
                    div().flex().flex_col().gap_3()
                        .child(div().font_weight(FontWeight::BOLD).text_lg().child("Delete Entries"))
                        .child(div().text_sm().child(format!("Are you sure you want to delete {} entr{}?", count, if *count == 1 { "y" } else { "ies" })))
                        .child(div().text_sm().text_color(theme.muted).child("This action cannot be undone."))
                        .child(h_flex().justify_end().gap_2()
                            .child(div().px_3().py_1().rounded_md().cursor_pointer().child("Cancel")
                                .on_mouse_down(MouseButton::Left, cx.listener(|_, _, _window, cx| cx.emit(DeleteViewIntent::Cancel))))
                            .child(div().px_3().py_1().rounded_md().bg(theme.error).cursor_pointer().child("Delete")
                                .on_mouse_down(MouseButton::Left, cx.listener(|_, _, _window, cx| cx.emit(DeleteViewIntent::Confirm)))))
                }
                DeletePhase::Processing { current, total, message } => {
                    let pct = if *total > 0 { (*current as f64 / *total as f64 * 100.0) as u32 } else { 0 };
                    div().flex().flex_col().gap_3()
                        .child(div().font_weight(FontWeight::BOLD).child("Deleting..."))
                        .child(div().text_sm().child(message.clone()))
                        .child(div().flex().flex_row().gap_2().items_center()
                            .child(div().flex_1().h(px(20.)).bg(theme.surface).rounded_md().overflow_hidden()
                                .child(div().h_full().bg(theme.primary).rounded_md().w(px(pct as f32 * 4.0))))
                            .child(div().text_sm().child(format!("{}%", pct))))
                }
                DeletePhase::Complete => {
                    div().flex().flex_col().gap_3()
                        .child(div().font_weight(FontWeight::BOLD).child("Delete Complete"))
                        .child(div().text_sm().child("Selected entries have been deleted."))
                        .child(h_flex().justify_end()
                            .child(div().px_3().py_1().rounded_md().bg(theme.primary).cursor_pointer().child("Close")
                                .on_mouse_down(MouseButton::Left, cx.listener(|_, _, _window, cx| cx.emit(DeleteViewIntent::Close)))))
                }
                DeletePhase::Error(msg) => {
                    div().flex().flex_col().gap_3()
                        .child(div().font_weight(FontWeight::BOLD).text_color(theme.error).child("Error"))
                        .child(div().text_sm().child(msg.clone()))
                        .child(h_flex().justify_end()
                            .child(div().px_3().py_1().rounded_md().bg(theme.primary).cursor_pointer().child("Close")
                                .on_mouse_down(MouseButton::Left, cx.listener(|_, _, _window, cx| cx.emit(DeleteViewIntent::Close)))))
                }
            })
    }
}
