use bit7z_domain::archive::{TestFailure, TestResult};
use bit7z_pres_components::window_dialog::{
    CloseAction, DialogContent, DialogFooter, DialogHeader, DialogTitle, WindowDialogOptions,
    open_window_dialog_async,
};
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::{h_flex, v_flex};

pub struct TestResultsDialog {
    result: TestResult,
    show_failed: bool,
}

impl TestResultsDialog {
    pub fn open(cx: &mut AsyncApp, result: TestResult) {
        open_window_dialog_async(
            cx,
            WindowDialogOptions {
                title: "Test Results".into(),
                width: px(480.),
                height: Some(px(400.)),
                min_width: None,
                min_height: None,
                kind: WindowKind::Dialog,
                close_action: CloseAction::RemoveWindow,
                window_decorations: Some(WindowDecorations::Client),
                window_background: WindowBackgroundAppearance::Opaque,
            },
            |_window, cx| {
                cx.new(move |_| Self {
                    result,
                    show_failed: false,
                })
            },
        );
    }
}

impl Render for TestResultsDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let all_pass = self.result.failed.is_empty();
        let h = cx.entity();

        v_flex()
            .size_full()
            .gap(px(12.))
            .child(DialogHeader::new().child(DialogTitle::new().child("Test Results")))
            .child(
                DialogContent::new().child(
                    v_flex()
                        .h_full()
                        .gap_3()
                        .child(div().text_lg().font_weight(FontWeight::BOLD).child(format!(
                            "{} passed, {} failed",
                            self.result.passed,
                            self.result.failed.len()
                        )))
                        .when(!all_pass, |el| {
                            el.child(
                                v_flex()
                                    .gap_1()
                                    .child(
                                        Button::new("toggle-failed")
                                            .label(if self.show_failed {
                                                "\u{25bc} Failed entries"
                                            } else {
                                                "\u{25b6} Failed entries"
                                            })
                                            .ghost()
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.show_failed = !this.show_failed;
                                                cx.notify();
                                            })),
                                    )
                                    .when(self.show_failed, |el| {
                                        el.child(
                                            v_flex().gap_1().pl_4().children(
                                                self.result
                                                    .failed
                                                    .iter()
                                                    .map(|f| failed_entry(f).into_any_element())
                                                    .collect::<Vec<_>>(),
                                            ),
                                        )
                                    }),
                            )
                        }),
                ),
            )
            .child(
                DialogFooter::new().justify_end().child(
                    Button::new("close")
                        .label("Close")
                        .primary()
                        .on_click(move |_, window, _| {
                            window.remove_window();
                        }),
                ),
            )
    }
}

fn failed_entry(f: &TestFailure) -> impl IntoElement {
    let reason = match &f.reason {
        bit7z_domain::archive::TestFailureReason::CrcMismatch { expected, actual } => {
            format!(
                "CRC mismatch: expected {:08X}, got {:08X}",
                expected, actual
            )
        }
        bit7z_domain::archive::TestFailureReason::ReadError(msg) => {
            format!("Read error: {}", msg)
        }
        bit7z_domain::archive::TestFailureReason::UnsupportedOperation => {
            "Unsupported operation".to_string()
        }
    };
    v_flex()
        .gap_0()
        .py_1()
        .child(
            div()
                .text_sm()
                .font_weight(FontWeight::MEDIUM)
                .child(format!("#{} {}", f.index, f.path)),
        )
        .child(div().text_xs().child(reason))
}
