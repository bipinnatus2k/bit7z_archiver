use crate::domain::archive::*;
use crate::theme::Theme;
use crossbeam::channel::{unbounded, Receiver, Sender};
use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::input::{Input, InputState};
use gpui_component::Disableable;
use gpui_component::Selectable;
use gpui_component::{h_flex, v_flex};
use std::sync::{Arc, Mutex};

type SharedSender<T> = Arc<Mutex<Option<Sender<T>>>>;

pub struct CreateArchiveDialog {
    pub file_list: Vec<CreateFileItem>,
    pub format: ArchiveFormat,
    pub compression_level: u8,
    pub password: String,
    pub password_confirm: String,
    pub encrypt_filenames: bool,
    pub destination: String,
    pub show_format_dropdown: bool,
    pub show_level_dropdown: bool,
    pub show_advanced: bool,
    pub compression_method: String,
    pub dictionary_size: String,
    pub word_size: String,
    pub solid: bool,
    pub volume_size: String,
    pub thread_count: String,
    password_state: Option<Entity<InputState>>,
    password_confirm_state: Option<Entity<InputState>>,
}

#[derive(Debug, Clone)]
pub enum CreateDialogEvent {
    CreateRequested(CreateDialogInput),
    CreateCompleted { success: bool, error: Option<String> },
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

fn writable_formats() -> Vec<ArchiveFormat> {
    vec![
        ArchiveFormat::SevenZip,
        ArchiveFormat::Zip,
        ArchiveFormat::Tar,
        ArchiveFormat::TarGz,
        ArchiveFormat::TarBz2,
        ArchiveFormat::TarXz,
    ]
}

fn compression_level_names(level: u8) -> &'static str {
    match level {
        0 => "None",
        1 => "Fastest",
        2 => "Fast",
        3 => "Normal",
        4 => "Maximum",
        5 => "Ultra",
        _ => "Normal",
    }
}

fn format_label(fmt: ArchiveFormat) -> String {
    fmt.display_name().to_string()
}

impl CreateArchiveDialog {
    pub fn new(cx: &mut Context<Self>, files: Vec<CreateFileItem>) -> Self {
        let prefs = &cx.global::<crate::gui::PreferencesGlobal>().0;
        let default_format = prefs.archive.default_format;
        let compression_level = prefs.archive.default_compression_level;
        let encrypt_filenames = prefs.archive.default_encrypt_filenames;
        let dest = if files.len() == 1 {
            let stem = files[0].path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            format!("{}.{}", stem, default_format.extension())
        } else {
            format!("archive.{}", default_format.extension())
        };
        Self {
            file_list: files,
            format: default_format,
            compression_level,
            password: String::new(),
            password_confirm: String::new(),
            encrypt_filenames,
            destination: dest,
            show_format_dropdown: false,
            show_level_dropdown: false,
            show_advanced: false,
            compression_method: String::new(),
            dictionary_size: String::new(),
            word_size: String::new(),
            solid: false,
            volume_size: String::new(),
            thread_count: String::new(),
            password_state: None,
            password_confirm_state: None,
        }
    }

