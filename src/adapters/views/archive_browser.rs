use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::button::Button;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::sidebar::{Sidebar, SidebarGroup, SidebarHeader, SidebarMenu, SidebarMenuItem};
use gpui_component::{h_flex, v_flex, IconName};
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
        let self_handle = cx.entity();
        let collapsed = self.collapsed;

        let sidebar = Sidebar::new("archive-browser")
            .collapsible(true)
            .collapsed(collapsed)
            .border_0()
            .header(
                v_flex()
                    .w_full()
                    .gap_3()
                    .child(
                        SidebarHeader::new().w_full().child(
                            h_flex()
                                .child(
                                    Button::new("collapse")
                                        .icon(IconName::FolderOpen)
                                        // .small()
                                        .on_click({
                                            let this = self_handle.clone();
                                            move |click_event: &ClickEvent, _: &mut Window, cx: &mut App| {
                                                if click_event.click_count() >= 2 {
                                                    this.update(cx, |this, cx| {
                                                        this.toggle_collapsed(cx)
                                                    });
                                                }
                                            }
                                        }),
                                )
                                .when(!collapsed, |this| this.gap_2().child("File Explorer")),
                        ),
                    )
                    .when(!collapsed,|this| this.w_full().child(Input::new(&self.input_state).rounded_2xl().bordered(false))),
            )
            .child(
                SidebarGroup::new("Folders").child(SidebarMenu::new().children(
                    self.subdirs.iter().cloned().map(|name| {
                        let h = self_handle.clone();
                        SidebarMenuItem::new(name.clone())
                            .icon(IconName::Folder)
                            .on_click(move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
                                h.update(cx, |_, cx| {
                                    cx.emit(BrowserIntent::NavigateInto(name.clone()))
                                });
                            })
                    }),
                )),
            )
            .when(!self.recent_files.is_empty(), |sidebar| {
                sidebar.child(
                    SidebarGroup::new("Recent Files").child(SidebarMenu::new().children(
                        self.recent_files.iter().cloned().map(|path| {
                            let h = self_handle.clone();
                            let file_name = Path::new(&path)
                                .file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_else(|| path.clone());
                            SidebarMenuItem::new(file_name)
                                .icon(IconName::File)
                                .on_click(move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
                                    h.update(cx, |_, cx| {
                                        cx.emit(BrowserIntent::OpenRecentFile(path.clone()))
                                    });
                                })
                        }),
                    )),
                )
            });

        sidebar.w(relative(1.))
    }
}
