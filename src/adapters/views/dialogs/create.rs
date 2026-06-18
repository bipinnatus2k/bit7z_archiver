use crate::domain::archive::*;
use crate::domain::preferences::Preferences;
use crate::theme::Theme;
use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::input::{Input, InputState};

pub struct CreateArchiveDialog {
    pub file_list: Vec<CreateFileItem>,
    pub format: ArchiveFormat,
    pub compression_level: u8,
    pub password: String,
    pub password_confirm: String,
    pub encrypt_filenames: bool,
    pub destination: String,
    password_state: Entity<InputState>,
    password_confirm_state: Entity<InputState>,
}

#[derive(Debug, Clone)]
pub enum CreateDialogEvent {
    CreateRequested(CreateDialogInput),
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

impl CreateArchiveDialog {
    pub fn new(window: &mut Window, cx: &mut Context<Self>, files: Vec<CreateFileItem>) -> Self {
        let prefs = cx.global::<Preferences>();
        let default_format = prefs.archive.default_format;
        let compression_level = prefs.archive.default_compression_level;
        let encrypt_filenames = prefs.archive.default_encrypt_filenames;
        let dest = if files.len() == 1 {
            let stem = files[0].path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            format!("{}.{}", stem, default_format.extension())
        } else {
            format!("archive.{}", default_format.extension())
        };
        let password_state = cx.new(|cx| InputState::new(window, cx).placeholder("Optional"));
        let password_confirm_state = cx.new(|cx| InputState::new(window, cx).placeholder("Confirm password"));
        Self {
            file_list: files,
            format: default_format,
            compression_level,
            password: String::new(),
            password_confirm: String::new(),
            encrypt_filenames,
            destination: dest,
            password_state,
            password_confirm_state,
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
}

impl Render for CreateArchiveDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>().clone();
        let password_state = self.password_state.clone();
        let password_confirm_state = self.password_confirm_state.clone();

        div()
            .flex().flex_col().gap_3().p_4().w(px(520.))
            .child(div().font_weight(FontWeight::BOLD).text_lg().child("Create Archive"))
            .child(
                div().border_1().border_color(theme.border).rounded_md().h(px(160.)).p_2()
                    .children(if self.file_list.is_empty() {
                        vec![div().text_color(theme.muted).child("Drop files here").into_any()]
                    } else {
                        self.file_list.iter().map(|f| {
                            div().px_2().py_1().child(f.display_name()).into_any()
                        }).collect()
                    })
            )
            .child(
                div().flex().flex_row().gap_2().children([
                    div().px_2().py_1().rounded_md().hover(|mut s| { s.background = Some(theme.hover.into()); s }).cursor_pointer().child("+ Add Files")
                        .on_mouse_down(MouseButton::Left, cx.listener(|this: &mut CreateArchiveDialog, _event: &MouseDownEvent, _window: &mut Window, cx| {
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
                        })).into_any(),
                    div().px_2().py_1().rounded_md().hover(|mut s| { s.background = Some(theme.hover.into()); s }).cursor_pointer().child("+ Add Folder").into_any(),
                ])
            )
            .child(div().child("Format")).child(div().child("Level: 0 [====] 9"))
            .child(
                div().flex().flex_col().gap_1()
                    .child(div().child("Password"))
                    .child(Input::new(&password_state))
                    .child(div().child("Confirm"))
                    .child(Input::new(&password_confirm_state))
                    .when(self.format.supports_encrypted_filenames(), |el| {
                        el.child(div().flex().flex_row().gap_1().child("☐ Encrypt filenames"))
                    })
            )
            .child(div().child("Destination")).child(
                div().px_2().py_1().border_1().border_color(theme.border).rounded_md().child(self.destination.clone())
            )
            .child(
                div().flex().flex_row().justify_end().gap_2().pt_2()
                    .child(div().px_3().py_1().rounded_md().cursor_pointer().child("Cancel")
                        .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _e, _window, cx| cx.emit(CreateDialogEvent::Canceled))))
                    .child(
                        div().px_3().py_1().rounded_md().cursor_pointer()
                            .bg(if self.is_valid() { theme.primary } else { theme.muted })
                            .child(if self.password.is_empty() { "Create" } else { "Create Encrypted" })
                            .when(self.is_valid(), |el| {
                                el.on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _e, _window, cx| {
                                    use crate::domain::repository::RepoGlobal;
                                    let repo = cx.global::<RepoGlobal>().0.clone();
                                    let dest = std::path::PathBuf::from(&this.destination);
                                    let _ = repo.create(&dest, this.format, this.build_encryption().as_ref());
                                    cx.emit(CreateDialogEvent::Canceled);
                                }))
                            })
                    )
            )
    }
}
