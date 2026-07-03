use bit7z_pres_theme::Theme;
use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::progress::Progress;
use gpui_component::{h_flex, v_flex};
use gpui_component::label::Label;

#[derive(Debug, Clone, PartialEq)]
pub enum ChecksumViewIntent {
    Start,
    Cancel,
    Close,
}

impl EventEmitter<ChecksumViewIntent> for ChecksumDialogView {}

pub enum ChecksumPhase {
    Idle { total: usize, algorithm: String },
    Processing { current: u64, total: u64, file: String, results: Vec<(String, u64, String)> },
    Complete { results: Vec<(String, u64, String)> },
    #[allow(dead_code)]
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

    #[allow(dead_code)]
    pub fn set_error(&mut self, msg: &str) {
        self.phase = ChecksumPhase::Error(msg.to_string());
    }
}

impl Render for ChecksumDialogView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        v_flex().p_4().gap_3().size_full()
            .child(match &self.phase {
                ChecksumPhase::Idle { total, algorithm } => {
                    v_flex().gap_3()
                        .child(div().font_weight(FontWeight::BOLD).text_lg().child("Calculate Checksum"))
                        .child(div().text_sm().child(format!("{} entr{} selected", total, if *total == 1 { "y" } else { "ies" })))
                        .child(h_flex().gap_1().items_center()
                            .child(div().text_sm().font_weight(FontWeight::BOLD).child("Algorithm:"))
                            .child(Button::new("algo").outline().label(algorithm.clone())))
                        .child(div().flex_1())
                        .child(h_flex().justify_end().gap_2()
                            .child(Button::new("cancel").outline().label("Cancel").on_click(cx.listener(|_, _, _, cx| cx.emit(ChecksumViewIntent::Cancel))))
                            .child(Button::new("calc").primary().label("Calculate").on_click(cx.listener(|_, _, _, cx| cx.emit(ChecksumViewIntent::Start)))))
                }
                ChecksumPhase::Processing { current, total, file, results } => {
                    let pct = if *total > 0 { (*current as f64 / *total as f64 * 100.0) as f32 } else { 0.0 };
                    v_flex().gap_3()
                        .child(div().font_weight(FontWeight::BOLD).child("Calculating..."))
                        .child(div().text_sm().child(file.clone()))
                        .child(Progress::new("progress").value(pct))
                        .children(results.iter().map(|(name, _, hash)| {
                            div().text_sm().child(format!("{}: {}", name, hash)).into_any_element()
                        }))
                }
                ChecksumPhase::Complete { results } => {
                    v_flex().gap_3()
                        .child(div().font_weight(FontWeight::BOLD).text_lg().child("Checksum Results"))
                        .children(results.iter().map(|(name, _a, hash)| {
                            Label::new(name).secondary(hash).into_element()
                        }))
                        .child(div().flex_1())
                        .child(h_flex().justify_end()
                            .child(Button::new("close").primary().label("Close").on_click(cx.listener(|_a, click_event, window, cx| {
                                window.remove_window();
                            }))))
                }
                ChecksumPhase::Error(msg) => {
                    v_flex().gap_3()
                        .child(div().font_weight(FontWeight::BOLD).text_color(theme.error).child("Error"))
                        .child(div().text_sm().child(msg.clone()))
                        .child(h_flex().justify_end()
                            .child(Button::new("close").primary().label("Close").on_click(cx.listener(|_, _, _, cx| cx.emit(ChecksumViewIntent::Close)))))
                }
            })
    }
}
