use crate::archive_sidebar::ArchiveSideBar;
use crate::file_list::archive_file_list::{ArchiveFileList, FileListEvent};
use crate::file_list::archive_fm_state::FileListState;
use crate::status_bar::AppStatusBar;
use crate::usecase::UseCases;
use crate::*;
use bit7z_domain::archive::Password;
use bit7z_domain::repository::{ArchiveError, ArchiveRepository};
use bit7z_pres_components::ext_table::Column;
use gpui::{prelude::*, *};
use gpui_component::breadcrumb::{Breadcrumb, BreadcrumbItem};
use gpui_component::button::Button;
use gpui_component::input::Input;
use gpui_component::sidebar::{Sidebar, SidebarGroup, SidebarItem, SidebarMenu, SidebarToggleButton};
use gpui_component::{
    h_flex, input::{InputEvent, InputState}, resizable::{h_resizable, resizable_panel}, v_flex,
    IconName,
    Side,
    Sizable,
};
use std::sync::Arc;

//处理文件管理器相关业务action
pub struct ArchiveFileManager {
    left_status: SharedString,
    right_status: SharedString,
    subdirs: Vec<String>,
    recent_files: Vec<String>,
    collapsed: bool,
    side: Side,
    show_address_input: bool,
    use_cases: Arc<UseCases>,
    // selection: HashSet<u32>,
    // selection_anchor: Option<u32>,
    address_input: Entity<InputState>,
    state: Entity<FileListState>,
    // entry_list: Entity<ArchiveFileList>,
    // ribbon: Entity<Ribbon>,
    _subscriptions: Vec<Subscription>,
}

