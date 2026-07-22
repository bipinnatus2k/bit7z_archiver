use gpui::*;
use gpui_component::button::Button;
use gpui_component::{Disableable, IconName, h_flex};

#[derive(Debug, Clone, PartialEq)]
pub enum ToolbarIntent {
    OpenArchive,
    CreateArchive,
    AddFiles,
    ExtractSelected,
    TestArchive,
    CloseArchive,
    ShowSettings,
    SaveArchive,
    Undo,
    Redo,
}

impl EventEmitter<ToolbarIntent> for Toolbar {}

pub struct Toolbar {
    is_open: bool,
    is_ready: bool,
    has_selection: bool,
    is_loading: bool,
    has_unsaved_changes: bool,
    can_undo: bool,
    can_redo: bool,
}

impl Toolbar {
    pub fn new() -> Self {
        Self {
            is_open: false,
            is_ready: false,
            has_selection: false,
            is_loading: false,
            has_unsaved_changes: false,
            can_undo: false,
            can_redo: false,
        }
    }

    pub fn set_state(&mut self, is_open: bool, is_ready: bool, has_selection: bool) {
        self.is_open = is_open;
        self.is_ready = is_ready;
        self.has_selection = has_selection;
    }

    pub fn set_edit_state(&mut self, has_unsaved: bool, can_undo: bool, can_redo: bool) {
        self.has_unsaved_changes = has_unsaved;
        self.can_undo = can_undo;
        self.can_redo = can_redo;
    }

    pub fn set_loading(&mut self, is_loading: bool) {
        self.is_loading = is_loading;
    }
}

impl Render for Toolbar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let window_width = window.bounds().size.width;
        let compact = window_width < px(640.0);
        let show_labels = !compact;

        let mut row = h_flex().gap_2().p_2().w_full();

        let self_handle = cx.entity().clone();

        let open_btn = Button::new("open")
            .icon(IconName::FolderOpen)
            .tooltip("Open archive (Ctrl+O)")
            .on_click({
                let h = self_handle.clone();
                move |_, _, cx| {
                    h.update(cx, |_, cx| cx.emit(ToolbarIntent::OpenArchive));
                }
            });
        let open_btn = if show_labels {
            open_btn.label("Open")
        } else {
            open_btn
        };
        row = row.child(open_btn);

        let create_btn = Button::new("create")
            .icon(IconName::Plus)
            .tooltip("Create new archive (Ctrl+N)")
            .on_click({
                let h = self_handle.clone();
                move |_, _, cx| {
                    h.update(cx, |_, cx| cx.emit(ToolbarIntent::CreateArchive));
                }
            });
        let create_btn = if show_labels {
            create_btn.label("Create")
        } else {
            create_btn
        };
        row = row.child(create_btn);

        let add_btn = Button::new("add")
            .icon(IconName::Plus)
            .tooltip("Add files to archive")
            .disabled(!self.is_open || self.is_loading)
            .on_click({
                let h = self_handle.clone();
                move |_, _, cx| {
                    h.update(cx, |_, cx| cx.emit(ToolbarIntent::AddFiles));
                }
            });
        let add_btn = if show_labels {
            add_btn.label("Add")
        } else {
            add_btn
        };
        row = row.child(add_btn);

        let extract_btn = Button::new("extract")
            .icon(IconName::ChevronDown)
            .tooltip("Extract selected files (Ctrl+E)")
            .disabled(!self.is_ready || !self.has_selection || self.is_loading)
            .on_click({
                let h = self_handle.clone();
                move |_, _, cx| {
                    h.update(cx, |_, cx| cx.emit(ToolbarIntent::ExtractSelected));
                }
            });
        let extract_btn = if show_labels {
            extract_btn.label("Extract")
        } else {
            extract_btn
        };
        row = row.child(extract_btn);

        let test_btn = Button::new("test")
            .icon(IconName::PanelRightClose)
            .tooltip("Test archive integrity (Ctrl+T)")
            .disabled(!self.is_open || self.is_loading)
            .on_click({
                let h = self_handle.clone();
                move |_, _, cx| {
                    h.update(cx, |_, cx| cx.emit(ToolbarIntent::TestArchive));
                }
            });
        let test_btn = if show_labels {
            test_btn.label("Test")
        } else {
            test_btn
        };
        row = row.child(test_btn);

        let close_btn = Button::new("close")
            .icon(IconName::Close)
            .tooltip("Close archive")
            .disabled(!self.is_open || self.is_loading)
            .on_click({
                let h = self_handle.clone();
                move |_, _, cx| {
                    h.update(cx, |_, cx| cx.emit(ToolbarIntent::CloseArchive));
                }
            });
        let close_btn = if show_labels {
            close_btn.label("Close")
        } else {
            close_btn
        };
        row = row.child(close_btn);

        // Save / Undo / Redo buttons (separator before them)
        if self.is_open {
            row = row.child(div().w(px(4.)));

            let save_btn = Button::new("save")
                .icon(IconName::Check)
                .tooltip("Save changes (Ctrl+S)")
                .disabled(!self.has_unsaved_changes || self.is_loading)
                .on_click({
                    let h = self_handle.clone();
                    move |_, _, cx| {
                        h.update(cx, |_, cx| cx.emit(ToolbarIntent::SaveArchive));
                    }
                });
            let save_btn = if show_labels {
                save_btn.label("Save")
            } else {
                save_btn
            };
            row = row.child(save_btn);

            let undo_btn = Button::new("undo")
                .icon(IconName::Undo2)
                .tooltip("Undo (Ctrl+Z)")
                .disabled(!self.can_undo || self.is_loading)
                .on_click({
                    let h = self_handle.clone();
                    move |_, _, cx| {
                        h.update(cx, |_, cx| cx.emit(ToolbarIntent::Undo));
                    }
                });
            row = row.child(undo_btn);

            let redo_btn = Button::new("redo")
                .icon(IconName::Redo2)
                .tooltip("Redo (Ctrl+Y)")
                .disabled(!self.can_redo || self.is_loading)
                .on_click({
                    let h = self_handle.clone();
                    move |_, _, cx| {
                        h.update(cx, |_, cx| cx.emit(ToolbarIntent::Redo));
                    }
                });
            row = row.child(redo_btn);
        }

        row.child(div().flex_1()).child(
            Button::new("settings")
                .icon(IconName::Settings)
                .tooltip("Settings")
                .on_click(move |_, _, cx| {
                    self_handle.update(cx, |_, cx| cx.emit(ToolbarIntent::ShowSettings));
                }),
        )
    }
}
