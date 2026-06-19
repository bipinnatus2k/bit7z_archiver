use crate::domain::archive::*;
use crate::domain::preferences::Preferences;
use crate::theme::Theme;
use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::input::{Input, InputState};

pub struct AddFilesDialog {
    pub format: ArchiveFormat,
    pub show_format_dropdown: bool,
    pub file_list: Vec<std::path::PathBuf>,
    pub wildcard_filter: String,
    pub filter_policy: FilterPolicy,
    pub show_policy_dropdown: bool,
    pub recursive: bool,
    pub archive_path_prefix: String,
    pub compression_level: u8,
    pub show_level_dropdown: bool,
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

fn format_label(fmt: ArchiveFormat) -> String {
    fmt.display_name().to_string()
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

impl AddFilesDialog {
    pub fn new(cx: &mut Context<Self>, format: ArchiveFormat) -> Self {
        let prefs = cx.global::<Preferences>();
        let compression_level = prefs.archive.default_compression_level;
        let encrypt_filenames = prefs.archive.default_encrypt_filenames;
        Self {
            format,
            show_format_dropdown: false,
            file_list: Vec::new(),
            wildcard_filter: String::new(),
            filter_policy: FilterPolicy::Exclude,
            show_policy_dropdown: false,
            recursive: true,
            archive_path_prefix: String::new(),
            compression_level,
            show_level_dropdown: false,
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
        }
    }

    fn ensure_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.filter_input.is_none() {
            self.filter_input = Some(cx.new(|cx| InputState::new(window, cx).placeholder("e.g. *.tmp")));
        }
        if self.prefix_input.is_none() {
            self.prefix_input = Some(cx.new(|cx| InputState::new(window, cx).placeholder("e.g. subdir/")));
        }
        if self.password_input.is_none() {
            self.password_input = Some(cx.new(|cx| InputState::new(window, cx).placeholder("Optional")));
        }
        if self.password_confirm_input.is_none() {
            self.password_confirm_input = Some(cx.new(|cx| InputState::new(window, cx).placeholder("Confirm")));
        }
    }

    fn is_valid(&self) -> bool {
        if self.file_list.is_empty() { return false; }
        if !self.password.is_empty() && self.password != self.password_confirm { return false; }
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
        let theme = cx.global::<Theme>().clone();
        let current = self.format;
        div().flex().flex_col().gap_1()
            .child(
                div().px_2().py_1().border_1().border_color(theme.border).rounded_md().cursor_pointer()
                    .flex().flex_row().justify_between()
                    .child(format_label(current))
                    .child(if self.show_format_dropdown { "\u{25B2}" } else { "\u{25BC}" })
                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _e, _window, cx| {
                        this.show_format_dropdown = !this.show_format_dropdown;
                        cx.notify();
                    }))
            )
            .when(self.show_format_dropdown, |el| {
                el.child(
                    div().border_1().border_color(theme.border).rounded_md().flex().flex_col()
                        .children(writable_formats().into_iter().map(|fmt| {
                            let is_current = fmt == current;
                            div().px_2().py_1().cursor_pointer()
                                .bg(if is_current { theme.selection } else { hsla(0., 0., 0., 0.) })
                                .hover(|mut s| { s.background = Some(theme.hover.into()); s })
                                .child(format_label(fmt))
                                .on_mouse_down(MouseButton::Left, cx.listener(move |this, _e, _window, cx| {
                                    this.format = fmt;
                                    this.show_format_dropdown = false;
                                    cx.notify();
                                }))
                                .into_any_element()
                        }).collect::<Vec<_>>())
                )
            })
    }
}

impl Render for AddFilesDialog {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.ensure_inputs(window, cx);
        let theme = cx.global::<Theme>().clone();
        let filter_input = self.filter_input.clone().expect("filter_input initialized");
        let prefix_input = self.prefix_input.clone().expect("prefix_input initialized");
        let password_input = self.password_input.clone().expect("password_input initialized");
        let password_confirm_input = self.password_confirm_input.clone().expect("password_confirm_input initialized");

