use std::time::Duration;
use gpui::*;
use gpui_component::sidebar::{
    Sidebar, SidebarCollapsible, SidebarGroup, SidebarMenu, SidebarMenuItem,
};
use gpui_component::IconName;

#[derive(IntoElement)]
pub struct ArchiveSideBar {
    id: ElementId,
    style: StyleRefinement,
    subdirs: Vec<SidebarMenuItem>,
    recent_files: Vec<SidebarMenuItem>,
    collections: Vec<SidebarMenuItem>,
    collapsed: bool,
}

impl Styled for ArchiveSideBar {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ArchiveSideBar {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            subdirs: vec![],
            recent_files: vec![],
            collections: vec![],
            collapsed: false,
        }
    }

    pub fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }

    pub fn dictionaries(mut self, subdirs: Vec<SidebarMenuItem>) -> Self {
        self.subdirs = subdirs;
        self
    }

    pub fn recent_files(mut self, files: Vec<SidebarMenuItem>) -> Self {
        self.recent_files = files;
        self
    }

    pub fn collections(mut self, files: Vec<SidebarMenuItem>) -> Self {
        self.collections = files;
        self
    }
}

impl RenderOnce for ArchiveSideBar {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        Sidebar::new(self.id)
            .collapsible(true)
            .collapsed(self.collapsed)
            .w(relative(1.))
            .border_0()
            .collapsible(SidebarCollapsible::Offcanvas)
            .child(
                SidebarGroup::new("Explorer").child(
                    SidebarMenu::new().child(
                        SidebarMenuItem::new("Folder")
                            .icon(IconName::Folder)
                            .children(self.subdirs),
                    ),
                ),
            )
            .child(
                SidebarGroup::new("Fast Access").child(
                    SidebarMenu::new()
                        .child(
                            SidebarMenuItem::new("Recent Files")
                                .icon(IconName::History)
                                .children(self.recent_files),
                        )
                        .child(SidebarMenuItem::new("Collection").icon(IconName::Star))
                        .children(self.collections),
                ),
            )
    }
}
