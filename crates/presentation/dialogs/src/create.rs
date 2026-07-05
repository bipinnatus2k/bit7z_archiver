use bit7z_domain::archive::*;
use bit7z_pres_theme::Theme;
use crossbeam_channel::{Receiver, Sender, unbounded};
use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::Disableable;
use gpui_component::IndexPath;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::checkbox::Checkbox;
use gpui_component::collapsible::Collapsible;
use gpui_component::input::{Input, InputState};
use gpui_component::select::{SearchableVec, Select, SelectItem, SelectState};
use gpui_component::{IconName, h_flex, v_flex};
use std::os::windows::fs::MetadataExt;
use std::sync::{Arc, Mutex};

type SharedSender<T> = Arc<Mutex<Option<Sender<T>>>>;

pub struct CreateArchiveDialog {
    pub file_list: Vec<CreateFileItem>,
    pub format: ArchiveFormat,
    pub compression_level: u8,
    pub password: String,
    pub password_confirm: String,
    pub show_password: bool,
    pub encrypt_filenames: bool,
    pub destination: String,
    pub show_advanced: bool,
    pub compression_method: String,
    pub dictionary_size: String,
    pub word_size: String,
    pub solid: bool,
    pub volume_size: String,
    pub thread_count: Entity<InputState>,
    password_state: Entity<InputState>,
    password_confirm_state: Entity<InputState>,
    format_select: Entity<SelectState<Vec<FormatItem>>>,
    level_select: Entity<SelectState<SearchableVec<LevelItem>>>,
}

#[derive(Debug, Clone)]
pub enum CreateDialogEvent {
    CreateRequested(CreateDialogInput),
    CreateCompleted {
        success: bool,
        error: Option<String>,
    },
    Canceled,
}

#[derive(Debug, Clone)]
pub struct CreateDialogInput {
    pub files: Vec<std::path::PathBuf>,
    pub destination: std::path::PathBuf,
    pub format: ArchiveFormat,
    pub compression_level: u8,
    pub encryption: Option<EncryptionConfig>,
}

impl EventEmitter<CreateDialogEvent> for CreateArchiveDialog {}

#[derive(Debug, Clone)]
struct FormatItem {
    format: ArchiveFormat,
    label: SharedString,
}

impl SelectItem for FormatItem {
    type Value = ArchiveFormat;
    fn title(&self) -> SharedString {
        self.label.clone()
    }
    fn value(&self) -> &Self::Value {
        &self.format
    }
}

#[derive(Debug, Clone)]
struct LevelItem {
    level: u8,
    label: SharedString,
}

impl SelectItem for LevelItem {
    type Value = u8;
    fn title(&self) -> SharedString {
        self.label.clone()
    }
    fn value(&self) -> &Self::Value {
        &self.level
    }
}

fn writable_formats() -> Vec<FormatItem> {
    vec![
        FormatItem { format: ArchiveFormat::SevenZip, label: "7Z".into() },
        FormatItem { format: ArchiveFormat::Zip, label: "ZIP".into() },
        FormatItem { format: ArchiveFormat::Tar, label: "TAR".into() },
        FormatItem { format: ArchiveFormat::TarGz, label: "TAR.GZ".into() },
        FormatItem { format: ArchiveFormat::TarBz2, label: "TAR.BZ2".into() },
        FormatItem { format: ArchiveFormat::TarXz, label: "TAR.XZ".into() },
    ]
}

fn compression_levels() -> Vec<LevelItem> {
    vec![
        LevelItem { level: 0, label: "None".into() },
        LevelItem { level: 1, label: "Fastest".into() },
        LevelItem { level: 2, label: "Fast".into() },
        LevelItem { level: 3, label: "Normal".into() },
        LevelItem { level: 4, label: "Maximum".into() },
        LevelItem { level: 5, label: "Ultra".into() },
    ]
}

