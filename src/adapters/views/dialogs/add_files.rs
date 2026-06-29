use crate::domain::archive::*;
use crate::theme::Theme;
use crossbeam::channel::{unbounded, Receiver, Sender};
use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::form::{field, v_form};
use gpui_component::input::{Input, InputState};
use gpui_component::select::{Select, SelectItem, SelectState, SearchableVec};
use gpui_component::Disableable;
use gpui_component::IndexPath;
use gpui_component::{h_flex, v_flex};
use std::sync::{Arc, Mutex};

type SharedSender<T> = Arc<Mutex<Option<Sender<T>>>>;

pub struct AddFilesDialog {
    pub format: ArchiveFormat,
    pub file_list: Vec<std::path::PathBuf>,
    pub wildcard_filter: String,
    pub filter_policy: FilterPolicy,
    pub recursive: bool,
    pub archive_path_prefix: String,
    pub compression_level: u8,
    pub compression_method: String,
    pub dictionary_size: String,
    pub word_size: String,
    pub solid: bool,
    pub volume_size: String,
    pub thread_count: String,
    pub show_advanced: bool,
    pub password: String,
    pub password_confirm: String,
    pub encrypt_filenames: bool,
    pub store_timestamps: bool,
    filter_input: Option<Entity<InputState>>,
    prefix_input: Option<Entity<InputState>>,
    password_input: Option<Entity<InputState>>,
    password_confirm_input: Option<Entity<InputState>>,
    format_select: Option<Entity<SelectState<SearchableVec<FormatItem>>>>,
    level_select: Option<Entity<SelectState<SearchableVec<LevelItem>>>>,
    archive: Option<ArchiveHandle>,
    repo: Option<std::sync::Arc<dyn crate::domain::repository::ArchiveRepository>>,
    is_solid: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FilterPolicy {
    Include,
    Exclude,
}

impl FilterPolicy {
    fn label(&self) -> &'static str {
        match self {
            FilterPolicy::Include => "Include",
            FilterPolicy::Exclude => "Exclude",
        }
    }
}

#[derive(Debug, Clone)]
pub enum AddFilesDialogEvent {
    AddRequested {
        files: Vec<std::path::PathBuf>,
        format: ArchiveFormat,
        compression_level: u8,
        encryption: Option<EncryptionConfig>,
    },
    Canceled,
}

impl EventEmitter<AddFilesDialogEvent> for AddFilesDialog {}

#[derive(Debug, Clone)]
struct FormatItem {
    format: ArchiveFormat,
    label: SharedString,
}

impl SelectItem for FormatItem {
    type Value = ArchiveFormat;
    fn title(&self) -> SharedString { self.label.clone() }
    fn value(&self) -> &Self::Value { &self.format }
}

#[derive(Debug, Clone)]
struct LevelItem {
    level: u8,
    label: SharedString,
}

impl SelectItem for LevelItem {
    type Value = u8;
    fn title(&self) -> SharedString { self.label.clone() }
    fn value(&self) -> &Self::Value { &self.level }
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

impl AddFilesDialog {
    pub fn new(
        cx: &mut Context<Self>,
        format: ArchiveFormat,
        archive: Option<ArchiveHandle>,
        repo: Option<std::sync::Arc<dyn crate::domain::repository::ArchiveRepository>>,
        is_solid: bool,
    ) -> Self {
        let prefs = &cx.global::<crate::gui::PreferencesGlobal>().0;
        let compression_level = prefs.archive.default_compression_level;
        let encrypt_filenames = prefs.archive.default_encrypt_filenames;
        Self {
            format,
            file_list: Vec::new(),
            wildcard_filter: String::new(),
            filter_policy: FilterPolicy::Exclude,
            recursive: true,
            archive_path_prefix: String::new(),
            compression_level,
            compression_method: String::new(),
            dictionary_size: String::new(),
            word_size: String::new(),
            solid: false,
            volume_size: String::new(),
            thread_count: String::new(),
            show_advanced: false,
            password: String::new(),
            password_confirm: String::new(),
            encrypt_filenames,
            store_timestamps: true,
            filter_input: None,
            prefix_input: None,
            password_input: None,
            password_confirm_input: None,
            format_select: None,
            level_select: None,
            archive,
            repo,
            is_solid,
        }
    }

