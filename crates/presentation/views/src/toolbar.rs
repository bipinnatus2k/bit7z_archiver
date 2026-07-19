use bit7z_pres_components::toolbar::{ToolbarAlignment, ToolbarButton, ToolbarDivider};
use gpui::*;
use gpui_component::button::Button;
use gpui_component::{Disableable, IconName};

const NAMESPACE: &str = "Toolbar";

actions!(
    NAMESPACE,
    [
        OpenArchive,
        CreateNewArchive,
        AppendFiles,
        Extract,
        TestArchive,
        TestSelection,
        CloseArchive,
        Info,
        Settings,
    ]
);

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

impl Render for Toolbar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let window_width = window.bounds().size.width;
        let compact = window_width < px(640.0);
        let show_labels = !compact;

        let self_handle = cx.entity().clone();

        bit7z_pres_components::toolbar::ToolBar::new()
            .title("BitArchiver")
            .action(
                ToolbarButton::new("open")
                    .label("Open")
                    .icon(IconName::FolderOpen)
                    .tooltip("Open archive (Ctrl+O)")
                    .on_action(|x: &OpenArchive, window, cx| {
                        
                    })
                    .on_click(cx.listener({
                        let h = self_handle.clone();
                        move |_, _, _,cx| {
                            h.update(cx, |_, cx| cx.emit(ToolbarIntent::OpenArchive));
                        }
                    })),
            )
            .action(
                ToolbarButton::new("create")
                    .label("Create")
                    .icon(IconName::Plus)
                    .tooltip("Create new archive (Ctrl+N)")
                    .on_click({
                        let h = self_handle.clone();
                        move |_, _, cx| {
                            h.update(cx, |_, cx| cx.emit(ToolbarIntent::CreateArchive));
                        }
                    }),
            )
            .action(
                Button::new("add")
                    .label("Add")
                    .icon(IconName::Plus)
                    .tooltip("Add files to archive")
                    .disabled(!self.is_open || self.is_loading)
                    .on_click({
                        let h = self_handle.clone();
                        move |_, _, cx| {
                            h.update(cx, |_, cx| cx.emit(ToolbarIntent::AddFiles));
                        }
                    }),
            )
            .action(
                ToolbarButton::new("extract")
                    .label("Extract")
                    .icon(IconName::ChevronDown)
                    .tooltip("Extract selected files (Ctrl+E)")
                    .disabled(!self.is_ready || !self.has_selection || self.is_loading)
                    .on_click({
                        let h = self_handle.clone();
                        move |_, _, cx| {
                            h.update(cx, |_, cx| cx.emit(ToolbarIntent::ExtractSelected));
                        }
                    }),
            )
            .action(
                ToolbarButton::new("test")
                    .icon(IconName::PanelRightClose)
                    .label("Test")
                    .tooltip("Test archive integrity (Ctrl+T)")
                    .disabled(!self.is_open || self.is_loading)
                    .on_click({
                        let h = self_handle.clone();
                        move |_, _, cx| {
                            h.update(cx, |_, cx| cx.emit(ToolbarIntent::TestArchive));
                        }
                    }),
            )
            .action(
                ToolbarButton::new("close")
                    .label("Close")
                    .icon(IconName::Close)
                    .tooltip("Close archive")
                    .disabled(!self.is_open || self.is_loading)
                    .on_click({
                        let h = self_handle.clone();
                        move |_, _, cx| {
                            h.update(cx, |_, cx| cx.emit(ToolbarIntent::CloseArchive));
                        }
                    }),
            )
            .action(
                    ToolbarButton::new("settings")
                        .label("Settings")
                        .icon(IconName::Settings)
                        .tooltip("Settings")
                        .on_click(move |_, _, cx| {
                            self_handle.update(cx, |_, cx| cx.emit(ToolbarIntent::ShowSettings));
                        }),
            )
            .action(ToolbarDivider::default())
            .actions_alignment(ToolbarAlignment::End)
    }
}