impl ArchiveFileManager {
    pub fn new(
        init_path: Option<&str>,
        init_password: Option<Password>,
        repo: Arc<dyn ArchiveRepository>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {

        let use_cases = Arc::new(UseCases::new(repo));

        let columns = vec![
            Column::new("name", "Name").width(300.).sortable(),
            Column::new("size", "Size").width(80.).sortable(),
            Column::new("packed", "Packed").width(80.).sortable(),
            Column::new("ratio", "Ratio").width(80.).sortable(),
            Column::new("date", "Date").width(140.).sortable(),
        ];

        let list_state = cx.new(|cx|
            FileListState::new(window, cx)
                .column(cx,columns)

        );

        let address_input_state = cx.new(|cx| {
            InputState::new(window, cx)
                .context_menu(true)
                .multi_line(false)
        });


        let _subscriptions = vec![
            cx.subscribe(&address_input_state, |this, _, e, cx| match e {
                InputEvent::Change => {
                    // this.active_group_index = Some(0);
                    // this.active_index = Some(0);
                    cx.notify()
                }
                _ => {}
            }),
            cx.subscribe(&list_state, |this, _, e, cx| {
                match e {
                    FileListEvent::SelectionChanged(rows) => {

                    }
                    FileListEvent::OpenEntry => {

                    }
                    _ => {}
                }
                cx.notify()
            }),
        ];

        Self {
            left_status: SharedString::default(),
            right_status: SharedString::default(),
            subdirs: vec![],
            recent_files: vec![],
            use_cases,
            address_input: address_input_state,
            state: list_state,
            side: Side::Left,
            collapsed: false,
            show_address_input: false,
            _subscriptions,
        }
    }

    fn set_collapse_state(&mut self, is_collapse: bool, window: &mut Window, cx: &mut App) {
        self.collapsed = is_collapse;
    }

    fn on_navigate_up(&mut self,cx: &mut Context<Self>) {
        self.state.update(cx,|state, cx| {
            state.navigate_up()
        });
    }

    fn on_action_open_archive(
        &mut self,
        _: &OpenArchive,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {

    }

    // pub fn handle_open_archive(&mut self, path: &Path, password: Option<Password>, cx: &mut Context<Self>) {
    //     self.state.status = ViewStatus::Loading;
    //     self.sync_root_view(cx);
    //     let uc = self.use_cases.clone();
    //     // let path_buf = path.to_path_buf();
    //     // let path_string = path.to_string_lossy().to_string();
    //     // let pw = password.map(|s| Password::new(s));
    //     // let pw_clone = pw.clone();
    //     // let path_for_password = path_string.clone();
    //
    //     let bg_task = cx.background_spawn(async move {
    //         uc.repo.open(&path_buf, password)
    //     });
    //     cx.spawn(async move |this, cx| {
    //         let this_strong = this.clone().upgrade();
    //         match bg_task.await {
    //             Ok(handle) => {
    //                 this.update(cx, |this, cx| {
    //                     this.state.handle = Some(handle); this.state.archive_password = pw_clone;
    //                     this.state.current_path = String::new(); this.state.path_history.clear(); this.state.directory_cache.clear();
    //                     bit7z_pres_settings::SettingsStore::get_mut(cx).update_and_save(|p| p.archive.add_recent(path_string.clone()));
    //                     this.load_current_directory(cx);
    //                 }).ok();
    //             }
    //             Err(ArchiveError::EncryptedArchiveRequiresPassword) => {
    //                 this.update(cx, |this, cx| { this.state.status = ViewStatus::Empty; }).ok();
    //                 if let Some(this_strong) = this_strong.clone() {
    //                     Self::password_prompt(path_for_password, this_strong, cx);
    //                 }
    //             }
    //             Err(e) => { this.update(cx, |this, _| { this.state.status = ViewStatus::Error(e.to_string()); }).ok(); }
    //         }
    //     }).detach();
    // }

    pub fn view(
        init_story: Option<&str>,
        init_password: Option<Password>,
        repo: Arc<dyn ArchiveRepository>,
        window: &mut Window,
        cx: &mut App,
    ) -> Entity<Self> {
        cx.new(|cx| Self::new(
            init_story,
            init_password,
            repo,
            window,
            cx
        ))
    }
}

impl Render for ArchiveFileManager {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {

        let sidebar: Sidebar<SidebarGroup<SidebarMenu>> = Sidebar::new("archive-browser")
            .w(relative(1.))
            .collapsed(self.collapsed);

        let body = h_resizable("gallery-container")
            .child(
                resizable_panel()
                    .when_else(
                        self.collapsed,
                        |this| this.size(px(0.)).size_range(px(0.)..px(0.)),
                        |this| this.size(px(255.)).size_range(px(200.)..px(320.)),
                    )
                    .child(sidebar),
            )
            .child(
                v_flex()
                    .flex_1()
                    // .h_full()
                    .overflow_x_hidden()
                    .child(
                        div().id("story").flex_1().overflow_y_scroll().child(
                            v_flex()
                                // .size_full()
                                .child(
                                    h_flex()
                                        .gap_2()
                                        .min_h_0()
                                        .child(
                                            Button::new("navigation-up-dir")
                                                .icon(IconName::ArrowUp)
                                                .small()
                                                .gap_2()
                                                .tooltip("Navigate Up")
                                                .on_click( move |e, _w, cx| {
                                                    cx.new(
                                                        |cx| {

                                                        }
                                                    );
                                                }),
                                        )
                                        .child(div().when_else(
                                            !self.show_address_input,
                                            |this| {
                                                this.child(
                                                    Breadcrumb::new().children(
                                                        self.state
                                                            .read(cx)
                                                            .current_path
                                                            .split("/")
                                                            .map(|x| {
                                                                BreadcrumbItem::new(x)
                                                                    .on_click(|e, window, cx| {

                                                                    })
                                                            }),
                                                    ),
                                                )
                                            },
                                            |this| {
                                                this.child(
                                                    Input::new(&self.address_input)
                                                        .w_full()
                                                        .suffix(
                                                            Button::new("navigation-to-button")
                                                                .icon(IconName::ArrowRight)
                                                                .on_click(|e, w, cx| {}),
                                                        ),
                                                )
                                            },
                                        )),
                                )
                                .child(ArchiveFileList::new(&self.state).size_full()),
                        ),
                    )
                    .into_any_element(),
            );

        v_flex()
            .size_full()
            .on_action(cx.listener(Self::on_action_open_archive))
            // .child(div().child(self.ribbon.clone()))
            .child(div().flex_1().child(body))
            .child(
                AppStatusBar::new()
                    .left_icon(
                        div().child(
                            SidebarToggleButton::new()
                                .side(self.side)
                                .on_click(cx.listener(|this, _event, _, cx| {
                                    this.collapsed = !this.collapsed;
                                    cx.notify();
                                }))
                                .collapsed(self.collapsed),
                        ),
                    )
                    .left(self.left_status.clone())
                    .right(self.right_status.clone()),
            )
    }
}