        div()
            .flex().flex_col().gap_3().p_4().w(px(520.))
            .child(div().font_weight(FontWeight::BOLD).text_lg().child("Add Files"))
            // Format (read-only display — pre-filled from current archive)
            .child(div().text_sm().font_weight(FontWeight::BOLD).child("Format"))
            .child(self.format_dropdown(cx))
            // Source section
            .child(div().font_weight(FontWeight::MEDIUM).child("Source"))
            .child(
                div().flex().flex_row().gap_2()
                    .child(
                        div().px_2().py_1().rounded_md().cursor_pointer()
                            .hover(|mut s| { s.background = Some(theme.hover.into()); s })
                            .child("+ Add Files")
                            .on_mouse_down(MouseButton::Left, cx.listener(|this, _e, _window, cx| {
                                if let Some(paths) = crate::adapters::platform::pick_files() {
                                    this.file_list.extend(paths);
                                    cx.notify();
                                }
                            }))
                    )
                    .child(
                        div().px_2().py_1().rounded_md().cursor_pointer()
                            .hover(|mut s| { s.background = Some(theme.hover.into()); s })
                            .child("+ Add Folder")
                            .on_mouse_down(MouseButton::Left, cx.listener(|this, _e, _window, cx| {
                                if let Some(path) = crate::adapters::platform::pick_folder() {
                                    this.file_list.push(path);
                                    cx.notify();
                                }
                            }))
                    )
            )
            // File list
            .child(
                div().border_1().border_color(theme.border).rounded_md().h(px(100.)).p_2()
                    .children(if self.file_list.is_empty() {
                        vec![div().text_color(theme.muted).text_sm().child("No files added").into_any()]
                    } else {
                        self.file_list.iter().map(|p| {
                            div().text_sm().px_1().child(p.to_string_lossy().to_string()).into_any()
                        }).collect()
                    })
            )
            // Wildcard filter
            .child(
                div().flex().flex_row().gap_2().items_center()
                    .child(div().text_sm().child("Filter:"))
                    .child(div().w(px(120.)).child(Input::new(&filter_input)))
                    .child(
                        div().px_2().py_1().border_1().border_color(theme.border).rounded_md().cursor_pointer()
                            .w(px(90.)).flex().flex_row().justify_between()
                            .child(self.filter_policy.label())
                            .child(if self.show_policy_dropdown { "\u{25B2}" } else { "\u{25BC}" })
                            .on_mouse_down(MouseButton::Left, cx.listener(|this, _e, _window, cx| {
                                this.show_policy_dropdown = !this.show_policy_dropdown;
                                cx.notify();
                            }))
                    )
            )
            .when(self.show_policy_dropdown, |el| {
                el.child(
                    div().border_1().border_color(theme.border).rounded_md().flex().flex_col().w(px(90.))
                        .child(
                            div().px_2().py_1().cursor_pointer()
                                .hover(|mut s| { s.background = Some(theme.hover.into()); s })
                                .child("Include")
                                .on_mouse_down(MouseButton::Left, cx.listener(|this, _e, _window, cx| {
                                    this.filter_policy = FilterPolicy::Include;
                                    this.show_policy_dropdown = false;
                                    cx.notify();
                                }))
                        )
                        .child(
                            div().px_2().py_1().cursor_pointer()
                                .hover(|mut s| { s.background = Some(theme.hover.into()); s })
                                .child("Exclude")
                                .on_mouse_down(MouseButton::Left, cx.listener(|this, _e, _window, cx| {
                                    this.filter_policy = FilterPolicy::Exclude;
                                    this.show_policy_dropdown = false;
                                    cx.notify();
                                }))
                        )
                )
            })
            // Recursive checkbox
            .child(
                div().flex().flex_row().gap_1().items_center()
                    .child(if self.recursive { "\u{2611}" } else { "\u{2610}" })
                    .child("Recursive")
                    .cursor_pointer()
                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _e, _window, cx| {
                        this.recursive = !this.recursive;
                        cx.notify();
                    }))
            )
            // Archive path prefix
            .child(
                div().flex().flex_row().gap_2().items_center()
                    .child(div().text_sm().child("Archive path:"))
                    .child(div().w(px(150.)).child(Input::new(&prefix_input)))
            )
            // Compression section
            .child(div().font_weight(FontWeight::MEDIUM).child("Compression"))
            .when(self.format.supports_compression_level(), |el| {
                el.child(
                    div().flex().flex_row().gap_2().items_center()
                        .child(div().text_sm().child("Level:"))
                        .child(
                            div().px_2().py_1().border_1().border_color(theme.border).rounded_md().cursor_pointer()
                                .w(px(120.)).flex().flex_row().justify_between()
                                .child(compression_level_names(self.compression_level))
                                .child(if self.show_level_dropdown { "\u{25B2}" } else { "\u{25BC}" })
                                .on_mouse_down(MouseButton::Left, cx.listener(|this, _e, _window, cx| {
                                    this.show_level_dropdown = !this.show_level_dropdown;
                                    cx.notify();
                                }))
                        )
                )
            })
            .when(self.show_level_dropdown && self.format.supports_compression_level(), |el| {
                el.child(
                    div().border_1().border_color(theme.border).rounded_md().flex().flex_col().w(px(120.))
                        .children((0u8..=5).map(|lvl| {
                            div().px_2().py_1().cursor_pointer()
                                .bg(if lvl == self.compression_level { theme.selection } else { hsla(0., 0., 0., 0.) })
                                .hover(|mut s| { s.background = Some(theme.hover.into()); s })
                                .child(compression_level_names(lvl))
                                .on_mouse_down(MouseButton::Left, cx.listener(move |this, _e, _window, cx| {
                                    this.compression_level = lvl;
                                    this.show_level_dropdown = false;
                                    cx.notify();
                                }))
                                .into_any_element()
                        }).collect::<Vec<_>>())
                )
            })
            .when(self.format == ArchiveFormat::SevenZip || self.format == ArchiveFormat::Zip, |el| {
                el.child(
                    div().flex().flex_row().gap_2().items_center()
                        .child(div().text_sm().child("Method:"))
                          .child(div().px_2().py_1().border_1().border_color(theme.border).rounded_md().w(px(100.)).child(
                            if self.compression_method.is_empty() { "LZMA2".to_string() } else { self.compression_method.clone() }
                        ))
                )
            })
            // Advanced
            .child(
                div().flex().flex_col().gap_1()
                    .child(
                        div().px_2().py_1().cursor_pointer().flex().flex_row().gap_1()
                            .hover(|mut s| { s.background = Some(theme.hover.into()); s })
                            .child(if self.show_advanced { "\u{25BC}" } else { "\u{25B6}" })
                            .child("Advanced")
                            .on_mouse_down(MouseButton::Left, cx.listener(|this, _e, _window, cx| {
                                this.show_advanced = !this.show_advanced;
                                cx.notify();
                            }))
                    )
                    .when(self.show_advanced, |el| {
                        el.child(div().flex().flex_col().gap_2().pl_4().pt_1()
                            .when(self.format == ArchiveFormat::SevenZip, |el| {
                                el.child(div().flex().flex_row().gap_2().items_center()
                                    .child(div().text_sm().child("Dictionary:"))
                                    .child(div().px_2().py_1().border_1().border_color(theme.border).rounded_md().w(px(100.)).child(
                                        if self.dictionary_size.is_empty() { "64 MB".to_string() } else { self.dictionary_size.clone() }
                                    ))
                                ).child(div().flex().flex_row().gap_2().items_center()
                                    .child(div().text_sm().child("Word size:"))
                                    .child(div().px_2().py_1().border_1().border_color(theme.border).rounded_md().w(px(80.)).child(
                                        if self.word_size.is_empty() { "64".to_string() } else { self.word_size.clone() }
                                    ))
                                )
                            })
                            .when(self.format == ArchiveFormat::SevenZip, |el| {
                                el.child(
                                    div().flex().flex_row().gap_1().items_center()
                                        .child(if self.solid { "\u{2611}" } else { "\u{2610}" })
                                        .child("Solid archive")
                                        .cursor_pointer()
                                        .on_mouse_down(MouseButton::Left, cx.listener(|this, _e, _window, cx| {
                                            this.solid = !this.solid;
                                            cx.notify();
                                        }))
                                )
                            })
                            .child(div().flex().flex_row().gap_2().items_center()
                                .child(div().text_sm().child("Volume size:"))
                                .child(div().px_2().py_1().border_1().border_color(theme.border).rounded_md().w(px(100.)).child(
                                    if self.volume_size.is_empty() { "\u{2014}".to_string() } else { self.volume_size.clone() }
                                ))
                            )
                            .child(div().flex().flex_row().gap_2().items_center()
                                .child(div().text_sm().child("Threads:"))
                                .child(div().px_2().py_1().border_1().border_color(theme.border).rounded_md().w(px(80.)).child(
                                    if self.thread_count.is_empty() { "auto".to_string() } else { self.thread_count.clone() }
                                ))
                            )
                            .child(
                                div().flex().flex_row().gap_1().items_center()
                                    .child(if self.store_timestamps { "\u{2611}" } else { "\u{2610}" })
                                    .child("Store timestamps")
                                    .cursor_pointer()
                                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _e, _window, cx| {
                                        this.store_timestamps = !this.store_timestamps;
                                        cx.notify();
                                    }))
                            )
                        )
                    })
            )
            // Encryption
            .child(div().font_weight(FontWeight::MEDIUM).child("Encryption"))
            .child(Input::new(&password_input))
            .child(Input::new(&password_confirm_input))
            .when(self.format.supports_encrypted_filenames(), |el| {
                el.child(
                    div().flex().flex_row().gap_1().items_center()
                        .child(if self.encrypt_filenames { "\u{2611}" } else { "\u{2610}" })
                        .child("Encrypt filenames")
                        .cursor_pointer()
                        .on_mouse_down(MouseButton::Left, cx.listener(|this, _e, _window, cx| {
                            this.encrypt_filenames = !this.encrypt_filenames;
                            cx.notify();
                        }))
                )
            })
            // Buttons
            .child(
                div().flex().flex_row().justify_end().gap_2().pt_2()
                    .child(div().px_3().py_1().rounded_md().cursor_pointer().child("Cancel")
                        .on_mouse_down(MouseButton::Left, cx.listener(|this, _e, _window, cx| cx.emit(AddFilesDialogEvent::Canceled))))
                    .child(
                        div().px_3().py_1().rounded_md().cursor_pointer()
                            .bg(if self.is_valid() { theme.primary } else { theme.muted })
                            .child("OK")
                            .when(self.is_valid(), |el| {
                                el.on_mouse_down(MouseButton::Left, cx.listener(|this, _e, _window, cx| {
                                    use crate::domain::repository::RepoGlobal;
                                    let repo = cx.global::<RepoGlobal>().0.clone();
                                    let files = this.file_list.clone();
                                    let format = this.format;
                                    let level = this.compression_level;
                                    let encryption = this.build_encryption();
                                    let archive = cx.global::<RepoGlobal>().0.clone();
                                    cx.spawn(async move |_, _cx| {
                                        let _ = files;
                                        let _ = format;
                                        let _ = level;
                                        let _ = encryption;
                                    }).detach();
                                    cx.emit(AddFilesDialogEvent::AddRequested {
                                        files,
                                        format,
                                        compression_level: level,
                                        encryption,
                                    });
                                }))
                            })
                    )
            )
    }
}
