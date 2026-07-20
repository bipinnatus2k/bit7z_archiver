use std::ops::Add;
use std::rc::Rc;
use gpui::*;
use gpui_component::{h_flex, v_flex, ActiveTheme, Disableable, IconName, Sizable};
use gpui_component::breadcrumb::{Breadcrumb, BreadcrumbItem};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::native_menu::NativeMenu;
use serde::Deserialize;
use crate::file_list::archive_fm_state::FileListState;

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub enum FileListEvent {
    SelectionChanged(Vec<u32>),
    OpenEntry,
}

actions!(archive_file_list,[
    SelectionChanged,
    SortByColumn,
]);

#[derive(IntoElement)]
pub struct ArchiveFileList {
    state: Entity<FileListState>,
    style: StyleRefinement,
    /// An optional context menu builder to allow a custom context menu on the input.
    ///
    /// If set, this overrides the built-in context menu.
    context_menu_builder: Option<Rc<dyn Fn(NativeMenu, &mut Window, &mut App) -> NativeMenu>>,
}

impl Styled for ArchiveFileList {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ArchiveFileList {

    pub fn init(cx: &mut App) {
        
    }


    pub fn new(state: &Entity<FileListState>) -> Self {
        Self {
            state: state.clone(),
            style: StyleRefinement::default(),
            context_menu_builder: None,
        }
    }

}

impl RenderOnce for ArchiveFileList {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        v_flex()
            .w_full()
            .flex_1()
            .key_context("archive_file_list")
            .track_focus(&self.state.read(cx).focus_handle.clone())
            .child(self.state.clone())
    }
}

#[derive(IntoElement)]
struct AddressBar {
    address: Vec<Address>,
}

type Address = String;

impl AddressBar {
    
}


impl RenderOnce for AddressBar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        h_flex()
            .w_full()
            .px_2()
            .py_1()
            .gap_1()
            .bg(cx.theme().background)
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                Button::new("nav-up")
                    .ghost()
                    .small()
                    .icon(IconName::ArrowUp)
                    // .disabled(!has_entries)
                ,
            )
            .child(
                Breadcrumb::new()
                    .bg(cx.theme().background)
                    .children({
                        let mut items: Vec<BreadcrumbItem> = Vec::new();
                        let _ = self.address.iter().fold(String::new(), |acc, p| {
                            let full_path = if acc.is_empty() {
                                p.to_string()
                            } else {
                                format!("{}/{}", acc, p)
                            };
                            items.push(BreadcrumbItem::new(p.to_string()));
                            full_path
                        });
                        items
                    }),
            )
    }
}