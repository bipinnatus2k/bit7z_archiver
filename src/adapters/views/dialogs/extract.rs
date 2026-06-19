use crate::domain::archive::ArchiveEntry;
use crate::theme::Theme;
use gpui::prelude::FluentBuilder as _;
use gpui::*;

pub struct ExtractDialog {
    pub entries: Vec<ArchiveEntry>,
    pub destination: String,
    pub preserve_paths: bool,
    pub entries_count: usize,
    pub overwrite_mode: OverwriteMode,
    pub show_overwrite_dropdown: bool,
    pub keep_broken: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OverwriteMode {
    Ask,
    Overwrite,
    Skip,
    RenameExtracted,
}

impl OverwriteMode {
    fn label(&self) -> &'static str {
        match self {
            OverwriteMode::Ask => "Ask",
            OverwriteMode::Overwrite => "Overwrite",
            OverwriteMode::Skip => "Skip",
            OverwriteMode::RenameExtracted => "Rename extracted",
        }
    }

    fn all() -> Vec<OverwriteMode> {
        vec![OverwriteMode::Ask, OverwriteMode::Overwrite, OverwriteMode::Skip, OverwriteMode::RenameExtracted]
    }
}

#[derive(Debug, Clone)]
pub enum ExtractDialogEvent {
    ExtractRequested {
        destination: std::path::PathBuf,
        preserve_paths: bool,
        overwrite_mode: OverwriteMode,
        keep_broken: bool,
    },
    Canceled,
}

impl EventEmitter<ExtractDialogEvent> for ExtractDialog {}

impl ExtractDialog {
    pub fn new(entries: Vec<ArchiveEntry>) -> Self {
        let count = entries.len();
        Self {
            entries,
            destination: String::new(),
            preserve_paths: true,
            entries_count: count,
            overwrite_mode: OverwriteMode::Ask,
            show_overwrite_dropdown: false,
            keep_broken: false,
        }
    }
}

impl Render for ExtractDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let can_extract = !self.destination.is_empty();
        let theme = cx.global::<Theme>().clone();
        div()
            .flex().flex_col().gap_3().p_4().w(px(480.))
            .child(div().font_weight(FontWeight::BOLD).text_lg().child("Extract"))
            .child(div().text_sm().text_color(theme.muted)
                .child(format!("{} entr{} selected", self.entries_count,
                    if self.entries_count == 1 { "y" } else { "ies" })))
            // Destination with Browse button
            .child(
                div().flex().flex_col().gap_1()
                    .child(div().text_sm().font_weight(FontWeight::BOLD).child("Destination"))
                    .child(
                        div().flex().flex_row().gap_1()
                            .child(
                                div().flex_1().px_2().py_1().border_1().border_color(theme.border)
                                    .rounded_md().text_sm().child(if self.destination.is_empty() {
                                        "Select destination folder...".to_string()
                                    } else {
                                        self.destination.clone()
                                    })
                            )
                            .child(
                                div().px_2().py_1().rounded_md()
                                    .hover(|mut s| { s.background = Some(theme.hover.into()); s })
                                    .cursor_pointer().child("Browse...")
                                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _e, _window, cx| {
                                        if let Some(path) = crate::adapters::platform::pick_folder() {
                                            this.destination = path.to_string_lossy().to_string();
                                            cx.notify();
                                        }
                                    }))
                            )
                    )
            )
            // Preserve directory structure
            .child(
                div().flex().flex_row().gap_1().items_center()
                    .child(if self.preserve_paths { "\u{2611}" } else { "\u{2610}" })
                    .child("Preserve directory structure")
                    .cursor_pointer()
                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _e, _window, cx| {
                        this.preserve_paths = !this.preserve_paths;
                        cx.notify();
                    }))
            )
            // Overwrite mode dropdown
            .child(
                div().flex().flex_col().gap_1()
                    .child(div().text_sm().font_weight(FontWeight::BOLD).child("Overwrite mode"))
                    .child(
                        div().flex().flex_col().gap_1()
                            .child(
                                div().px_2().py_1().border_1().border_color(theme.border).rounded_md().cursor_pointer()
                                    .w(px(180.))
                                    .flex().flex_row().justify_between()
                                    .child(self.overwrite_mode.label())
                                    .child(if self.show_overwrite_dropdown { "\u{25B2}" } else { "\u{25BC}" })
                                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _e, _window, cx| {
                                        this.show_overwrite_dropdown = !this.show_overwrite_dropdown;
                                        cx.notify();
                                    }))
                            )
                            .when(self.show_overwrite_dropdown, |el| {
                                el.child(
                                    div().border_1().border_color(theme.border).rounded_md().flex().flex_col().w(px(180.))
                                        .children(OverwriteMode::all().into_iter().map(|m| {
                                            let is_current = m == self.overwrite_mode;
                                            div().px_2().py_1().cursor_pointer()
                                                .bg(if is_current { theme.selection } else { hsla(0., 0., 0., 0.) })
                                                .hover(|mut s| { s.background = Some(theme.hover.into()); s })
                                                .child(m.label())
                                                .on_mouse_down(MouseButton::Left, cx.listener(move |this, _e, _window, cx| {
                                                    this.overwrite_mode = m;
                                                    this.show_overwrite_dropdown = false;
                                                    cx.notify();
                                                }))
                                                .into_any_element()
                                        }).collect::<Vec<_>>())
                                )
                            })
                    )
            )
            // Keep broken files
            .child(
                div().flex().flex_row().gap_1().items_center()
                    .child(if self.keep_broken { "\u{2611}" } else { "\u{2610}" })
                    .child("Keep broken files")
                    .cursor_pointer()
                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _e, _window, cx| {
                        this.keep_broken = !this.keep_broken;
                        cx.notify();
                    }))
            )
            // Buttons
            .child(
                div().flex().flex_row().justify_end().gap_2().pt_2()
                    .child(div().px_3().py_1().rounded_md().cursor_pointer().child("Cancel")
                        .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _e, _window, cx| cx.emit(ExtractDialogEvent::Canceled))))
                    .child(
                        div().px_3().py_1().rounded_md().cursor_pointer()
                            .bg(if can_extract { theme.primary } else { theme.muted })
                            .child("Extract")
                            .when(can_extract, |el| {
                                el.on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _e, _window, cx| {
                                    cx.emit(ExtractDialogEvent::ExtractRequested {
                                        destination: std::path::PathBuf::from(&this.destination),
                                        preserve_paths: this.preserve_paths,
                                        overwrite_mode: this.overwrite_mode,
                                        keep_broken: this.keep_broken,
                                    });
                                }))
                            })
                    )
            )
    }
}