    fn is_existing_archive(&self) -> bool {
        self.archive.is_some()
    }

    fn ensure_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.filter_input.is_none() {
            self.filter_input = Some(cx.new(|cx| InputState::new(window, cx).placeholder("e.g. *.tmp")));
        }
        if self.prefix_input.is_none() {
            self.prefix_input = Some(cx.new(|cx| InputState::new(window, cx).placeholder("e.g. subdir/")));
        }
        if self.password_input.is_none() {
            self.password_input = Some(cx.new(|cx| InputState::new(window, cx).placeholder("Optional").masked(true)));
        }
        if self.password_confirm_input.is_none() {
            self.password_confirm_input = Some(cx.new(|cx| InputState::new(window, cx).placeholder("Confirm").masked(true)));
        }
        if self.format_select.is_none() {
            let formats = writable_formats();
            let selected = formats.iter().position(|f| f.format == self.format);
            self.format_select = Some(cx.new(|cx| {
                let items: SearchableVec<FormatItem> = formats.into();
                SelectState::new(items, selected.map(|i| IndexPath::default().row(i)), window, cx)
            }));
        }
        if self.level_select.is_none() {
            let levels = compression_levels();
            let selected = levels.iter().position(|l| l.level == self.compression_level);
            self.level_select = Some(cx.new(|cx| {
                let items: SearchableVec<LevelItem> = levels.into();
                SelectState::new(items, selected.map(|i| IndexPath::default().row(i)), window, cx)
            }));
        }
    }

