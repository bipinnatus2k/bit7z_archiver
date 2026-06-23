use crate::domain::archive::TestFailure;
use crate::theme::Theme;
use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::h_flex;

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

        div().flex().flex_col().gap_3().p_4().w(px(520.)).h(px(400.))
            .child(match &self.phase {
                TestPhase::Idle { total } => {
                    div().flex().flex_col().gap_3()
                        .child(div().font_weight(FontWeight::BOLD).text_lg().child("Test Archive"))
                        .child(div().text_sm().child(format!("Test {} entr{} for integrity.", total, if *total == 1 { "y" } else { "ies" })))
                        .child(div().flex_1())
                        .child(h_flex().justify_end().gap_2()
                            .child(div().px_3().py_1().rounded_md().cursor_pointer().child("Cancel")
                                .on_mouse_down(MouseButton::Left, cx.listener(|_, _, cx| cx.emit(TestViewIntent::Cancel))))
                            .child(div().px_3().py_1().rounded_md().bg(theme.primary).cursor_pointer().child("Start Test")
                                .on_mouse_down(MouseButton::Left, cx.listener(|_, _, cx| cx.emit(TestViewIntent::Start)))))
                }
                TestPhase::Processing { current, total, file } => {
                    let pct = if *total > 0 { (*current as f64 / *total as f64 * 100.0) as u32 } else { 0 };
                    div().flex().flex_col().gap_3()
                        .child(div().font_weight(FontWeight::BOLD).child("Testing..."))
                        .child(div().text_sm().child(file.clone()))
                        .child(div().flex().flex_row().gap_2().items_center()
                            .child(div().flex_1().h(px(20.)).bg(theme.surface).rounded_md().overflow_hidden()
                                .child(div().h_full().bg(theme.primary).rounded_md().w(px(pct as f32 * 4.0))))
                            .child(div().text_sm().child(format!("{}/{}", current, total))))
                }
                TestPhase::Complete { passed, failed } => {
                    let total = passed + failed.len();
                    div().flex().flex_col().gap_3().overflow_y_scroll()
                        .child(match failed.len() {
                            0 => div().font_weight(FontWeight::BOLD).text_lg().child("All Tests Passed").into_any_element(),
                            _ => div().font_weight(FontWeight::BOLD).text_lg().text_color(theme.error).child(format!("{}/{} Failed", failed.len(), total)).into_any_element(),
                        })
                        .child(div().text_sm().child(match failed.len() {
                            0 => format!("All {} entr{} passed integrity check.", total, if total == 1 { "y" } else { "ies" }),
                            n => format!("{} passed, {} failed.", passed, n),
                        }))
                        .children(failed.iter().map(|f| {
                            div().flex().flex_row().gap_2().px_2().py_1().text_sm()
                                .child(div().text_color(theme.danger).child("\u{2716}"))
                                .child(div().child(format!("{}", f.entry_path)))
                                .child(div().text_color(theme.muted).child(&f.error))
                                .into_any_element()
                        }))
                        .child(div().flex_1())
                        .child(h_flex().justify_end()
                            .child(div().px_3().py_1().rounded_md().bg(theme.primary).cursor_pointer().child("Close")
                                .on_mouse_down(MouseButton::Left, cx.listener(|_, _, cx| cx.emit(TestViewIntent::Close)))))
                }
                TestPhase::Error(msg) => {
                    div().flex().flex_col().gap_3()
                        .child(div().font_weight(FontWeight::BOLD).text_color(theme.error).child("Error"))
                        .child(div().text_sm().child(msg.clone()))
                        .child(h_flex().justify_end()
                            .child(div().px_3().py_1().rounded_md().bg(theme.primary).cursor_pointer().child("Close")
                                .on_mouse_down(MouseButton::Left, cx.listener(|_, _, cx| cx.emit(TestViewIntent::Close)))))
                }
            })
    }
}
