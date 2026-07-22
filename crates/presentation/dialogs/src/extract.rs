use bit7z_domain::archive::*;
use bit7z_pres_components::window_dialog::{
    CloseAction, DialogContent, DialogFooter, DialogHeader, DialogTitle, WindowDialogOptions,
    open_window_dialog_async,
};
use crossbeam_channel::{Receiver, Sender, unbounded};
use gpui::*;
use gpui_component::Disableable;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::checkbox::Checkbox;
use gpui_component::input::{Input, InputState};
use gpui_component::select::{Select, SelectItem, SelectState};
use gpui_component::v_flex;
use gpui_component::{Icon, IconName, IndexPath};
use std::sync::{Arc, Mutex};

type SharedSender<T> = Arc<Mutex<Option<Sender<T>>>>;

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

impl EventEmitter<ExtractDialogEvent> for ExtractContent {}

#[derive(Debug, Clone)]
struct OverwriteSelect {
    overwrite_mode: OverwriteMode,
}

impl SelectItem for OverwriteSelect {
    type Value = OverwriteMode;

    fn title(&self) -> SharedString {
        self.overwrite_mode.label().into()
    }

    fn value(&self) -> &Self::Value {
        &self.overwrite_mode
    }
}

fn selectable_overwrite() -> Vec<OverwriteSelect> {
    vec![
        OverwriteSelect {
            overwrite_mode: OverwriteMode::Ask,
        },
        OverwriteSelect {
            overwrite_mode: OverwriteMode::Overwrite,
        },
        OverwriteSelect {
            overwrite_mode: OverwriteMode::Skip,
        },
        OverwriteSelect {
            overwrite_mode: OverwriteMode::RenameExtracted,
        },
    ]
}

struct ExtractContent {
    entries: Vec<ArchiveEntry>,
    destination: String,
    preserve_paths: bool,
    entries_count: usize,
    overwrite_mode: Entity<SelectState<Vec<OverwriteSelect>>>,
    keep_broken: bool,
    result_tx: SharedSender<ExtractDialogEvent>,
}

impl ExtractContent {
    fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        entries: Vec<ArchiveEntry>,
        event_tx: SharedSender<ExtractDialogEvent>,
    ) -> Self {
        let state = cx.new(|cx| {
            SelectState::new(
                selectable_overwrite(),
                Some(IndexPath::default()),
                window,
                cx,
            )
        });
        let count = entries.len();
        Self {
            entries,
            destination: String::new(),
            preserve_paths: true,
            entries_count: count,
            overwrite_mode: state,
            keep_broken: false,
            result_tx: event_tx,
        }
    }

    fn emit(&self, event: ExtractDialogEvent, window: &mut Window) {
        if let Ok(guard) = self.result_tx.lock() {
            if let Some(ref tx) = *guard {
                let _ = tx.send(event);
            }
        }
        window.remove_window();
    }
}

impl Render for ExtractContent {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let h = cx.entity();

        let input = cx.new(|cx2| {
            InputState::new(window, cx2)
                .placeholder("Select destination folder...")
                .default_value(&self.destination)
        });

        v_flex()
            .size_full()
            .gap(px(12.))
            .child(DialogHeader::new().child(DialogTitle::new().child("Extract")))
            .child(
                DialogContent::new().child(
                    v_flex()
                        .gap_3()
                        .child(
                            div()
                                .text_base()
                                .font_weight(FontWeight::BOLD)
                                .child("Extract"),
                        )
                        .child(div().text_sm().child(format!(
                            "{} entr{} selected",
                            self.entries_count,
                            if self.entries_count == 1 { "y" } else { "ies" }
                        )))
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
                                            if let Some(path) = bit7z_infra_platform::pick_folder()
                                            {
                                                this.destination =
                                                    path.to_string_lossy().to_string();
                                                input.update(cx, |state, cx2| {
                                                    state.set_value(
                                                        path.to_string_lossy(),
                                                        window2,
                                                        cx2,
                                                    )
                                                });
                                                cx.notify();
                                            }
                                        }),
                                    ),
                            ),
                        )
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
                        .child(
                            Select::new(&self.overwrite_mode)
                                .cleanable(false)
                                .title_prefix("Overwrite mode: "),
                        )
                        .child(
                            Checkbox::new("keepBroken")
                                .checked(self.keep_broken)
                                .label("Keep broken files")
                                .cursor_pointer()
                                .on_click(cx.listener(|this, _e, _window, cx| {
                                    this.keep_broken = !this.keep_broken;
                                    cx.notify();
                                })),
                        ),
                ),
            )
            .child(
                DialogFooter::new()
                    .justify_end()
                    .gap_2()
                    .child(Button::new("cancel").label("Cancel").on_click({
                        let h = h.clone();
                        move |_, window, cx| {
                            h.update(cx, |this, cx| {
                                this.emit(ExtractDialogEvent::Canceled, window);
                            });
                        }
                    }))
                    .child(if !self.destination.is_empty() {
                        Button::new("extract")
                            .label("Extract")
                            .primary()
                            .on_click({
                                let h = h.clone();
                                move |_, window, cx| {
                                    h.update(cx, |this, cx| {
                                        let ev = ExtractDialogEvent::ExtractRequested {
                                            destination: std::path::PathBuf::from(
                                                &this.destination,
                                            ),
                                            preserve_paths: this.preserve_paths,
                                            overwrite_mode: *this
                                                .overwrite_mode
                                                .read(cx)
                                                .selected_value()
                                                .unwrap(),
                                            keep_broken: this.keep_broken,
                                        };
                                        this.emit(ev, window);
                                    });
                                }
                            })
                            .into_any_element()
                    } else {
                        Button::new("extract")
                            .label("Extract")
                            .primary()
                            .disabled(true)
                            .into_any_element()
                    }),
            )
    }
}

pub struct ExtractDialog;

impl ExtractDialog {
    pub fn open(entries: Vec<ArchiveEntry>, cx: &mut AsyncApp) -> Receiver<ExtractDialogEvent> {
        let (tx, rx) = unbounded::<ExtractDialogEvent>();
        let event_tx: SharedSender<ExtractDialogEvent> = Arc::new(Mutex::new(Some(tx)));
        let et = event_tx.clone();

        open_window_dialog_async(
            cx,
            WindowDialogOptions {
                title: "Extract".into(),
                width: px(560.),
                height: Some(px(460.)),
                min_width: None,
                min_height: None,
                kind: WindowKind::Dialog,
                close_action: CloseAction::RemoveWindow,
                window_decorations: Some(WindowDecorations::Client),
                window_background: WindowBackgroundAppearance::Opaque,
            },
            move |window, cx| cx.new(|cx| ExtractContent::new(window, cx, entries.clone(), et)),
        );
        rx
    }
}
