use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::progress::Progress;
use gpui_component::{h_flex, v_flex};
use bit7z_pres_components::window_dialog::{
    DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle,
};

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
}

impl Render for DeleteDialogView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let handle = cx.entity();

        v_flex()
            .size_full()
            .gap(px(12.))
            .child(
                DialogHeader::new()
                    .child(DialogTitle::new().child("Delete Entries")),
            )
            .child(
                DialogContent::new().child(match &self.phase {
                    DeletePhase::Idle { count } => v_flex()
                        .h_full()
                        .gap_3()
                        .child(DialogDescription::new().child(format!(
                            "Are you sure you want to delete {} entr{}?",
                            count,
                            if *count == 1 { "y" } else { "ies" }
                        )))
                        .child(
                            DialogDescription::new()
                                .child("This action cannot be undone."),
                        )
                        .into_any_element(),
                    DeletePhase::Processing { current, total, message } => {
                        let pct = if *total > 0 {
                            (*current as f64 / *total as f64 * 100.0) as f32
                        } else {
                            0.0
                        };
                        v_flex()
                            .h_full()
                            .gap_2()
                            .child(div().text_sm().child(message.clone()))
                            .child(Progress::new("delete-progress").value(pct))
                            .child(div().text_sm().child(format!("{}/{}", current, total)))
                            .into_any_element()
                    }
                    DeletePhase::Complete => v_flex()
                        .h_full()
                        .gap_3()
                        .child(
                            DialogDescription::new()
                                .child("Selected entries have been deleted."),
                        )
                        .into_any_element(),
                    DeletePhase::Error(msg) => v_flex()
                        .h_full()
                        .gap_3()
                        .child(div().text_sm().child(msg.clone()))
                        .into_any_element(),
                }),
            )
            .child(DialogFooter::new().justify_end().gap_2().child(match &self.phase {
                DeletePhase::Idle { .. } => h_flex()
                    .gap_2()
                    .child(
                        Button::new("cancel")
                            .label("Cancel")
                            .on_click({
                                let h = handle.clone();
                                move |_, _, cx| h.update(cx, |_, cx| cx.emit(DeleteViewIntent::Cancel))
                            }),
                    )
                    .child(
                        Button::new("delete")
                            .label("Delete")
                            .danger()
                            .on_click({
                                let h = handle.clone();
                                move |_, _, cx| h.update(cx, |_, cx| cx.emit(DeleteViewIntent::Confirm))
                            }),
                    )
                    .into_any_element(),
                DeletePhase::Processing { .. } => Button::new("cancel-processing")
                    .label("Cancel")
                    .on_click({
                        let h = handle.clone();
                        move |_, _, cx| h.update(cx, |_, cx| cx.emit(DeleteViewIntent::Cancel))
                    })
                    .into_any_element(),
                DeletePhase::Complete | DeletePhase::Error(_) => Button::new("close")
                    .label("Close")
                    .primary()
                    .on_click({
                        let h = handle.clone();
                        move |_, _, cx| h.update(cx, |_, cx| cx.emit(DeleteViewIntent::Close))
                    })
                    .into_any_element(),
            }))
    }
}
