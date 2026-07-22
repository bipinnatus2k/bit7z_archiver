use crate::menu::ToggleSidebar;
use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::button::Button;
use gpui_component::input::{InputEvent, InputState};
use gpui_component::sidebar::{
    Sidebar, SidebarFooter, SidebarGroup, SidebarHeader, SidebarMenu, SidebarMenuItem,
    SidebarToggleButton,
};
use gpui_component::{Icon, IconName, h_flex};
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub enum BrowserIntent {
    NavigateInto(String),
    OpenRecentFile(String),
    SetFilter(String),
}

impl EventEmitter<BrowserIntent> for ArchiveBrowser {}

pub struct ArchiveBrowser {
    subdirs: Vec<String>,
    recent_files: Vec<String>,
    input_state: Entity<InputState>,
    collapsed: bool,
    _subscriptions: Vec<Subscription>,
}

impl ArchiveBrowser {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input_state = cx.new(|cx| InputState::new(window, cx).placeholder("Filter..."));
        let _subscriptions = vec![cx.subscribe_in(&input_state, window, {
            move |this: &mut Self, _, ev: &InputEvent, _: &mut Window, cx| match ev {
                InputEvent::Change => {
                    let value = this.input_state.read(cx).value();
                    cx.emit(BrowserIntent::SetFilter(value.to_string()))
                }
                _ => {}
            }
        })];
        Self {
            subdirs: vec![],
            recent_files: vec![],
            input_state,
            collapsed: false,
            _subscriptions,
        }
    }

    pub fn set_state(&mut self, subdirs: Vec<String>, recent_files: Vec<String>) {
        self.subdirs = subdirs;
        self.recent_files = recent_files;
    }

    pub fn set_collapsed(&mut self, collapsed: bool, cx: &mut Context<Self>) {
        self.collapsed = collapsed;
        cx.notify();
    }
}

impl Render for ArchiveBrowser {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let self_handle = cx.entity();
        let collapsed = self.collapsed;

        let sidebar = Sidebar::new("archive-browser")
            .collapsible(true)
            .collapsed(collapsed)
            .header(SidebarHeader::new().w_full().when_else(
                !collapsed,
                |el| {
                    el.child(
                        h_flex()
                            .gap_2()
                            .child(Button::new("icon").icon(IconName::Folder))
                            .child("Explorer"),
                    )
                },
                |el| el.child(Icon::new(IconName::Menu)),
            ))
            .child(
                SidebarGroup::new("Explorer").child(
                    SidebarMenu::new().child(
                        SidebarMenuItem::new("Folder")
                            .icon(IconName::Folder)
                            .children(self.subdirs.iter().cloned().map(|name| {
                                let h = self_handle.clone();
                                SidebarMenuItem::new(name.clone())
                                    .icon(IconName::Folder)
                                    .on_click(
                                        move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
                                            h.update(cx, |_, cx| {
                                                cx.emit(BrowserIntent::NavigateInto(name.clone()))
                                            });
                                        },
                                    )
                            })),
                    ),
                ),
            )
            .child(
                SidebarGroup::new("Fast Access").child(
                    SidebarMenu::new()
                        .child(
                            SidebarMenuItem::new("Recent Files")
                                .icon(IconName::History)
                                .children(self.recent_files.iter().cloned().map(|path| {
                                    let h = self_handle.clone();
                                    let file_name = Path::new(&path)
                                        .file_name()
                                        .map(|n| n.to_string_lossy().to_string())
                                        .unwrap_or_else(|| path.clone());
                                    SidebarMenuItem::new(file_name)
                                        .icon(IconName::File)
                                        .on_click(
                                            move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
                                                h.update(cx, |_, cx| {
                                                    cx.emit(BrowserIntent::OpenRecentFile(
                                                        path.clone(),
                                                    ))
                                                });
                                            },
                                        )
                                })),
                        )
                        .child(SidebarMenuItem::new("Collection").icon(IconName::Star)),
                ),
            )
            .footer(
                SidebarFooter::new().child(
                    SidebarToggleButton::new()
                        .collapsed(self.collapsed)
                        .on_click(|_, window, cx| {
                            window.dispatch_action(Box::new(ToggleSidebar), cx);
                        }),
                ),
            );

        sidebar.w(relative(1.))
    }
}