    fn ensure_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.password_state.is_none() {
            self.password_state = Some(cx.new(|cx| InputState::new(window, cx).placeholder("Optional")));
        }
        if self.password_confirm_state.is_none() {
            self.password_confirm_state = Some(cx.new(|cx| InputState::new(window, cx).placeholder("Confirm password")));
        }
    }

    fn is_valid(&self) -> bool {
        if self.file_list.is_empty() || self.destination.is_empty() { return false; }
        if !self.password.is_empty() && self.password != self.password_confirm { return false; }
        if !self.password.is_empty() && self.password.len() < 4 { return false; }
        true
    }

    fn build_encryption(&self) -> Option<EncryptionConfig> {
        if self.password.is_empty() { return None; }
        Some(EncryptionConfig {
            password: Password::new(self.password.clone()),
            method: EncryptionMethod::Aes256,
            encrypt_filenames: self.encrypt_filenames && self.format.supports_encrypted_filenames(),
        })
    }

    fn format_dropdown(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let current = self.format;
        v_flex().gap_1()
            .child(
                Button::new("format-dropdown")
                    .label(format_label(current))
                    .dropdown_caret(true)
                    .on_click(cx.listener(|this, _e, _window, cx| {
                        this.show_format_dropdown = !this.show_format_dropdown;
                        this.show_level_dropdown = false;
                        cx.notify();
                    }))
            )
            .when(self.show_format_dropdown, |el| {
                el.child(
                    v_flex()
                        .border_1()
                        .border_color(cx.global::<Theme>().border)
                        .rounded_md()
                        .children(writable_formats().into_iter().map(|fmt| {
                            let is_current = fmt == current;
                            let id = format!("format-{}", fmt.extension());
                            Button::new(id)
                                .label(format_label(fmt))
                                .when(is_current, |b| b.selected(true))
                                .ghost()
                                .on_click(cx.listener(move |this, _e, _window, cx| {
                                    this.format = fmt;
                                    this.show_format_dropdown = false;
                                    let ext = fmt.extension();
                                    if let Some(stem) = std::path::Path::new(&this.destination).file_stem()
                                        .map(|s| s.to_string_lossy().to_string())
                                    {
                                        this.destination = format!("{}.{}", stem, ext);
                                    }
                                    cx.notify();
                                }))
                                .into_any_element()
                        }).collect::<Vec<_>>())
                )
            })
    }

    fn level_dropdown(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let current = self.compression_level;
        v_flex().gap_1()
            .child(
                Button::new("level-dropdown")
                    .label(compression_level_names(current))
                    .dropdown_caret(true)
                    .on_click(cx.listener(|this, _e, _window, cx| {
                        this.show_level_dropdown = !this.show_level_dropdown;
                        this.show_format_dropdown = false;
                        cx.notify();
                    }))
            )
            .when(self.show_level_dropdown, |el| {
                el.child(
                    v_flex()
                        .border_1()
                        .border_color(cx.global::<Theme>().border)
                        .rounded_md()
                        .children((0u8..=5).map(|lvl| {
                            let is_current = lvl == current;
                            let id = format!("level-{}", lvl);
                            Button::new(id)
                                .label(format!("{} — {}", lvl, compression_level_names(lvl)))
                                .when(is_current, |b| b.selected(true))
                                .ghost()
                                .on_click(cx.listener(move |this, _e, _window, cx| {
                                    this.compression_level = lvl;
                                    this.show_level_dropdown = false;
                                    cx.notify();
                                }))
                                .into_any_element()
                        }).collect::<Vec<_>>())
                )
            })
    }

    /// Open as independent window. Returns receiver for dialog events.
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
                    ..Default::default()
                },
                move |window, cx| {
                    let dialog = cx.new(|cx| CreateArchiveDialog::new(cx, files));
                    let et = et.clone();
                    cx.subscribe::<CreateArchiveDialog, CreateDialogEvent>(&dialog, move |_, evt: &CreateDialogEvent, _| {
                        if let Ok(guard) = et.lock() {
                            if let Some(ref sender) = *guard {
                                let _ = sender.send(evt.clone());
                            }
                        }
                    }).detach();
                    cx.new(|cx| gpui_component::Root::new(dialog, window, cx))
                },
            );
        }).detach();
        rx
    }
}

impl Render for CreateArchiveDialog {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.ensure_inputs(window, cx);
        let theme = cx.global::<Theme>().clone();
        let password_state = self.password_state.clone().expect("password_state initialized");
        let password_confirm_state = self.password_confirm_state.clone().expect("password_confirm_state initialized");

