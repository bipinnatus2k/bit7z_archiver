use crate::file_list::archive_file_list::{ArchiveFileList, FileListEvent};
use crate::file_list::archive_fm_state::FileListState;
use crate::file_list::ViewStatus;
use crate::preview_panel::PreviewPanel;
use crate::status_bar::AppStatusBar;
use crate::usecase::UseCases;
use crate::utils::notifications::show_info;
use crate::*;
use bit7z_app_preview::PreviewData;
use bit7z_domain::archive::{ArchiveHandle, Password};
use bit7z_domain::repository::{ArchiveError, ArchiveRepository, ProgressUpdate};
use bit7z_pres_components::ext_table::Column;
use bit7z_pres_components::toolbar::{Toolbar, ToolbarButton};
use bit7z_pres_dialogs::delete::DeleteDialog;
use bit7z_pres_dialogs::password::{PasswordDialog, PasswordResult};
use gpui::{prelude::*, *};
use gpui_component::button::Button;
use gpui_component::input::{InputEvent, InputState};
use gpui_component::notification::Notification;
use gpui_component::resizable::{h_resizable, resizable_panel, v_resizable};
use gpui_component::sidebar::{
    Sidebar, SidebarCollapsible, SidebarGroup, SidebarMenu, SidebarMenuItem, SidebarToggleButton,
};
use gpui_component::{h_flex, v_flex, Icon, Sizable};
use gpui_component::{IconName, Side};
use std::rc::Rc;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub struct ArchiveFileManager {
    left_status: SharedString,
    right_status: SharedString,
    subdirs: Vec<String>,
    recent_files: Vec<String>,
    collapsed: bool,
    side: Side,
    show_address_input: bool,
    show_preview: bool,
    handle: Option<ArchiveHandle>,
    archive_password: Option<Password>,
    use_cases: Arc<UseCases>,
    address_input: Entity<InputState>,
    state: Entity<FileListState>,
    preview_data: Option<Rc<PreviewData>>,
    preview_loading: bool,
    _subscriptions: Vec<Subscription>,
}

impl ArchiveFileManager {
    pub fn new(
        init_path: Option<PathBuf>,
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

        let list_state = cx.new(|cx| FileListState::new(window, cx).column(cx, columns));

        let address_input_state = cx.new(|cx| {
            InputState::new(window, cx)
                .context_menu(true)
                .multi_line(false)
        });

        let _subscriptions = vec![
            cx.subscribe(&address_input_state, |this, _, e, cx| match e {
                InputEvent::Change => cx.notify(),
                InputEvent::PressEnter { secondary, shift } => {}
                InputEvent::Focus => {}
                InputEvent::Blur => {}
            }),
            cx.subscribe(&list_state, move |this, _, event, cx| {
                match event {
                    FileListEvent::SelectionChanged(indices) => {
                        this.selection_changed(indices.clone(), cx);
                    }
                    FileListEvent::OpenEntry => {
                        this.handle_open_entry(cx);
                    }
                    // FileListEvent::NavigateUp => {
                    //     this.state.update(cx, |state, cx| {
                    //         state.navigate_up();
                    //     });
                    //     this.load_current_directory(cx);
                    // }
                    // FileListEvent::ExtractSelected => {
                    //     this.handle_extract(cx);
                    // }
                    // FileListEvent::DeleteSelected => {
                    //     this.handle_delete(cx);
                    // }
                    // FileListEvent::TestSelected => {
                    //     this.handle_test(true, cx);
                    // }
                    // FileListEvent::Refresh => {
                    //     this.state.update(cx, |state, _| {
                    //         state.directory_cache.remove(&state.current_path);
                    //     });
                    //     this.load_current_directory(cx);
                    // }
                    // FileListEvent::ShowProperties => {
                    //     this.handle_properties(cx);
                    // }
                    // FileListEvent::SelectAll => {
                    //     this.state.update(cx, |state, cx| {
                    //         state.select_all();
                    //         cx.emit(FileListEvent::SelectionChanged(
                    //             state
                    //                 .level_entries
                    //                 .iter()
                    //                 .map(|e| e.original_index)
                    //                 .collect(),
                    //         ));
                    //     });
                    // }
                    // FileListEvent::ClearSelection => {
                    //     this.state.update(cx, |state, cx| {
                    //         state.clear_selection();
                    //         cx.emit(FileListEvent::SelectionChanged(vec![]));
                    //     });
                    // }
                    // FileListEvent::SortByColumn(col, asc) => {
                    //     this.state.update(cx, |state, cx| {
                    //         state.apply_sort(*col, *asc, cx);
                    //     });
                    // }
                    _ => {}
                }
                cx.notify();
            }),
        ];

        let fm = Self {
            left_status: SharedString::default(),
            right_status: SharedString::default(),
            subdirs: vec![],
            recent_files: vec![],
            use_cases,
            address_input: address_input_state,
            state: list_state,
            side: Side::Left,
            show_preview: false,
            collapsed: false,
            handle: None,
            archive_password: None,
            show_address_input: false,
            preview_data: None,
            preview_loading: false,
            _subscriptions,
        };

        // Auto-open if path provided
        if let (Some(path), _pw) = (init_path, &init_password) {
            let fm_entity = cx.entity();
            let p = path.clone();
            cx.defer(move |cx| {
                fm_entity.update(cx, |this, cx| {
                    this.handle_open_archive(&p, None, cx);
                });
            });
        }

        fm
    }

