use crate::domain::archive::{TestFailure, TestResult};
use crate::theme::Theme;
use gpui::prelude::FluentBuilder as _;
use gpui::*;

pub struct TestResultsDialog {
    pub result: Option<TestResult>,
    pub show_failed: bool,
}

#[derive(Debug, Clone)]
pub enum TestResultsEvent {
    Close,
}

impl EventEmitter<TestResultsEvent> for TestResultsDialog {}

impl TestResultsDialog {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            result: None,
            show_failed: false,
        }
    }

    pub fn set_result(&mut self, result: TestResult, cx: &mut Context<Self>) {
        self.result = Some(result);
        cx.notify();
    }
}

impl Render for TestResultsDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>().clone();

        div().flex().flex_col().gap_3().p_4().w(px(420.))
            .child(div().font_weight(FontWeight::BOLD).text_lg().child("Test Results"))
            .when_some(self.result.as_ref(), |el, result| {
                let all_pass = result.failed.is_empty();
                el.child(
                    div().text_lg().font_weight(FontWeight::BOLD)
                        .text_color(if all_pass { theme.primary } else { theme.error })
                        .child(format!("{} passed, {} failed", result.passed, result.failed.len()))
                )
                .when(!all_pass, |el| {
                    el.child(
                        div().flex().flex_col().gap_1()
                            .child(
                                div().px_2().py_1().cursor_pointer().flex().flex_row().gap_1()
                                    .hover(|mut s| { s.background = Some(theme.hover.into()); s })
                                    .child(if self.show_failed { "\u{25BC}" } else { "\u{25B6}" })
                                    .child(format!("Failed entries ({})", result.failed.len()))
                                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _e, _window, cx| {
                                        this.show_failed = !this.show_failed;
                                        cx.notify();
                                    }))
                            )
                            .when(self.show_failed, |el| {
                                el.child(
                                    div().flex().flex_col().gap_1().pl_4()
                                        .children(result.failed.iter().map(|f| {
                                            failed_entry(f, &theme).into_any_element()
                                        }).collect::<Vec<_>>())
                                )
                            })
                    )
                })
            })
            .when(self.result.is_none(), |el| {
                el.child(div().text_sm().text_color(theme.muted).child("Loading results..."))
            })
            .child(
                div().flex().flex_row().justify_end().gap_2().pt_2()
                    .child(
                        div().px_3().py_1().rounded_md().bg(theme.primary).cursor_pointer().child("Close")
                            .on_mouse_down(MouseButton::Left, cx.listener(|_this, _e, _window, cx| {
                                cx.emit(TestResultsEvent::Close);
                            }))
                    )
            )
    }
}

fn failed_entry(f: &TestFailure, theme: &Theme) -> impl IntoElement {
    let reason = match &f.reason {
        crate::domain::archive::TestFailureReason::CrcMismatch { expected, actual } => {
            format!("CRC mismatch: expected {:08X}, got {:08X}", expected, actual)
        }
        crate::domain::archive::TestFailureReason::ReadError(msg) => {
            format!("Read error: {}", msg)
        }
        crate::domain::archive::TestFailureReason::UnsupportedOperation => {
            "Unsupported operation".to_string()
        }
    };
    div().flex().flex_col().gap_0().py_1()
        .child(div().text_sm().font_weight(FontWeight::MEDIUM).child(format!("#{} {}", f.index, f.path)))
        .child(div().text_xs().text_color(theme.error).child(reason))
}
