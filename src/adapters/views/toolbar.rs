use gpui::*;
use gpui_component::button::Button;
use gpui_component::Disableable;

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
}

impl Toolbar {
    pub fn new() -> Self {
        Self { is_open: false, is_ready: false, has_selection: false }
    }

    pub fn set_state(&mut self, is_open: bool, is_ready: bool, has_selection: bool) {
        self.is_open = is_open;
        self.is_ready = is_ready;
        self.has_selection = has_selection;
    }
}

impl Render for Toolbar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let window_width = window.bounds().size.width;
        let compact = window_width < px(640.);
        let show_labels = !compact;

        let mut row = gpui_component::h_flex().gap_2().p_2().w_full();

        let self_handle = cx.entity().clone();

        let open_btn = Button::new("open").on_click({
            let h = self_handle.clone();
            move |_, _, cx| { h.update(cx, |_, cx| cx.emit(ToolbarIntent::OpenArchive)); }
        });
        let open_btn = if show_labels { open_btn.label("Open") } else { open_btn };
        row = row.child(open_btn);

        let create_btn = Button::new("create").on_click({
            let h = self_handle.clone();
            move |_, _, cx| { h.update(cx, |_, cx| cx.emit(ToolbarIntent::CreateArchive)); }
        });
        let create_btn = if show_labels { create_btn.label("Create") } else { create_btn };
        row = row.child(create_btn);

        let add_btn = Button::new("add").disabled(!self.is_open).on_click({
            let h = self_handle.clone();
            move |_, _, cx| { h.update(cx, |_, cx| cx.emit(ToolbarIntent::AddFiles)); }
        });
        let add_btn = if show_labels { add_btn.label("Add") } else { add_btn };
        row = row.child(add_btn);

        let extract_btn = Button::new("extract").disabled(!self.is_ready || !self.has_selection).on_click({
            let h = self_handle.clone();
            move |_, _, cx| { h.update(cx, |_, cx| cx.emit(ToolbarIntent::ExtractSelected)); }
        });
        let extract_btn = if show_labels { extract_btn.label("Extract") } else { extract_btn };
        row = row.child(extract_btn);

        let test_btn = Button::new("test").disabled(!self.is_open).on_click({
            let h = self_handle.clone();
            move |_, _, cx| { h.update(cx, |_, cx| cx.emit(ToolbarIntent::TestArchive)); }
        });
        let test_btn = if show_labels { test_btn.label("Test") } else { test_btn };
        row = row.child(test_btn);

        let close_btn = Button::new("close").disabled(!self.is_open).on_click({
            let h = self_handle.clone();
            move |_, _, cx| { h.update(cx, |_, cx| cx.emit(ToolbarIntent::CloseArchive)); }
        });
        let close_btn = if show_labels { close_btn.label("Close") } else { close_btn };
        row = row.child(close_btn);

        row.child(div().flex_1())
            .child(
                Button::new("settings").on_click(move |_, _, cx| {
                    self_handle.update(cx, |_, cx| cx.emit(ToolbarIntent::ShowSettings));
                })
            )
    }
}