    fn selection_changed(&mut self, indices: Vec<u32>, cx: &mut Context<Self>) {
        let uc = self.use_cases.as_ref();

        if !indices.is_empty() {
        } else {
        }
        self.update_status(cx);
    }

    fn handle_open_entry(&mut self, cx: &mut Context<Self>) {
        let handle = self.handle.clone();
        let idx = self.state.read(cx).first_selected_index();
        let Some(idx) = idx else { return };
        let is_dir = self
            .state
            .read(cx)
            .level_entries
            .iter()
            .find(|e| e.original_index == idx)
            .map_or(false, |e| e.is_directory);

        if is_dir {
            let name = self
                .state
                .read(cx)
                .level_entries
                .iter()
                .find(|e| e.original_index == idx)
                .map(|e| e.display_name.clone())
                .unwrap();
            self.state.update(cx, |state, cx| {
                state.navigate_into(&name);
            });
            self.load_current_directory(cx);
        } else if let Some(h) = handle {
            let uc = self.use_cases.clone();
            cx.background_spawn(async move {
                let _ = uc.open_entry(&h, idx);
            })
            .detach();
        }
    }

    fn handle_extract(&mut self, cx: &mut Context<Self>) {
        let indices = self.state.read(cx).selected_indices();
        let entries = self.state.read(cx).selected_entries();
        if entries.is_empty() || self.handle.is_none() {
            return;
        }
        let handle = self.handle.clone().unwrap();
        let uc = self.use_cases.clone();
        let busy = indices.clone();

        cx.spawn(async move |this, cx| {
            use crossbeam_channel::TryRecvError;
            let rx = bit7z_pres_dialogs::extract::ExtractDialog::open(entries, cx);
            loop {
                match rx.try_recv() {
                    Ok(bit7z_pres_dialogs::extract::ExtractDialogEvent::ExtractRequested {
                        destination,
                        overwrite_mode,
                        ..
                    }) => {
                        let (tx, progress_rx) = bit7z_infra_progress::progress_channel();
                        let cancel = Arc::new(AtomicBool::new(false));
                        let paused = Arc::new(AtomicBool::new(false));
                        bit7z_pres_dialogs::progress::ProgressDialog::open(
                            cx,
                            "Extracting...".into(),
                            progress_rx,
                            Some(cancel.clone()),
                            Some(paused.clone()),
                        );
                        // let h = self.handle.clone();
                        let dest = destination.clone();
                        let idx = busy.clone();
                        let u = uc.clone();
                        cx.background_spawn(async move {
                            let _ = u.extract(&handle, &idx, &dest, overwrite_mode);
                            let _ = tx.send(ProgressUpdate {
                                file_current: 0,
                                file_total: 0,
                                current_file: None,
                                items_done: 0,
                                items_total: 0,
                                bytes_done: 0,
                                bytes_total: 0,
                                error: None,
                            });
                        })
                        .detach();
                        break;
                    }
                    Ok(bit7z_pres_dialogs::extract::ExtractDialogEvent::Canceled) => break,
                    Err(TryRecvError::Empty) => {
                        cx.background_spawn(std::future::ready(())).await;
                    }
                    Err(TryRecvError::Disconnected) => break,
                }
            }
            let _ = this.update(cx, |_, cx| cx.notify());
        })
        .detach();
    }

