use gpui::{AnyElement, FontWeight};

use crate::prelude::*;

/// A flex row pairing a media element (image/avatar) with a text block —
/// Tailwind's "Media Object" pattern.
#[derive(IntoElement, RegisterComponent)]
pub struct MediaObject {
    media: AnyElement,
    heading: SharedString,
    description: Option<SharedString>,
}

impl MediaObject {
    pub fn new(media: impl IntoElement, heading: impl Into<SharedString>) -> Self {
        Self {
            media: media.into_any_element(),
            heading: heading.into(),
            description: None,
        }
    }

    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }
}

impl RenderOnce for MediaObject {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        h_flex().items_start().gap_4().child(self.media).child(
            v_flex()
                .gap_1()
                .child(Label::new(self.heading).font_weight(FontWeight::MEDIUM))
                .children(
                    self.description
                        .map(|d| Label::new(d).text_sm().text_color(cx.theme().muted)),
                ),
        )
    }
}

impl Component for MediaObject {
    fn scope() -> ComponentScope {
        ComponentScope::DataDisplay
    }

    fn description() -> Option<&'static str> {
        Some("A flex row pairing a media element (image/avatar) with a text block.")
    }

    fn preview(_window: &mut Window, _cx: &mut App) -> Option<AnyElement> {
        let example_avatar = "https://avatars.githubusercontent.com/u/1714999?v=4";

        Some(
            v_flex()
                .gap_4()
                .child(MediaObject::new(
                    Avatar::new().src(example_avatar).size(px(48.)),
                    "Jane Cooper",
                ))
                .child(
                    MediaObject::new(Avatar::new().src(example_avatar).size(px(48.)), "Cody Fisher")
                        .description("Regional Paradigm Technician"),
                )
                .into_any_element(),
        )
    }
}
