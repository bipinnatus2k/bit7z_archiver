use crate::domain::archive::ArchiveEntry;
use crate::theme::Theme;
use gpui::prelude::FluentBuilder as _;
use gpui::*;

pub struct ExtractDialog {
    pub entries: Vec<ArchiveEntry>,
    pub destination: String,
    pub preserve_paths: bool,
    pub entries_count: usize,
}

#[derive(Debug, Clone)]
pub enum ExtractDialogEvent {
    ExtractRequested {
        destination: std::path::PathBuf,
        preserve_paths: bool,
    },
    Canceled,
}

impl EventEmitter<ExtractDialogEvent> for ExtractDialog {}

impl ExtractDialog {
    pub fn new(entries: Vec<ArchiveEntry>, cx: &mut Context<Self>) -> Entity<Self> {
        let count = entries.len();
        cx.new(|_cx| Self {
            entries,
            destination: String::new(),
            preserve_paths: true,
            entries_count: count,
        })
    }
}

impl Render for ExtractDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let can_extract = !self.destination.is_empty();
        div()
            .flex().flex_col().gap_3().p_4().w(px(480.))
            .child(div().font_weight(FontWeight::BOLD).text_lg().child("Extract"))
            .child(div().text_sm().text_color(cx.global::<Theme>().muted)
                .child(format!("{} entr{} selected", self.entries_count,
                    if self.entries_count == 1 { "y" } else { "ies" })))
            .child(
                div().flex().flex_col().gap_1()
                    .child(div().text_sm().font_weight(FontWeight::BOLD).child("Destination"))
                    // .child(
                    //     div().flex().flex_row().gap_1()
                    //         .child(div().flex_1().px_2().py_1().border_1().border_color(cx.global::<Theme>().border)
                    //             .rounded_md().text_sm().child(if self.destination.is_empty() {
                    //                 "Select destination folder..."
                    //             } else {
                    //                 &self.destination
                    //             }))
                    //         .child(div().px_2().py_1().rounded_md().hover(|mut s| { s.background = Some(cx.global::<Theme>().hover.into()); s })
                    //             .cursor_pointer().child("Browse..."))
                    // )
            )
            .child(
                div().flex().flex_row().gap_1().items_center()
                    .child(if self.preserve_paths { "☑" } else { "☐" })
                    .child("Preserve directory structure")
            )
            .child(
                div().flex().flex_row().justify_end().gap_2().pt_2()
                    .child(div().px_3().py_1().rounded_md().cursor_pointer().child("Cancel")
                        .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _e, _window, cx| cx.emit(ExtractDialogEvent::Canceled))))
                    .child(
                        div().px_3().py_1().rounded_md().cursor_pointer()
                            .bg(if can_extract { cx.global::<Theme>().primary } else { cx.global::<Theme>().muted })
                            .child("Extract")
                            .when(can_extract, |el| {
                                el.on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _e, _window, cx| {
                                    cx.emit(ExtractDialogEvent::ExtractRequested {
                                        destination: std::path::PathBuf::from(&this.destination),
                                        preserve_paths: this.preserve_paths,
                                    });
                                }))
                            })
                    )
            )
    }
}