    fn handle_delete(&mut self, cx: &mut Context<Self>) {
        let state = self.state.read(cx);
        if let (Some(h), false) = (&self.handle, state.selected_indices().is_empty()) {
            let indices = state.selected_indices();
            let uc = self.use_cases.clone();
            let repo = uc.repo.clone();
            let handle = h.clone();
            cx.spawn(async move |_, cx| {
                DeleteDialog::open(cx, indices, handle, repo);
            })
            .detach();
        }
    }

    fn handle_test(&mut self, selected: bool, cx: &mut Context<Self>) {
        if let Some(h) = self.handle.clone() {
            let indices = if selected {
                Some(self.state.read(cx).selected_indices())
            } else {
                None
            };
            let uc = self.use_cases.clone();
            cx.spawn(async move |_, cx| {
                bit7z_pres_dialogs::test::TestDialog::open_with_entries(
                    cx,
                    h,
                    indices,
                    uc.repo.clone(),
                );
            })
            .detach();
        }
    }

    fn handle_properties(&mut self, cx: &mut Context<Self>) {
        let entries = self.state.read(cx).selected_entries();
        if !entries.is_empty() {
            cx.spawn(async move |_, cx| {
                bit7z_pres_dialogs::properties::PropertiesDialog::open_entries(entries, cx);
            })
            .detach();
        } else if let Some(ref h) = self.handle.clone() {
            let path_str = h
                .path()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            let uc = self.use_cases.clone();
            let handle = h.clone();
            cx.spawn(async move |_, cx| {
                if let Ok(props) = uc.properties(&handle) {
                    bit7z_pres_dialogs::properties::PropertiesDialog::open_archive(
                        path_str, props, cx,
                    );
                }
            })
            .detach();
        }
    }

    fn on_action_open_archive(
        &mut self,
        _: &OpenArchive,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(path) = bit7z_infra_platform::pick_archive_file() {
            println!("in pick file");
            self.handle_open_archive(&path, None, cx);
        }
    }

    fn on_action_create_archive(
        &mut self,
        _: &CreateArchive,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let repo = self.use_cases.repo.clone();
        cx.spawn(async move |_, cx| {
            bit7z_pres_dialogs::create::CreateArchiveDialog::open(cx, vec![], Some(repo));
        })
        .detach();
    }

    fn on_action_close_archive(
        &mut self,
        _: &CloseArchive,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(ref h) = self.handle.clone() {
            self.use_cases.close(h);
        }
        self.state.update(cx, |state, _| {
            state.clear_archive();
        });
        self.preview_data = None;
        self.preview_loading = false;
        self.show_preview = false;
        self.update_status(cx);
    }

    fn on_action_toggle_search(
        &mut self,
        _: &ToggleSearch,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.propagate();
        if window.has_focused_input(cx) {
            return;
        }

        struct Search;
        let note = Notification::new()
            .message("You have toggled search.")
            .id::<Search>();
        window.push_notification(note, cx);
    }

    fn on_action_add_files(
        &mut self,
        _: &RequestAddFiles,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(ref h) = self.handle.clone() {
            let uc = self.use_cases.clone();
            let handle = h.clone();
            cx.spawn(async move |_, cx| {
                bit7z_pres_dialogs::add_files::AddFilesDialog::open(
                    cx,
                    bit7z_domain::archive::ArchiveFormat::SevenZip,
                    Some(handle),
                    Some(uc.repo.clone()),
                    false,
                );
            })
            .detach();
        }
    }

