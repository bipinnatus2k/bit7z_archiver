use crate::domain::archive::TestFailure;
use crate::theme::Theme;
use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::progress::Progress;
use gpui_component::scroll::ScrollableElement;
use gpui_component::{h_flex, v_flex};

#[derive(Debug, Clone, PartialEq)]
pub enum TestViewIntent {
    Start,
    Cancel,
    Close,
}

impl EventEmitter<TestViewIntent> for TestDialogView {}

pub enum TestPhase {
    Idle { total: usize },
    Processing { current: u64, total: u64, file: String },
    Complete { passed: usize, failed: Vec<TestFailure> },
    Error(String),
}

pub struct TestDialogView {
    phase: TestPhase,
}

impl TestDialogView {
    pub fn new(total: usize) -> Self {
        Self { phase: TestPhase::Idle { total } }
    }

    pub fn set_processing(&mut self, current: u64, total: u64, file: &str) {
        self.phase = TestPhase::Processing { current, total, file: file.to_string() };
    }

    pub fn set_complete(&mut self, passed: usize, failed: Vec<TestFailure>) {
        self.phase = TestPhase::Complete { passed, failed };
    }

    pub fn set_error(&mut self, msg: &str) {
        self.phase = TestPhase::Error(msg.to_string());
    }
}

impl Render for TestDialogView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        v_flex().gap_3().p_4().w(px(520.)).h(px(400.))
            .child(match &self.phase {
                TestPhase::Idle { total } => {
                    v_flex().gap_3()
                        .child(div().font_weight(FontWeight::BOLD).text_lg().child("Test Archive"))
                        .child(div().text_sm().child(format!("Test {} entr{} for integrity.", total, if *total == 1 { "y" } else { "ies" })))
                        .child(div().flex_1())
                        .child(h_flex().justify_end().gap_2()
                            .child(Button::new("cancel").label("Cancel").on_click(cx.listener(|_, _, _window, cx| cx.emit(TestViewIntent::Cancel))))
                            .child(Button::new("start").label("Start Test").primary().on_click(cx.listener(|_, _, _window, cx| cx.emit(TestViewIntent::Start)))))
                        .into_any_element()
                }
                TestPhase::Processing { current, total, file } => {
                    let pct = if *total > 0 { (*current as f64 / *total as f64 * 100.0) as f32 } else { 0.0 };
                    v_flex().gap_3()
                        .child(div().font_weight(FontWeight::BOLD).child("Testing..."))
                        .child(div().text_sm().child(file.clone()))
                        .child(
                            v_flex().gap_1()
                                .child(h_flex().justify_between()
                                    .child(div().text_sm().text_color(theme.muted).child(file.clone()))
                                    .child(div().text_sm().text_color(theme.muted).child(format!("{}/{}", current, total))))
                                .child(Progress::new("test-progress").value(pct))
                        )
                        .into_any_element()
                }
                TestPhase::Complete { passed, failed } => {
                    let total = passed + failed.len();
                    v_flex().gap_3().overflow_y_scrollbar()
                        .child(div().font_weight(FontWeight::BOLD).text_lg().child(if failed.len() == 0 { "All Tests Passed".to_string() } else { format!("{}/{} Failed", failed.len(), total) }))
                        .child(div().text_sm().child(if failed.len() == 0 { format!("All {} entr{} passed integrity check.", total, if total == 1 { "y" } else { "ies" }) } else { format!("{} passed, {} failed.", passed, failed.len()) }))
                        .children(failed.iter().map(|f| {
                            h_flex().gap_2().px_2().py_1().text_sm()
                                .child(div().text_color(theme.error).child("\u{2716}"))
                                .child(div().child(format!("{}", f.entry_path)))
                                .child(div().text_color(theme.muted).child(f.error.clone()))
                                .into_any_element()
                        }))
                        .child(div().flex_1())
                        .child(h_flex().justify_end()
                            .child(Button::new("close").label("Close").primary().on_click(cx.listener(|_, _, _window, cx| cx.emit(TestViewIntent::Close)))))
                        .into_any_element()
                }
                TestPhase::Error(msg) => {
                    v_flex().gap_3()
                        .child(div().font_weight(FontWeight::BOLD).text_color(theme.error).child("Error"))
                        .child(div().text_sm().child(msg.clone()))
                        .child(h_flex().justify_end()
                            .child(Button::new("close").label("Close").primary().on_click(cx.listener(|_, _, _window, cx| cx.emit(TestViewIntent::Close)))))
                        .into_any_element()
                }
            })
    }
}
