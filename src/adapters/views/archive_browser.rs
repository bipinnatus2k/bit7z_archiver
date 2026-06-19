use crate::adapters::view_models::archive_vm::ArchiveViewModel;
use crate::domain::preferences::Preferences;
use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::sidebar::{
    Sidebar, SidebarFooter, SidebarGroup, SidebarHeader, SidebarMenu, SidebarMenuItem,
    SidebarToggleButton,
};
use gpui_component::{Icon, IconName};
use std::path::Path;

pub struct ArchiveBrowser {
    archive_vm: Entity<ArchiveViewModel>,
    input_state: Entity<InputState>,
    collapsed: bool,
    _subscriptions: Vec<Subscription>,
}

impl ArchiveBrowser {
    pub fn new(archive_vm: Entity<ArchiveViewModel>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input_state = cx.new(|cx| InputState::new(window, cx).placeholder("Filter..."));

        let _subscriptions = vec![cx.subscribe_in(&input_state, window, {
            let archive_vm = archive_vm.clone();
            let input_state = input_state.clone();
            move |_this, _, ev: &InputEvent, _window, cx| match ev {
                InputEvent::Change => {
                    let value = input_state.read(cx).value();
                    archive_vm.update(cx, |vm, cx| vm.set_filter(&value, cx));
                }
                _ => {}
            }
        })];

        Self {
            archive_vm,
            input_state,
            collapsed: false,
            _subscriptions,
        }
    }

    pub fn toggle_collapsed(&mut self, cx: &mut Context<Self>) {
        self.collapsed = !self.collapsed;
        cx.notify();
    }

    pub fn is_collapsed(&self) -> bool {
        self.collapsed
    }
}

impl Render for ArchiveBrowser {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let vm = self.archive_vm.read(cx);
        let prefs = cx.global::<Preferences>();
        let recent_files = &prefs.archive.recent_files;
        let has_recent = !recent_files.is_empty();
        let collapsed = self.collapsed;
        let subdirs = vm.filtered_subdirs();
        drop(vm);

        let this = cx.entity();

        let sidebar = Sidebar::new("archive-browser")
            .collapsible(true)
            .collapsed(collapsed)
            .header(
                SidebarHeader::new()
                    .child(
                        div().flex().flex_row().gap_2()
                            .child(Icon::new(IconName::FolderOpen))
                            .when(!collapsed, |this| this.child("File Explorer"))
                    )
            )
            .child(
                SidebarGroup::new("Folders")
                    .child(
                        SidebarMenu::new()
                            .children(subdirs.into_iter().map(|name| {
                                let avm = self.archive_vm.clone();
                                SidebarMenuItem::new(name.clone())
                                    .icon(IconName::Folder)
                                    .on_click(move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
                                        avm.update(cx, |vm, cx| {
                                            vm.navigate_into(&name, cx);
                                        });
                                    })
                            }))
                    )
            )
            .when(has_recent, |sidebar| sidebar.child(
                SidebarGroup::new("Recent Files")
                    .child(
                        SidebarMenu::new()
                            .children(recent_files.iter().map(|path| {
                                let path = path.clone();
                                let avm = self.archive_vm.clone();
                                let file_name = Path::new(&path)
                                    .file_name()
                                    .map(|n| n.to_string_lossy().to_string())
                                    .unwrap_or_else(|| path.clone());
                                SidebarMenuItem::new(file_name)
                                    .icon(IconName::File)
                                    .on_click(move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
                                        let path_ref = std::path::Path::new(&path);
                                        avm.update(cx, |vm, cx| {
                                            vm.open_archive(path_ref, None, cx);
                                        });
                                    })
                            }))
                    )
            ))
            .footer(
                SidebarFooter::new()
                    .child(
                        SidebarToggleButton::new()
                            .collapsed(collapsed)
                            .on_click({
                                let this = this.clone();
                                move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
                                    this.update(cx, |this, cx| this.toggle_collapsed(cx));
                                }
                            })
                    )
            );

        div().flex().flex_col().size_full()
            .child(Input::new(&self.input_state).px_1().py_1())
            .child(sidebar.flex_1())
    }
}
