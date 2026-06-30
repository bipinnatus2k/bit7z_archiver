use crate::domain::archive::*;
use crate::theme::Theme;
use crossbeam::channel::{unbounded, Receiver, Sender};
use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::checkbox::Checkbox;
use gpui_component::input::{Input, InputState};
use gpui_component::label::Label;
use gpui_component::select::{Select, SelectItem, SelectState};
use gpui_component::{v_flex, Icon, IconName, IndexPath, StyledExt};

pub struct ExtractDialog {
    pub entries: Vec<ArchiveEntry>,
    pub destination: String,
    pub preserve_paths: bool,
    pub entries_count: usize,
    pub overwrite_mode: OverwriteMode,
    // pub show_overwrite_dropdown: bool,
    pub keep_broken: bool,
    result_tx: Option<Sender<ExtractDialogEvent>>,
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

impl SelectItem for OverwriteMode {
    type Value = i32;

    fn title(&self) -> SharedString {
        self.label().into()
    }

    fn display_title(&self) -> Option<gpui::AnyElement> {
        Some(format!("{} ({})", self.label(), self.index()).into_any_element())
    }

    fn value(&self) -> &Self::Value {
        // SAFETY: OverwriteMode has exactly 4 variants (0..3), matching the array bounds.
        const VALUES: [i32; 4] = [0, 1, 2, 3];
        &VALUES[self.index() as usize]
    }

    fn matches(&self, query: &str) -> bool {
        self.label().to_lowercase().contains(&query.to_lowercase())
    }
}

impl ExtractDialog {
    fn new(entries: Vec<ArchiveEntry>, result_tx: Sender<ExtractDialogEvent>) -> Self {
        let count = entries.len();
        Self {
            entries,
            destination: String::new(),
            preserve_paths: true,
            entries_count: count,
            overwrite_mode: OverwriteMode::Ask,
            // show_overwrite_dropdown: false,
            keep_broken: false,
            result_tx: Some(result_tx),
        }
    }

    pub fn open(entries: Vec<ArchiveEntry>, cx: &mut AsyncApp) -> Receiver<ExtractDialogEvent> {
        let (tx, rx) = unbounded::<ExtractDialogEvent>();
        cx.spawn(async move |cx| {
            let _ = cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                        point(px(100.), px(100.)),
                        size(px(560.), px(400.)),
                    ))),
                    window_background: WindowBackgroundAppearance::Opaque,
                    window_decorations: Some(WindowDecorations::Client),
                    ..Default::default()
                },
                move |window, cx| {
                    let dialog = cx.new(|_cx| ExtractDialog::new(entries, tx));
                    cx.new(|cx| gpui_component::Root::new(dialog, window, cx))
                },
            );
        })
        .detach();
        rx
    }

    fn finish(&mut self, event: ExtractDialogEvent, window: &mut Window) {
        if let Some(tx) = self.result_tx.take() {
            let _ = tx.send(event);
        }
        window.remove_window();
    }
}

impl Render for ExtractDialog {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let can_extract = !self.destination.is_empty();
        let theme = cx.global::<Theme>().clone();
        let input = cx.new(|cx2| {
            InputState::new(window, cx2)
                .placeholder("Select destination folder...")
                .default_value(&self.destination)
        });
        let state = cx.new(|cx| {
            SelectState::new(OverwriteMode::all(), Some(IndexPath::default()), window, cx)
        });

        v_flex()
            .gap_3()
            .p_4()
            .w_full()
            .child(Label::new("Extract").font_black())
            .child(div().text_sm().text_color(theme.muted).child(format!(
                "{} entr{} selected",
                self.entries_count,
                if self.entries_count == 1 { "y" } else { "ies" }
            )))
            // Destination with Browse button
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::BOLD)
                    .child("Destination"),
            )
            .child(
                Input::new(&input).suffix(
                    Button::new("BrowsePath")
                        .icon(Icon::new(IconName::FolderOpen))
                        .ghost()
                        .cursor_pointer()
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |this, _e, window2, cx| {
                                if let Some(path) = crate::adapters::platform::pick_folder() {
                                    this.destination = path.to_string_lossy().to_string();
                                    input.update(cx, |state, cx2| {
                                        state.set_value(path.to_string_lossy(), window2, cx2)
                                    });
                                    cx.notify();
                                }
                            }),
                        ),
                ),
            )
            // Preserve directory structure
            .child(
                Checkbox::new("dirStructure")
                    .checked(self.preserve_paths)
                    .label("Preserve directory structure")
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _e, _window, cx| {
                        this.preserve_paths = !this.preserve_paths;
                        cx.notify();
                    })),
            )
            // Overwrite mode dropdown
            .child(
                Select::new(&state)
                    .cleanable(false)
                    .title_prefix("Overwrite mode: "),
            )
            // Keep broken files
            .child(
                Checkbox::new("keepBroken")
                    .checked(self.keep_broken)
                    .label("Keep broken files")
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _e, _window, cx| {
                        this.keep_broken = !this.keep_broken;
                        cx.notify();
                    })),
            )
            // Buttons
            .child(
                div()
                    .flex()
                    .flex_row()
                    .justify_end()
                    .gap_2()
                    .pt_2()
                    .child(
                        Button::new("cancel")
                            .cursor_pointer()
                            .child("Cancel")
                            .on_click(cx.listener(|this, _e, window, _cx| {
                                this.finish(ExtractDialogEvent::Canceled, window);
                            })),
                    )
                    .child(
                        Button::new("extract")
                            .cursor_pointer()
                            .primary()
                            .child("Extract")
                            .when(can_extract, |el| {
                                el.on_click(cx.listener(|this, _e, window, _cx| {
                                    this.finish(
                                        ExtractDialogEvent::ExtractRequested {
                                            destination: std::path::PathBuf::from(
                                                &this.destination,
                                            ),
                                            preserve_paths: this.preserve_paths,
                                            overwrite_mode: this.overwrite_mode,
                                            keep_broken: this.keep_broken,
                                        },
                                        window,
                                    );
                                }))
                            }),
                    ),
            )
    }
}