impl CreateArchiveDialog {
    pub fn new(window: &mut Window, cx: &mut Context<Self>, files: Vec<CreateFileItem>) -> Self {
        let prefs = &cx.global::<bit7z_rt_globals::PreferencesGlobal>().0;
        let default_format = prefs.archive.default_format;
        let compression_level = prefs.archive.default_compression_level;
        let encrypt_filenames = prefs.archive.default_encrypt_filenames;
        let dest = if files.len() == 1 {
            let stem = files[0]
                .path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            format!("{}.{}", stem, default_format.extension())
        } else {
            format!("archive.{}", default_format.extension())
        };

        let password_state = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Optional")
                .masked(true)
        });
        let password_confirm_state = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Confirm password")
                .masked(true)
        });

        let format_select = cx.new(|cx| {
            SelectState::new(writable_formats(), Some(IndexPath::default()), window, cx)
        });

        cx.subscribe_in(
            &format_select,
            window,
            |this, _state, event, _window, cx| {
                if let gpui_component::select::SelectEvent::Confirm(Some(fmt)) = event {
                    this.format = *fmt;
                    let ext = fmt.extension();
                    if let Some(stem) = std::path::Path::new(&this.destination)
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                    {
                        this.destination = format!("{}.{}", stem, ext);
                    }
                    cx.notify();
                }
            },
        )
        .detach();

        let levels = compression_levels();
        let level_select = cx.new(|cx| {
            let items: SearchableVec<LevelItem> = levels.into();
            SelectState::new(items, Some(IndexPath::default()), window, cx)
        });
        cx.subscribe_in(
            &level_select,
            window,
            |this,
             _state,
             event: &gpui_component::select::SelectEvent<SearchableVec<LevelItem>>,
             _window,
             cx| {
                if let gpui_component::select::SelectEvent::Confirm(Some(level)) = event {
                    this.compression_level = *level;
                    cx.notify();
                }
            },
        )
        .detach();

        let thread_count = "";
        let thread_input = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value("")
                .placeholder(if thread_count.is_empty() {
                    "auto".to_string()
                } else {
                    thread_count.to_string()
                })
        });

        Self {
            file_list: files,
            format: default_format,
            compression_level,
            password: String::new(),
            password_confirm: String::new(),
            show_password: false,
            encrypt_filenames,
            destination: dest,
            show_advanced: false,
            compression_method: String::new(),
            dictionary_size: String::new(),
            word_size: String::new(),
            solid: false,
            volume_size: String::new(),
            thread_count: thread_input,
            password_state,
            password_confirm_state,
            format_select,
            level_select,
        }
    }

    fn is_valid(&self) -> bool {
        if self.file_list.is_empty() || self.destination.is_empty() {
            return false;
        }
        if !self.password.is_empty() && self.password != self.password_confirm {
            return false;
        }
        true
    }

    fn build_encryption(&self) -> Option<EncryptionConfig> {
        if self.password.is_empty() {
            return None;
        }
        Some(EncryptionConfig {
            password: Password::new(self.password.clone()),
            method: EncryptionMethod::Aes256,
            encrypt_filenames: self.encrypt_filenames && self.format.supports_encrypted_filenames(),
        })
    }

    pub fn open(cx: &mut AsyncApp, files: Vec<CreateFileItem>) -> Receiver<CreateDialogEvent> {
        let (tx, rx) = unbounded::<CreateDialogEvent>();
        let event_tx: SharedSender<CreateDialogEvent> = Arc::new(Mutex::new(Some(tx)));
        let et = event_tx.clone();
        cx.spawn(async move |cx| {
            let _ = cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                        point(px(100.), px(100.)),
                        size(px(560.), px(600.)),
                    ))),
                    window_background: WindowBackgroundAppearance::Opaque,
                    window_decorations: Some(WindowDecorations::Client),
                    focus: true,
                    ..Default::default()
                },
                move |window, cx| {
                    let dialog = cx.new(|cx| CreateArchiveDialog::new(window, cx, files));
                    let et = et.clone();
                    cx.subscribe::<CreateArchiveDialog, CreateDialogEvent>(
                        &dialog,
                        move |_, evt: &CreateDialogEvent, _| {
                            if let Ok(guard) = et.lock() {
                                if let Some(ref sender) = *guard {
                                    let _ = sender.send(evt.clone());
                                }
                            }
                        },
                    )
                    .detach();
                    cx.new(|cx| gpui_component::Root::new(dialog, window, cx))
                },
            );
        })
        .detach();
        rx
    }
}

