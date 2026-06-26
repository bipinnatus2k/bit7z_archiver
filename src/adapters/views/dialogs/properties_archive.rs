use crate::domain::repository::ArchiveProperties;
use crate::theme::Theme;
use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::description_list::{DescriptionList, DescriptionItem};
use gpui_component::Sizable;

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

fn bool_yn(v: bool) -> &'static str {
    if v { "Yes" } else { "No" }
}

fn props_type_label(props: &ArchiveProperties) -> String {
    let mut parts = Vec::new();
    if props.is_solid {
        parts.push("Solid");
    }
    if props.is_multi_volume {
        parts.push("Multi-volume");
    }
    if parts.is_empty() {
        "Archive".to_string()
    } else {
        parts.join(" ")
    }
}

impl Render for PropertiesArchiveDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>().clone();

        div().flex().flex_col().gap_2().p_4().w(px(420.))
            .child(div().font_weight(FontWeight::BOLD).text_lg().child("Archive Properties"))
            .when_some(self.properties.as_ref(), |el, props| {
                el
                    .child(div().font_weight(FontWeight::MEDIUM).child("General"))
                    .child(
                        DescriptionList::vertical()
                            .bordered(false)
                            .small()
                            .children([
                                DescriptionItem::new("Type").value(props_type_label(props)),
                                DescriptionItem::new("Location").value(self.archive_path.clone()),
                                DescriptionItem::new("Size").value(human_size(props.total_size)),
                                DescriptionItem::new("Packed").value(human_size(props.packed_size)),
                                DescriptionItem::new("Ratio")
                                    .value(format!("{:.0}%", if props.total_size > 0 {
                                        (1.0 - props.packed_size as f64 / props.total_size as f64) * 100.0
                                    } else {
                                        0.0
                                    })),
                                DescriptionItem::new("Files").value(format!("{}", props.files_count)),
                                DescriptionItem::new("Folders").value(format!("{}", props.folders_count)),
                            ])
                    )
                    .child(div().font_weight(FontWeight::MEDIUM).pt_2().child("Advanced"))
                    .child({
                        let mut items = vec![
                            DescriptionItem::new("Solid").value(bool_yn(props.is_solid)),
                            DescriptionItem::new("Encrypted").value(bool_yn(props.is_encrypted)),
                            DescriptionItem::new("Encrypted names").value(bool_yn(props.encrypted_names)),
                            DescriptionItem::new("Multi-volume").value(bool_yn(props.is_multi_volume)),
                            DescriptionItem::new("Has comment").value(bool_yn(props.has_comment)),
                            DescriptionItem::new("Recovery record").value(bool_yn(props.has_recovery_record)),
                            DescriptionItem::new("Locked").value(bool_yn(props.locked)),
                        ];
                        if let Some(sz) = props.dictionary_size {
                            items.push(DescriptionItem::new("Dictionary").value(human_size(sz)));
                        }
                        DescriptionList::vertical().bordered(false).small().children(items)
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
                            })),
                    ),
            )
    }
}