    fn is_valid(&self) -> bool {
        if self.file_list.is_empty() { return false; }
        if !self.password.is_empty() && self.password != self.password_confirm { return false; }
        if self.is_existing_archive() && self.is_solid { return false; }
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

    /// Open as independent window. Returns receiver for dialog events.
    pub fn open(
        cx: &mut AsyncApp,
        format: ArchiveFormat,
        archive: Option<ArchiveHandle>,
        repo: Option<Arc<dyn crate::domain::repository::ArchiveRepository>>,
        is_solid: bool,
    ) -> Receiver<AddFilesDialogEvent> {
        let (tx, rx) = unbounded::<AddFilesDialogEvent>();
        let event_tx: SharedSender<AddFilesDialogEvent> = Arc::new(Mutex::new(Some(tx)));
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
                    let dialog = cx.new(|cx| AddFilesDialog::new(cx, format, archive, repo, is_solid));
                    let et = et.clone();
                    cx.subscribe::<AddFilesDialog, AddFilesDialogEvent>(&dialog, move |_, evt: &AddFilesDialogEvent, _| {
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

impl Render for AddFilesDialog {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.ensure_inputs(window, cx);
        let filter_input = self.filter_input.clone().expect("init");
        let prefix_input = self.prefix_input.clone().expect("init");
        let password_input = self.password_input.clone().expect("init");
        let password_confirm_input = self.password_confirm_input.clone().expect("init");
        let format_select = self.format_select.clone().expect("init");
        let level_select = self.level_select.clone().expect("init");

        let is_valid = self.is_valid();
        let file_count = self.file_list.len();
        let has_password = !self.password.is_empty();
        let show_level = self.format.supports_compression_level();
        let show_7z = self.format == ArchiveFormat::SevenZip;
        let show_encrypted_names = self.format.supports_encrypted_filenames();
        let is_existing = self.is_existing_archive();
        let is_solid = self.is_solid;
        let compression_method_display = if self.compression_method.is_empty() { "LZMA2".to_string() } else { self.compression_method.clone() };
        let dictionary_size_display = if self.dictionary_size.is_empty() { "64 MB".to_string() } else { self.dictionary_size.clone() };
        let word_size_display = if self.word_size.is_empty() { "64".to_string() } else { self.word_size.clone() };
        let volume_size_display = if self.volume_size.is_empty() { "—".to_string() } else { self.volume_size.clone() };
        let thread_count_display = if self.thread_count.is_empty() { "auto".to_string() } else { self.thread_count.clone() };

        v_form()
            .p_4()
            .w(px(520.))
            .when(is_existing && is_solid, |el| {
                el.child(
                    field()
                        .label_indent(false)
                        .child(
                            div()
                                .p_3()
                                .rounded_md()
                                .border_1()
                                .border_color(cx.global::<Theme>().error)
                                .bg(cx.global::<Theme>().error.alpha(0.1))
                                .child(div().text_sm().text_color(cx.global::<Theme>().error).child(
                                    "This archive uses solid compression. Files cannot be added to solid archives."
                                ))
                        )
                )
            })
            .child(
                field()
                    .label("Format")
                    .child(Select::new(&format_select).appearance(false).disabled(is_existing))
            )
            .child(
                field()
                    .label("Source")
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                Button::new("add-files")
                                    .label("+ Add Files")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        if let Some(paths) = crate::adapters::platform::pick_files() {
                                            this.file_list.extend(paths);
                                            cx.notify();
                                        }
                                    }))
                            )
                            .child(
                                Button::new("add-folder")
                                    .label("+ Add Folder")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        if let Some(path) = crate::adapters::platform::pick_folder() {
                                            this.file_list.push(path);
                                            cx.notify();
                                        }
                                    }))
                            )
                    )
            )
            .child(
                field()
                    .label("Files")
                    .description(format!("{} file{} selected", file_count, if file_count == 1 { "" } else { "s" }))
                    .child(
                        div()
                            .border_1()
                            .border_color(cx.global::<Theme>().border)
                            .rounded_md()
                            .h(px(80.))
                            .p_2()
                            .overflow_hidden()
                            .child(
                                div().child(
                                    if self.file_list.is_empty() {
                                        div().text_color(cx.global::<Theme>().muted).text_sm().child("No files added").into_any()
                                    } else {
                                        v_flex().gap_px().children(
                                            self.file_list.iter().map(|p| {
                                                div().text_sm().truncate().child(p.to_string_lossy().to_string()).into_any()
                                            }).collect::<Vec<_>>()
                                        ).into_any()
                                    }
                                )
                            )
                    )
            )
            .child(
                field()
                    .label("Filter")
                    .child(
                        h_flex()
                            .gap_2()
                            .child(div().flex_1().child(Input::new(&filter_input)))
                            .child(
                                Button::new("filter-policy")
                                    .label(self.filter_policy.label())
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.filter_policy = match this.filter_policy {
                                            FilterPolicy::Include => FilterPolicy::Exclude,
                                            FilterPolicy::Exclude => FilterPolicy::Include,
                                        };
                                        cx.notify();
                                    }))
                            )
                    )
            )
            .child(
                field()
                    .label("Recursive")
                    .child(
                        Button::new("toggle-recursive")
                            .label(if self.recursive { "Yes" } else { "No" })
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.recursive = !this.recursive;
                                cx.notify();
                            }))
                    )
            )
            .child(
                field()
                    .label("Archive path")
                    .child(Input::new(&prefix_input))
            )
            .when(show_level, |el| {
                el.child(
                    field()
                        .label("Compression level")
                        .child(Select::new(&level_select).appearance(false).disabled(is_existing))
                )
            })
            .when(show_7z, |el| {
                el.child(
                    field()
                        .label("Method")
                        .child(
                            div()
                                .px_2().py_1()
                                .border_1().border_color(cx.global::<Theme>().border)
                                .rounded_md()
                                .opacity(if is_existing { 0.5 } else { 1.0 })
                                .child(compression_method_display)
                        )
                )
            })
            .child(
                field()
                    .label_indent(false)
                    .child(
                        Button::new("toggle-advanced")
                            .label(if self.show_advanced { "▼ Advanced" } else { "▶ Advanced" })
                            .ghost()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.show_advanced = !this.show_advanced;
                                cx.notify();
                            }))
                    )
            )
            .when(self.show_advanced && show_7z, |el| {
                el.child(
                    field().label("Dictionary size").child(
                        div().px_2().py_1().border_1().border_color(cx.global::<Theme>().border).rounded_md().w(px(100.)).child(dictionary_size_display)
                    )
                )
                .child(
                    field().label("Word size").child(
                        div().px_2().py_1().border_1().border_color(cx.global::<Theme>().border).rounded_md().w(px(80.)).child(word_size_display)
                    )
                )
                .child(
                    field().label("Solid").child(
                        Button::new("toggle-solid").label(if self.solid { "Yes" } else { "No" }).on_click(cx.listener(|this, _, _, cx| { this.solid = !this.solid; cx.notify(); }))
                    )
                )
            })
            .when(self.show_advanced, |el| {
                el.child(field().label("Volume size").child(
                    div().px_2().py_1().border_1().border_color(cx.global::<Theme>().border).rounded_md().w(px(100.)).child(volume_size_display)
                ))
                .child(field().label("Threads").child(
                    div().px_2().py_1().border_1().border_color(cx.global::<Theme>().border).rounded_md().w(px(80.)).child(thread_count_display)
                ))
                .child(field().label("Store timestamps").child(
                    Button::new("toggle-timestamps").label(if self.store_timestamps { "Yes" } else { "No" }).on_click(cx.listener(|this, _, _, cx| { this.store_timestamps = !this.store_timestamps; cx.notify(); }))
                ))
            })
            .child(field().label("Password").description("Leave empty for no encryption").child(Input::new(&password_input)))
            .child(field().label("Confirm password").visible(has_password).child(Input::new(&password_confirm_input)))
            .when(show_encrypted_names, |el| {
                el.child(field().label("Encrypt filenames").child(
                    Button::new("toggle-encrypt-filenames").label(if self.encrypt_filenames { "Yes" } else { "No" }).on_click(cx.listener(|this, _, _, cx| { this.encrypt_filenames = !this.encrypt_filenames; cx.notify(); }))
                ))
            })
            .child(
                field().label_indent(false).child(
                    h_flex().justify_end().gap_2()
                        .child(Button::new("cancel").label("Cancel").on_click(cx.listener(|_, _, _, cx| { cx.emit(AddFilesDialogEvent::Canceled); })))
                        .child(
                            Button::new("ok").primary().label("OK").disabled(!is_valid)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    if let (Some(handle), Some(repo)) = (&this.archive, &this.repo) {
                                        let files = this.file_list.clone();
                                        let encryption = this.build_encryption();
                                        let uc = crate::application::add_to::AddToArchiveUseCase::new(repo.clone());
                                        let (tx, rx) = crate::application::progress::progress_channel();
                                        cx.update_global::<crate::adapters::view_models::progress_vm::ProgressState, _>(|state, _cx| {
                                            state.is_active = true;
                                            state.is_complete = false;
                                            state.is_paused = false;
                                            state.receiver = Some(std::sync::Arc::new(std::sync::Mutex::new(rx)));
                                            state.message = format!("Adding {} files...", files.len());
                                            state.current = 0;
                                            state.total = files.len() as u64;
                                            state.error = None;
                                        });
                                        let mut handle = handle.clone();
                                        let pw = encryption.as_ref().map(|e| Password::new(e.password.as_str().to_string()));
                                        cx.background_spawn(async move {
                                            let _ = uc.execute_with_password(&mut handle, &files, Some(tx), pw.as_ref());
                                        }).detach();
                                    }
                                    cx.emit(AddFilesDialogEvent::Canceled);
                                }))
                        )
                )
            )
    }
}
