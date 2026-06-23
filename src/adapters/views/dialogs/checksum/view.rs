use crate::domain::repository::ProgressUpdate;
use crate::theme::Theme;
use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::h_flex;

#[derive(Debug, Clone, PartialEq)]
pub enum ChecksumViewIntent {
    SelectAlgorithm(String),
    Start,
    Cancel,
    Close,
}

impl EventEmitter<ChecksumViewIntent> for ChecksumDialogView {}

pub enum ChecksumPhase {
    Idle { total: usize, algorithm: String },
    Processing { current: u64, total: u64, file: String, results: Vec<(String, u64, String)> },
    Complete { results: Vec<(String, u64, String)> },
    Error(String),
}

pub struct ChecksumDialogView {
    phase: ChecksumPhase,
}

impl ChecksumDialogView {
    pub fn new(total: usize) -> Self {
        Self { phase: ChecksumPhase::Idle { total, algorithm: "CRC32".to_string() } }
    }

    pub fn set_processing(&mut self, current: u64, total: u64, file: &str, results: Vec<(String, u64, String)>) {
        self.phase = ChecksumPhase::Processing { current, total, file: file.to_string(), results };
    }

    pub fn set_complete(&mut self, results: Vec<(String, u64, String)>) {
        self.phase = ChecksumPhase::Complete { results };
    }

    pub fn set_error(&mut self, msg: &str) {
        self.phase = ChecksumPhase::Error(msg.to_string());
    }
}

impl Render for ChecksumDialogView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div().flex().flex_col().gap_3().p_4().w(px(520.)).h(px(420.))
            .child(match &self.phase {
                ChecksumPhase::Idle { total, algorithm } => {
                    div().flex().flex_col().gap_3()
                        .child(div().font_weight(FontWeight::BOLD).text_lg().child("Calculate Checksum"))
                        .child(div().text_sm().child(format!("{} entr{} selected", total, if *total == 1 { "y" } else { "ies" })))
                        .child(div().flex().flex_row().gap_1().items_center()
                            .child(div().text_sm().font_weight(FontWeight::BOLD).child("Algorithm:"))
                            .child(div().px_2().py_1().border_1().border_color(theme.border).rounded_md().cursor_pointer().child(algorithm.clone())))
                        .child(div().flex_1())
                        .child(h_flex().justify_end().gap_2()
                            .child(div().px_3().py_1().rounded_md().cursor_pointer().child("Cancel")
                                .on_mouse_down(MouseButton::Left, cx.listener(|_, _, _window, cx| cx.emit(ChecksumViewIntent::Cancel))))
                            .child(div().px_3().py_1().rounded_md().bg(theme.primary).cursor_pointer().child("Calculate")
                                .on_mouse_down(MouseButton::Left, cx.listener(|_, _, _window, cx| cx.emit(ChecksumViewIntent::Start)))))
                }
                ChecksumPhase::Processing { current, total, file, results } => {
                    let pct = if *total > 0 { (*current as f64 / *total as f64 * 100.0) as u32 } else { 0 };
                    div().flex().flex_col().gap_3()
                        .child(div().font_weight(FontWeight::BOLD).child("Calculating..."))
                        .child(div().text_sm().child(file.clone()))
                        .child(div().flex().flex_row().gap_2().items_center()
                            .child(div().flex_1().h(px(20.)).bg(theme.surface).rounded_md().overflow_hidden()
                                .child(div().h_full().bg(theme.primary).rounded_md().w(px(pct as f32 * 4.0))))
                            .child(div().text_sm().child(format!("{}/{}", current, total))))
                        .children(results.iter().map(|(name, _, hash)| {
                            div().text_sm().child(format!("{}: {}", name, hash)).into_any_element()
                        }))
                }
                ChecksumPhase::Complete { results } => {
                    div().flex().flex_col().gap_3()
                        .child(div().font_weight(FontWeight::BOLD).text_lg().child("Checksum Results"))
                        .children(results.iter().map(|(name, _, hash)| {
                            div().flex().flex_row().gap_2().px_1().py_1().text_sm()
                                .child(div().font_weight(FontWeight::BOLD).child(name.clone()))
                                .child(div().text_color(theme.muted).child(": "))
                                .child(div().child(hash.clone()))
                                .into_any_element()
                        }))
                        .child(div().flex_1())
                        .child(h_flex().justify_end()
                            .child(div().px_3().py_1().rounded_md().bg(theme.primary).cursor_pointer().child("Close")
                                .on_mouse_down(MouseButton::Left, cx.listener(|_, _, _window, cx| cx.emit(ChecksumViewIntent::Close)))))
                }
                ChecksumPhase::Error(msg) => {
                    div().flex().flex_col().gap_3()
                        .child(div().font_weight(FontWeight::BOLD).text_color(theme.error).child("Error"))
                        .child(div().text_sm().child(msg.clone()))
                        .child(h_flex().justify_end()
                            .child(div().px_3().py_1().rounded_md().bg(theme.primary).cursor_pointer().child("Close")
                                .on_mouse_down(MouseButton::Left, cx.listener(|_, _, _window, cx| cx.emit(ChecksumViewIntent::Close)))))
                }
            })
    }
}