impl Render for CreateArchiveDialog {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        v_flex()
            .gap_3().p_4().w(relative(1.))
            .child(div().font_weight(FontWeight::BOLD).text_lg().child("Create Archive"))
            .child(
                div().border_1().border_color(theme.border).rounded_md().h(px(120.)).p_2()
                    .children(if self.file_list.is_empty() {
                        vec![div().text_color(theme.muted).child("Drop files here").into_any()]
                    } else {
                        self.file_list.iter().map(|f| {
                            div().px_2().py_1().child(f.display_name()).into_any()
                        }).collect()
                    })
            )
            .child(
                h_flex().gap_2()
                    .child(
                        Button::new("add-files").icon(IconName::Plus).label("Add Files")
                            .on_click(cx.listener(|this: &mut CreateArchiveDialog, _event: &ClickEvent, _window: &mut Window, cx| {
                                if let Some(path) = bit7z_infra_platform::pick_files() {
                                    let _ = path.iter().map(|i| {
                                        this.file_list.push(bit7z_domain::archive::CreateFileItem {
                                            path: i.clone(),
                                            is_directory: false,
                                            size: Some(i.metadata().unwrap().file_size()),
                                        });
                                    });
                                    if this.destination.is_empty() {
                                        let stem = path.last().unwrap().file_stem()
                                            .map(|s| s.to_string_lossy().to_string())
                                            .unwrap_or_default();
                                        this.destination = format!("{}.{}", stem, this.format.extension());
                                    }
                                    cx.notify();
                                }
                            })
                            ),
                    )
                    .child(
                        Button::new("add-folder")
                            .icon(IconName::FolderOpen)
                            .label("Add Folder")
                            .on_click(cx.listener(|this, _event: &ClickEvent, _window: &mut Window, cx| {
                                if let Some(path) = bit7z_infra_platform::pick_folder() {
                                    this.file_list.push(bit7z_domain::archive::CreateFileItem {
                                        path: path.clone(),
                                        is_directory: true,
                                        size: None,
                                    });
                                    if this.destination.is_empty() {
                                        let path = path.clone();
                                        let stem = path.file_stem()
                                            .map(|s| s.to_string_lossy().to_string())
                                            .unwrap_or_default();
                                        this.destination = format!("{}.{}", stem, this.format.extension());
                                    }
                                    cx.notify();
                                }
                            }))
                    )
            )
            .child(div().text_sm().font_weight(FontWeight::BOLD).child("Destination"))
            .child(
                div().px_2().py_1().border_1().border_color(theme.border).rounded_md().child(self.destination.clone())
            )
            .child(
                h_flex()
                    .w_full()
                    .justify_between()
                    .gap_2()
                    .child(Select::new(&self.format_select).title_prefix("Format: "))
                .child(Select::new(&self.level_select).title_prefix("Compression Level: ").disabled(!self.format.supports_compression_level()))
            )
            .child(
                v_flex().gap_1()
                    .child(
                        Collapsible::new()
                            .w_full()
                            .open(self.show_advanced)
                            .child(
                                h_flex().justify_center().child(
                                    Button::new("toggle_advanced")
                                        .icon(IconName::ChevronDown)
                                        .label("Show more")
                                        .when(self.show_advanced, |this| {
                                            this.icon(IconName::ChevronUp).label("Show less")
                                        })
                                        .link()
                                        .on_click({
                                            cx.listener(move |this, _, _, cx| {
                                                this.show_advanced = !this.show_advanced;
                                                cx.notify();
                                            })
                                        }),
                                ),
                            )
                            .content(
                                v_flex().gap_2().pl_4().pt_1()
                                    .child(
                                        h_flex()
                                            .justify_between()
                                            .when(self.format == ArchiveFormat::SevenZip, |el| {
                                                el.child(h_flex().gap_2()
                                                    .child(div().text_sm().child("Method:"))
                                                    .child(div().px_2().py_1().border_1().border_color(theme.border).rounded_md().w(px(120.)).child(
                                                        if self.compression_method.is_empty() { "LZMA2".to_string() } else { self.compression_method.clone() }
                                                    ))
                                                )
                                            })
                                            .when(self.format == ArchiveFormat::SevenZip || self.format == ArchiveFormat::Zip, |el| {
                                                el.child(h_flex().gap_2()
                                                    .child(div().text_sm().child("Dictionary:"))
                                                    .child(div().px_2().py_1().border_1().border_color(theme.border).rounded_md().w(px(100.)).child(
                                                        if self.dictionary_size.is_empty() { "64 MB".to_string() } else { self.dictionary_size.clone() }
                                                    ))
                                                )
                                            })
                                            .when(self.format == ArchiveFormat::SevenZip, |el| {
                                                el.child(h_flex().gap_2()
                                                    .child(div().text_sm().child("Word size:"))
                                                    .child(div().px_2().py_1().border_1().border_color(theme.border).rounded_md().w(px(80.)).child(
                                                        if self.word_size.is_empty() { "64".to_string() } else { self.word_size.clone() }
                                                    ))
                                                )
                                            })
                                    )
                                    .when(self.format == ArchiveFormat::SevenZip, |el| {
                                        el.child(
                                            Checkbox::new("solid-checkbox")
                                                .checked(self.solid)
                                                .label("Solid archive")
                                                .on_click(cx.listener(|this, _e, _window, cx| {
                                                    this.solid = !this.solid;
                                                    cx.notify();
                                                }))
                                        )
                                    })
                                    .child(h_flex().gap_2()
                                        .child(div().text_sm().child("Volume size:"))
                                        .child(div().px_2().py_1().border_1().border_color(theme.border).rounded_md().w(px(100.)).child(
                                            if self.volume_size.is_empty() { "\u{2014}".to_string() } else { self.volume_size.clone() }
                                        ))
                                    )
                                    .child(h_flex().gap_2()
                                        .child(div().text_sm().child("Threads:"))
                                        .child(Input::new(&self.thread_count).border_color(theme.border).rounded_md().w(px(80.)))
                                    )
                            )
                    )
            )
            .child(
                v_flex().gap_1()
                    .child(div().text_sm().child("Password (optional)"))
                    .child(
                        Input::new(&self.password_state).mask_toggle().flex_1()
                    )
                    .child(div().text_sm().child("Confirm"))
                    .child(Input::new(&self.password_confirm_state).mask_toggle())
                    .when(self.format.supports_encrypted_filenames(), |el| {
                        el.child(
                            Checkbox::new("encrypt-filenames")
                                .checked(self.encrypt_filenames)
                                .label("Encrypt filenames")
                                .on_click(cx.listener(|this, _e, _window, cx| {
                                    this.encrypt_filenames = !this.encrypt_filenames;
                                    cx.notify();
                                }))
                        )
                    })
            )
            .child(
                h_flex().justify_end().gap_2().pt_2()
                    .child(
                        Button::new("cancel").label("Cancel")
                            .on_click(cx.listener(|_this, _e, _window, cx| cx.emit(CreateDialogEvent::Canceled)))
                    )
                    .child(
                        Button::new("create")
                            .label(if self.password.is_empty() { "Create" } else { "Create Encrypted" })
                            .primary()
                            .disabled(!self.is_valid())
                            .on_click(cx.listener(|this, _e, _window, cx| {
                                let repo = cx.global::<bit7z_rt_globals::RepoGlobal>().0.clone();
                                let dest = std::path::PathBuf::from(&this.destination);
                                let format = this.format;
                                let encryption = this.build_encryption();
                                let files: Vec<std::path::PathBuf> = this.file_list.iter().map(|f| f.path.clone()).collect();
                                let (tx, rx) = bit7z_infra_progress::progress_channel();
                                cx.update_global::<bit7z_pres_view_models::progress_vm::ProgressState, _>(|state, _cx| {
                                    state.is_active = true;
                                    state.is_complete = false;
                                    state.is_paused = false;
                                    state.receiver = Some(std::sync::Arc::new(std::sync::Mutex::new(rx)));
                                    state.message = "Creating archive...".to_string();
                                    state.current = 0;
                                    state.total = 1;
                                    state.error = None;
                                });
                                let dialog_entity = cx.entity();
                                cx.background_spawn(async move {
                                    let mut handle = match repo.create(&dest, format, encryption.as_ref()) {
                                        Ok(h) => h,
                                        Err(e) => {
                                            let _ = tx.send(bit7z_domain::repository::ProgressUpdate {
                                                file_current: 0, file_total: 0,
                                                current_file: None,
                                                items_done: 0, items_total: 0,
                                                bytes_done: 0, bytes_total: 0,
                                                error: Some(e.to_string()),
                                            });
                                            return;
                                        }
                                    };
                                    if !files.is_empty() {
                                        let uc = bit7z_app_archive::add_to::AddToArchiveUseCase::new(repo.clone());
                                        let notifier: Option<Arc<dyn bit7z_domain::repository::ProgressNotifier>> = Some(Arc::new(bit7z_infra_progress::CrossbeamNotifier(tx)));
                                        let _ = uc.execute(&mut handle, &files, notifier);
                                    } else {
                                        drop(tx);
                                    }
                                }).detach();
                                cx.spawn(async move |_, cx| {
                                    loop {
                                        let done = cx.update_global::<bit7z_pres_view_models::progress_vm::ProgressState, _>(|state, _| {
                                            let _ = state.poll();
                                            state.is_complete
                                        });
                                        if done {
                                            break;
                                        }
                                        cx.background_spawn(async move { std::thread::sleep(std::time::Duration::from_millis(80)); }).await;
                                    }
                                    let error = cx.update_global::<bit7z_pres_view_models::progress_vm::ProgressState, _>(|state, _| state.error.clone());
                                    dialog_entity.update(cx, |_, cx| {
                                        cx.emit(CreateDialogEvent::CreateCompleted { success: error.is_none(), error });
                                    });
                                }).detach();
                            }))
                    )
            )
    }
}