    fn on_action_checksum(
        &mut self,
        action: &Checksum,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(ref h) = self.handle.clone() {
            let indices = self.state.read(cx).selected_indices();
            if !indices.is_empty() {
                let handle = h.clone();
                let uc = self.use_cases.clone();
                let idxs = indices.clone();
                cx.spawn(async move |_, cx| {
                    bit7z_pres_dialogs::checksum::ChecksumDialog::open_with_entries(
                        cx,
                        handle,
                        idxs,
                        uc.repo.clone(),
                    );
                })
                .detach();
            }
        }
    }

    fn handle_open_archive(
        &mut self,
        path: &PathBuf,
        password: Option<Password>,
        cx: &mut Context<Self>,
    ) {
        self.state.update(cx, |state, _| {
            state.status = ViewStatus::Loading;
        });
        // self.left_status = self.state.read(cx).status_text().into();

        let path_buf = path.clone();
        let uc = self.use_cases.clone();
        let pw = password.clone();
        let state = self.state.clone();
        let this = cx.entity().downgrade();

        let bg_uc = uc.clone();
        let bg_path = path_buf.clone();
        let bg_task = cx.background_spawn(async move { bg_uc.repo.open(&bg_path, pw.as_ref()) });

        cx.spawn(async move |_this, cx| match bg_task.await {
            Ok(handle) => {
                this.update(cx, |this, cx| {
                    this.handle = Some(handle.clone());
                    this.archive_password = password;
                    this.state.update(cx, |s, _| {
                        s.current_path = String::new();
                        s.path_history.clear();
                        s.directory_cache.clear();
                        s.status = ViewStatus::Loading;
                    });
                    this.load_current_directory(cx);
                    cx.notify();
                })
                .ok();
            }
            Err(ArchiveError::EncryptedArchiveRequiresPassword) => {
                state.update(cx, |state, _| {
                    state.status = ViewStatus::Empty;
                });
                Self::password_prompt(&path_buf, &state, &uc, this.clone(), cx);
            }
            Err(e) => {
                state.update(cx, |state, _| {
                    state.status = ViewStatus::Error(e.to_string());
                });
            }
        })
        .detach();
    }

    pub fn password_prompt(
        path: &PathBuf,
        state: &Entity<FileListState>,
        uc: &Arc<UseCases>,
        this: WeakEntity<Self>,
        cx: &mut AsyncApp,
    ) {
        use crossbeam_channel::TryRecvError;
        let path_buf = path.clone();
        let state = state.clone();
        let uc = uc.clone();
        let rx = PasswordDialog::open(path.file_name().unwrap().to_string_lossy().to_string(), cx);
        cx.spawn(async move |cx| {
            loop {
                match rx.try_recv() {
                    Ok(result) => {
                        if let PasswordResult::Submitted(pw) = result {
                            match uc.repo.open(&path_buf, Some(&pw)) {
                                Ok(handle) => {
                                    this.update(cx, |this, _| {
                                        this.archive_password = Some(pw.clone());
                                    })
                                    .ok();
                                    state.update(cx, |state, _| {
                                        state.current_path = String::new();
                                        state.path_history.clear();
                                        state.directory_cache.clear();
                                        state.status = ViewStatus::Loading;
                                    });
                                    Self::load_directory_async(
                                        &state,
                                        &uc,
                                        &handle,
                                        String::new(),
                                        cx,
                                    );
                                }
                                Err(ArchiveError::EncryptedArchiveRequiresPassword) => {
                                    state.update(cx, |state, _| {
                                        state.status = ViewStatus::Empty;
                                    });
                                }
                                Err(e) => {
                                    state.update(cx, |state, _| {
                                        state.status = ViewStatus::Error(e.to_string());
                                    });
                                }
                            }
                        }
                        break;
                    }
                    Err(TryRecvError::Empty) => {
                        cx.background_spawn(std::future::ready(())).await;
                    }
                    Err(TryRecvError::Disconnected) => break,
                }
            }
        })
        .detach();
    }

