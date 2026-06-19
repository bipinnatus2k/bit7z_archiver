use crate::domain::repository::ArchiveProperties;
use crate::theme::Theme;
use gpui::prelude::FluentBuilder as _;
use gpui::*;

pub struct PropertiesArchiveDialog {
    pub properties: Option<ArchiveProperties>,
    pub archive_path: String,
}

#[derive(Debug, Clone)]
pub enum PropertiesArchiveEvent {
    Close,
}

impl EventEmitter<PropertiesArchiveEvent> for PropertiesArchiveDialog {}

impl PropertiesArchiveDialog {
    pub fn new(archive_path: String, cx: &mut Context<Self>) -> Self {
        Self {
            properties: None,
            archive_path,
        }
    }

    pub fn set_properties(&mut self, props: ArchiveProperties, cx: &mut Context<Self>) {
        self.properties = Some(props);
        cx.notify();
    }
}

fn human_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

fn prop_row(label: &str, value: String, theme: &Theme) -> impl IntoElement {
    div().flex().flex_row().gap_4().py_1p5()
        .child(div().w(px(120.)).text_sm().text_color(theme.muted).child(label.to_string()))
        .child(div().text_sm().child(value))
}

fn bool_yn(v: bool) -> &'static str {
    if v { "Yes" } else { "No" }
}

impl Render for PropertiesArchiveDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>().clone();

        div().flex().flex_col().gap_2().p_4().w(px(420.))
            .child(div().font_weight(FontWeight::BOLD).text_lg().child("Archive Properties"))
            .when_some(self.properties.as_ref(), |el, props| {
                el.child(div().font_weight(FontWeight::MEDIUM).child("General"))
                    .child(prop_row("Type:", props_type_label(props), &theme))
                    .child(prop_row("Location:", self.archive_path.clone(), &theme))
                    .child(prop_row("Size:", human_size(props.total_size), &theme))
                    .child(prop_row("Packed:", human_size(props.packed_size), &theme))
                    .child(prop_row("Ratio:", format!("{:.0}%", if props.total_size > 0 {
                        (1.0 - props.packed_size as f64 / props.total_size as f64) * 100.0
                    } else { 0.0 }), &theme))
                    .child(prop_row("Files:", format!("{}", props.files_count), &theme))
                    .child(prop_row("Folders:", format!("{}", props.folders_count), &theme))
                    .child(div().font_weight(FontWeight::MEDIUM).pt_2().child("Advanced"))
                    .child(prop_row("Solid:", bool_yn(props.is_solid).to_string(), &theme))
                    .child(prop_row("Encrypted:", bool_yn(props.is_encrypted).to_string(), &theme))
                    .child(prop_row("Encrypted names:", bool_yn(props.encrypted_names).to_string(), &theme))
                    .child(prop_row("Multi-volume:", bool_yn(props.is_multi_volume).to_string(), &theme))
                    .child(prop_row("Has comment:", bool_yn(props.has_comment).to_string(), &theme))
                    .child(prop_row("Recovery record:", bool_yn(props.has_recovery_record).to_string(), &theme))
                    .child(prop_row("Locked:", bool_yn(props.locked).to_string(), &theme))
                    .when_some(props.dictionary_size, |el, sz| {
                        el.child(prop_row("Dictionary:", human_size(sz), &theme))
                    })
            })
            .when(self.properties.is_none(), |el| {
                el.child(div().text_sm().text_color(theme.muted).child("Loading properties..."))
            })
            .child(
                div().flex().flex_row().justify_end().gap_2().pt_2()
                    .child(
                        div().px_3().py_1().rounded_md().bg(theme.primary).cursor_pointer().child("Close")
                            .on_mouse_down(MouseButton::Left, cx.listener(|_this, _e, _window, cx| {
                                cx.emit(PropertiesArchiveEvent::Close);
                            }))
                    )
            )
    }
}

fn props_type_label(props: &ArchiveProperties) -> String {
    let mut parts = Vec::new();
    if props.is_solid { parts.push("Solid"); }
    if props.is_multi_volume { parts.push("Multi-volume"); }
    if parts.is_empty() { "Archive".to_string() } else { parts.join(" ") }
}
