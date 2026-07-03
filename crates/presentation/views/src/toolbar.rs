use bit7z_pres_theme::design::BREAKPOINT_SM;
use gpui::*;
use gpui_component::button::Button;
use gpui_component::{h_flex, Disableable, IconName};

#[derive(Debug, Clone, PartialEq)]
pub enum ToolbarIntent {
    OpenArchive,
    CreateArchive,
    AddFiles,
    ExtractSelected,
    TestArchive,
    CloseArchive,
    ShowSettings,
}

impl EventEmitter<ToolbarIntent> for Toolbar {}

pub struct Toolbar {
    is_open: bool,
    is_ready: bool,
    has_selection: bool,
    is_loading: bool,
}

impl Toolbar {
    pub fn new() -> Self {
        Self {
            is_open: false,
            is_ready: false,
            has_selection: false,
            is_loading: false,
        }
    }

    pub fn set_state(&mut self, is_open: bool, is_ready: bool, has_selection: bool) {
        self.is_open = is_open;
        self.is_ready = is_ready;
        self.has_selection = has_selection;
    }

    pub fn set_loading(&mut self, is_loading: bool) {
        self.is_loading = is_loading;
    }
}

impl Render for Toolbar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let window_width = window.bounds().size.width;
        let compact = window_width < px(BREAKPOINT_SM);
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
