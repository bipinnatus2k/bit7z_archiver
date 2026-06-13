use crate::domain::preferences::*;
use crate::theme::Theme;
use gpui::prelude::FluentBuilder as _;
use gpui::*;

pub struct SettingsDialog {
    pub prefs: Preferences,
    pub active_tab: SettingsTab,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SettingsTab { General, Archive, Preview }

#[derive(Debug, Clone)]
pub enum SettingsDialogEvent {
    Saved(Preferences),
    Canceled,
}

impl EventEmitter<SettingsDialogEvent> for SettingsDialog {}

impl SettingsDialog {
    pub fn new(cx: &mut Context<Self>) -> Entity<Self> {
        let prefs = cx.global::<Preferences>().clone();
        cx.new(|_cx| Self { prefs, active_tab: SettingsTab::General })
    }
}

impl Render for SettingsDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex().flex_col().gap_3().p_4().w(px(480.))
            .child(div().font_weight(FontWeight::BOLD).text_lg().child("Settings"))
            .child(
                div().flex().flex_row().gap_1().border_b_1().border_color(cx.global::<Theme>().border).pb_1()
                    .child(tab_button("General".to_string(), SettingsTab::General, &self.active_tab, cx))
                    .child(tab_button("Archive".to_string(), SettingsTab::Archive, &self.active_tab, cx))
                    .child(tab_button("Preview".to_string(), SettingsTab::Preview, &self.active_tab, cx))
            )
            .child(match self.active_tab {
                SettingsTab::General => div().flex().flex_col().gap_2()
                    .child(div().child("Minimize to tray"))
                    .child(div().child(if self.prefs.ui.minimize_to_tray { "☑ Enabled" } else { "☐ Disabled" }))
                    .child(div().child("Confirm before delete"))
                    .child(div().child(if self.prefs.ui.confirm_delete { "☑ Yes" } else { "☐ No" }))
                    .into_any(),
                SettingsTab::Archive => div().flex().flex_col().gap_2()
                    .child(div().child(format!("Default format: {}", self.prefs.archive.default_format.display_name())))
                    .child(div().child(format!("Compression level: {}", self.prefs.archive.default_compression_level)))
                    .child(div().child(format!("Encrypt filenames: {}", self.prefs.archive.default_encrypt_filenames)))
                    .child(div().text_sm().text_color(cx.global::<Theme>().muted).child("Recent files:"))
                    .children(self.prefs.archive.recent_files.iter().map(|f|
                        div().text_sm().text_color(cx.global::<Theme>().muted).child(f.clone()).into_any()
                    ).collect::<Vec<_>>())
                    .into_any(),
                SettingsTab::Preview => div().flex().flex_col().gap_2()
                    .child(div().child(format!("Text preview max: {} KB", self.prefs.preview.text_max_bytes / 1024)))
                    .child(div().child(format!("Hex dump bytes: {}", self.prefs.preview.hex_dump_bytes)))
                    .child(div().child(format!("Auto-preview: {}", self.prefs.preview.auto_preview)))
                    .into_any(),
            })
            .child(
                div().flex().flex_row().justify_end().gap_2().pt_2()
                    .child(div().px_3().py_1().rounded_md().cursor_pointer().child("Cancel")
                        .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _e, _window, cx| cx.emit(SettingsDialogEvent::Canceled))))
                    .child(div().px_3().py_1().rounded_md().bg(cx.global::<Theme>().primary).cursor_pointer().child("Save")
                        .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _e, _window, cx| cx.emit(SettingsDialogEvent::Saved(this.prefs.clone())))))
            )
    }
}

fn tab_button(label: String, tab: SettingsTab, active: &SettingsTab, cx: &mut Context<SettingsDialog>) -> impl IntoElement {
    let is_active = *active == tab;
    div()
        .px_3().py_1().rounded_md()
        .when(is_active, |el| el.bg(cx.global::<Theme>().selection))
        .hover(|mut s| { s.background = Some(cx.global::<Theme>().hover.into()); s })
        .cursor_pointer()
        .child(label)
}