        v_flex()
            .gap_3().p_4().w(px(520.))
            .child(div().font_weight(FontWeight::BOLD).text_lg().child("Create Archive"))
            // File list area
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
            // Add buttons
            .child(
                h_flex().gap_2()
                    .child(
                        Button::new("add-files").label("+ Add Files")
                            .on_click(cx.listener(|this: &mut CreateArchiveDialog, _event: &ClickEvent, _window: &mut Window, cx| {
                                if let Some(path) = crate::adapters::platform::pick_archive_file() {
                                    this.file_list.push(crate::domain::archive::CreateFileItem {
                                        path: path.clone(),
                                        is_directory: false,
                                        size: None,
                                    });
                                    if this.destination.is_empty() {
                                        let stem = path.file_stem()
                                            .map(|s| s.to_string_lossy().to_string())
                                            .unwrap_or_default();
                                        this.destination = format!("{}.{}", stem, this.format.extension());
                                    }
                                    cx.notify();
                                }
                            }))
                    )
                    .child(
                        Button::new("add-folder").label("+ Add Folder")
                    )
            )
            // Format dropdown
            .child(div().text_sm().font_weight(FontWeight::BOLD).child("Format"))
            .child(self.format_dropdown(cx))
            // Compression level dropdown (only for formats that support it)
            .when(self.format.supports_compression_level(), |el| {
                el.child(div().text_sm().font_weight(FontWeight::BOLD).child("Compression Level"))
                    .child(self.level_dropdown(cx))
            })
            // Advanced section
            .child(
                v_flex().gap_1()
                    .child(
                        Button::new("toggle-advanced")
                            .label(if self.show_advanced { "\u{25BC} Advanced" } else { "\u{25B6} Advanced" })
                            .ghost()
                            .on_click(cx.listener(|this, _e, _window, cx| {
                                this.show_advanced = !this.show_advanced;
                                cx.notify();
                            }))
                    )
                    .when(self.show_advanced, |el| {
                        el.child(v_flex().gap_2().pl_4().pt_1()
                            // Compression method (7z only)
                            .when(self.format == ArchiveFormat::SevenZip, |el| {
                                el.child(h_flex().gap_2()
                                    .child(div().text_sm().child("Method:"))
                                    .child(div().px_2().py_1().border_1().border_color(theme.border).rounded_md().w(px(120.)).child(
                                        if self.compression_method.is_empty() { "LZMA2".to_string() } else { self.compression_method.clone() }
                                    ))
                                )
                            })
                            // Dictionary size
                            .when(self.format == ArchiveFormat::SevenZip || self.format == ArchiveFormat::Zip, |el| {
                                el.child(h_flex().gap_2()
                                    .child(div().text_sm().child("Dictionary:"))
                                    .child(div().px_2().py_1().border_1().border_color(theme.border).rounded_md().w(px(100.)).child(
                                        if self.dictionary_size.is_empty() { "64 MB".to_string() } else { self.dictionary_size.clone() }
                                    ))
                                )
                            })
                            // Word size
                            .when(self.format == ArchiveFormat::SevenZip, |el| {
                                el.child(h_flex().gap_2()
                                    .child(div().text_sm().child("Word size:"))
                                    .child(div().px_2().py_1().border_1().border_color(theme.border).rounded_md().w(px(80.)).child(
                                        if self.word_size.is_empty() { "64".to_string() } else { self.word_size.clone() }
                                    ))
                                )
                            })
                            // Solid (7z only)
                            .when(self.format == ArchiveFormat::SevenZip, |el| {
                                el.child(
                                    Button::new("toggle-solid")
                                        .label(if self.solid { "\u{2611} Solid archive" } else { "\u{2610} Solid archive" })
                                        .ghost()
                                        .on_click(cx.listener(|this, _e, _window, cx| {
                                            this.solid = !this.solid;
                                            cx.notify();
                                        }))
                                )
                            })
                            // Volume size
                            .child(h_flex().gap_2()
                                .child(div().text_sm().child("Volume size:"))
                                .child(div().px_2().py_1().border_1().border_color(theme.border).rounded_md().w(px(100.)).child(
                                    if self.volume_size.is_empty() { "\u{2014}".to_string() } else { self.volume_size.clone() }
                                ))
                            )
                            // Thread count
                            .child(h_flex().gap_2()
                                .child(div().text_sm().child("Threads:"))
                                .child(div().px_2().py_1().border_1().border_color(theme.border).rounded_md().w(px(80.)).child(
                                    if self.thread_count.is_empty() { "auto".to_string() } else { self.thread_count.clone() }
                                ))
                            )
                        )
                    })
            )
            // Password section
            .child(
                v_flex().gap_1()
                    .child(div().text_sm().font_weight(FontWeight::BOLD).child("Password (optional)"))
                    .child(Input::new(&password_state))
                    .child(div().text_sm().font_weight(FontWeight::BOLD).child("Confirm"))
                    .child(Input::new(&password_confirm_state))
                    .when(self.format.supports_encrypted_filenames(), |el| {
                        el.child(
                            Button::new("encrypt-filenames")
                                .label(if self.encrypt_filenames { "\u{2611} Encrypt filenames" } else { "\u{2610} Encrypt filenames" })
                                .ghost()
                                .on_click(cx.listener(|this, _e, _window, cx| {
                                    this.encrypt_filenames = !this.encrypt_filenames;
                                    cx.notify();
                                }))
                        )
                    })
            )
            // Destination
            .child(div().text_sm().font_weight(FontWeight::BOLD).child("Destination"))
            .child(
                div().px_2().py_1().border_1().border_color(theme.border).rounded_md().child(self.destination.clone())
            )
            // Buttons
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
                                let repo = cx.global::<crate::gui::RepoGlobal>().0.clone();
                                let dest = std::path::PathBuf::from(&this.destination);
                                let format = this.format;
                                let encryption = this.build_encryption();
                                let files: Vec<std::path::PathBuf> = this.file_list.iter().map(|f| f.path.clone()).collect();
                                let (tx, rx) = crate::application::progress::progress_channel();
                                cx.update_global::<crate::adapters::view_models::progress_vm::ProgressState, _>(|state, _cx| {
                                    state.is_active = true;
                                    state.is_complete = false;
                                    state.is_paused = false;
                                    state.receiver = Some(std::sync::Arc::new(std::sync::Mutex::new(rx)));
                                    state.message = format!("Creating archive...");
                                    state.current = 0;
                                    state.total = 1;
                                    state.error = None;
                                });
                                let dialog_entity = cx.entity();
                                cx.background_spawn(async move {
                                    let mut handle = match repo.create(&dest, format, encryption.as_ref()) {
                                        Ok(h) => h,
                                        Err(e) => {
                                            let _ = tx.send(crate::application::progress::ProgressUpdate {
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
                                        let uc = crate::application::add_to::AddToArchiveUseCase::new(repo.clone());
                                        let notifier: Option<Arc<dyn crate::domain::repository::ProgressNotifier>> = Some(Arc::new(crate::application::progress::CrossbeamNotifier(tx)));
                                        let _ = uc.execute(&mut handle, &files, notifier);
                                    } else {
                                        drop(tx);
                                    }
                                }).detach();
                                cx.spawn(async move |_, cx| {
                                    loop {
                                        let done = cx.update_global::<crate::adapters::view_models::progress_vm::ProgressState, _>(|state, _| {
                                            let _ = state.poll();
                                            state.is_complete
                                        });
                                        if done {
                                            break;
                                        }
                                        cx.background_spawn(async move { std::thread::sleep(std::time::Duration::from_millis(80)); }).await;
                                    }
                                    let error = cx.update_global::<crate::adapters::view_models::progress_vm::ProgressState, _>(|state, _| state.error.clone());
                                    dialog_entity.update(cx, |_, cx| {
                                        cx.emit(CreateDialogEvent::CreateCompleted { success: error.is_none(), error });
                                    });
                                }).detach();
                            }))
                    )
            )
    }
}