    fn load_directory_async(
        state: &Entity<FileListState>,
        uc: &Arc<UseCases>,
        handle: &ArchiveHandle,
        current_path: String,
        cx: &mut AsyncApp,
    ) {
        let h = handle.clone();
        let state = state.clone();
        let uc = uc.clone();
        let bg_task = cx.background_spawn(async move { uc.list_directory(&h, &current_path) });
        cx.spawn(async move |cx| match bg_task.await {
            Ok(entries) => {
                state.update(cx, |state, cx| {
                    state.populate_directory(entries, cx);
                    cx.notify();
                });
            }
            Err(e) => {
                state.update(cx, |state, _| {
                    state.status = ViewStatus::Error(e.to_string());
                });
            }
        })
        .detach();
    }

    fn load_current_directory(&mut self, cx: &mut Context<Self>) {
        let handle = self.handle.clone();
        let current_path = self.state.read(cx).current_path.clone();
        log::info!(
            "load_current_directory: path='{}', has_handle={}",
            current_path,
            handle.is_some()
        );
        let Some(h) = handle else { return };

        let state = self.state.clone();
        let uc = self.use_cases.clone();
        let bg_task = cx.background_spawn(async move {
            log::info!("background: list_directory path='{}'", current_path);
            uc.list_directory(&h, &current_path)
        });
        cx.spawn(async move |_this, cx| match bg_task.await {
            Ok(entries) => {
                log::info!("list_directory OK: {} entries", entries.len());
                state.update(cx, |state, cx| {
                    state.populate_directory(entries, cx);
                    cx.notify();
                });
            }
            Err(e) => {
                log::error!("list_directory failed: {:?}", e);
                state.update(cx, |state, _| {
                    state.status = crate::file_list::ViewStatus::Error(format!("{:?}", e));
                });
            }
        })
        .detach();
    }

    fn update_subdirs(&mut self, cx: &mut Context<Self>) {
        self.subdirs = self.state.read(cx).filtered_subdirs();
    }

    fn update_status(&mut self, cx: &mut Context<Self>) {
        let state = self.state.read(cx);
        self.left_status = state.status_text().into();
        self.right_status = self
            .handle
            .as_ref()
            .and_then(|h| h.path())
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default()
            .into();
    }

    pub fn view(
        init_path: Option<PathBuf>,
        init_password: Option<Password>,
        repo: Arc<dyn ArchiveRepository>,
        window: &mut Window,
        cx: &mut App,
    ) -> Entity<Self> {
        cx.new(|cx| Self::new(init_path, init_password, repo, window, cx))
    }
}

impl Render for ArchiveFileManager {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let sidebar_dirs: Vec<SidebarMenuItem> = self
            .subdirs
            .iter()
            .map(|name| {
                let h = cx.entity().downgrade();
                let n = name.clone();
                SidebarMenuItem::new(name.clone())
                    .icon(IconName::Folder)
                    .on_click(move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
                        if let Some(fm) = h.upgrade() {
                            fm.update(cx, |this, cx| {
                                this.state.update(cx, |state, cx| {
                                    state.navigate_into(&n);
                                });
                                this.load_current_directory(cx);
                            });
                        }
                    })
            })
            .collect::<Vec<_>>();

        let sidebar = Sidebar::new("archive-browser")
            .w(relative(1.))
            .collapsible(true)
            .collapsible(SidebarCollapsible::Offcanvas)
            .collapsed(self.collapsed)
            .child(
                SidebarGroup::new("Explorer").child(
                    SidebarMenu::new().child(
                        SidebarMenuItem::new("Folder")
                            .icon(IconName::Folder)
                            .children(sidebar_dirs),
                    ),
                ),
            );

        let body = h_resizable("file-manager-container")
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
                v_resizable("main-vt")
                    .child(
                        resizable_panel().child(
                            v_flex()
                                .flex_1()
                                .child(
                                    Toolbar::new().w(relative(1.)).child(
                                        ToolbarButton::new("open-archive-btn")
                                            .large()
                                            .vertical()
                                            .icon(IconName::FolderOpen)
                                            .label("Open Archive")
                                            .on_click(|_, window, cx| {
                                                // let actions: Vec<_> = window
                                                //     .available_actions(cx)
                                                //     .iter()
                                                //     .map(|action| action.name().to_string())
                                                //     .collect();

                                                window.dispatch_action(Box::new(OpenArchive()), cx);
                                                // show_info(
                                                //     window,
                                                //     cx,
                                                //     format!("actions：{:?}", actions).as_str(),
                                                // )
                                            }),
                                    ),
                                )
                                .child(ArchiveFileList::new(&self.state)),
                        ),
                    )
                    .child(
                        resizable_panel()
                            .visible(self.show_preview)
                            .size(px(200.))
                            .size_range(px(100.)..px(500.))
                            .child(
                                PreviewPanel::new()
                                    .with_data(
                                        self.preview_data.as_ref().map(|rc| rc.as_ref().clone()),
                                    )
                                    .with_loading(self.preview_loading),
                            ),
                    ),
            );

        v_flex()
            .size_full()
            // .id("archive_file_manager")
            .on_action(cx.listener(Self::on_action_open_archive))
            .on_action(cx.listener(Self::on_action_create_archive))
            .on_action(cx.listener(Self::on_action_close_archive))
            .on_action(cx.listener(Self::on_action_add_files))
            .on_action(cx.listener(Self::on_action_checksum))
            .on_action(cx.listener(|this, _: &OpenEntry, _, cx| {
                this.handle_open_entry(cx);
            }))
            // .on_action(cx.listener(|this, _: &NavigateUp, _, cx| {
            //     this.state.update(cx, |state, cx| {
            //         state.navigate_up();
            //     });
            //     this.load_current_directory(cx);
            // }))
            // .on_action(cx.listener(|this, _: &ExtractSelected, _, cx| {
            //     this.handle_extract(cx);
            // }))
            .on_action(cx.listener(|this, _: &DeleteSelected, _, cx| {
                this.handle_delete(cx);
            }))
            .on_action(cx.listener(|this, _: &TestSelected, _, cx| {
                this.handle_test(true, cx);
            }))
            .on_action(cx.listener(|this, _: &TestAll, _, cx| {
                this.handle_test(false, cx);
            }))
            .on_action(cx.listener(|this, _: &ShowProperties, _, cx| {
                this.handle_properties(cx);
            }))
            .on_action(cx.listener(|this, _: &Refresh, _, cx| {
                this.state.update(cx, |state, _| {
                    state.directory_cache.remove(&state.current_path);
                });
                this.load_current_directory(cx);
            }))
            // .on_action(cx.listener(|this, _: &SelectAll, _, cx| {
            //     this.state.update(cx, |state, cx| {
            //         state.select_all();
            //         cx.emit(FileListEvent::SelectionChanged(
            //             state
            //                 .level_entries
            //                 .iter()
            //                 .map(|e| e.original_index)
            //                 .collect(),
            //         ));
            //     });
            // }))
            // .on_action(cx.listener(|this, _: &ClearSelection, _, cx| {
            //     this.state.update(cx, |state, cx| {
            //         state.clear_selection();
            //         cx.emit(FileListEvent::SelectionChanged(vec![]));
            //     });
            // }))
            // .on_action(cx.listener(|this, _: &PreviewEntry, _, _| {
            //     // Preview is already shown on selection; no-op.
            // }))
            // .on_action(cx.listener(|this, _: &RenameEntry, _, _| {
            //     // TODO: implement inline rename
            // }))
            .on_action(cx.listener(|this, action: &Checksum, _, cx| {
                // cx.dispatch_action(&Checksum(ChecksumAlgorithm::Crc32));
            }))
            .on_action(cx.listener(|this, _: &RequestNewFolder, _, cx| {
                // TODO: implement new folder dialog
            }))
            .on_action(cx.listener(|this, _: &RequestNewFile, _, cx| {
                if let Some(ref h) = this.handle.clone() {
                    let uc = this.use_cases.clone();
                    let handle = h.clone();
                    cx.background_spawn(async move {
                        let _ = uc.new_file(&handle);
                    })
                    .detach();
                }
            }))
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
