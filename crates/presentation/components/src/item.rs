use gpui::{AnyElement, FontWeight};
use gpui_component::Sizable;
use crate::{prelude::*};

/// Media slot for an [`Item`] row (icon, avatar, thumbnail, etc.).
#[derive(IntoElement)]
pub struct ItemMedia {
    child: AnyElement,
}

impl ItemMedia {
    pub fn new(child: impl IntoElement) -> Self {
        Self {
            child: child.into_any_element(),
        }
    }
}

impl RenderOnce for ItemMedia {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div().flex_none().child(self.child)
    }
}

/// Primary text block for an [`Item`] row.
#[derive(IntoElement)]
pub struct ItemContent {
    title: SharedString,
    description: Option<SharedString>,
}

impl ItemContent {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            description: None,
        }
    }

    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }
}

impl RenderOnce for ItemContent {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        v_flex()
            .flex_1()
            .min_w_0()
            .gap_0p5()
            .child(Label::new(self.title).font_weight(FontWeight::MEDIUM))
            .children(
                self.description
                    .map(|d| Label::new(d).text_color(cx.theme().muted)),
            )
    }
}

/// Trailing actions slot for an [`Item`] row.
#[derive(IntoElement)]
pub struct ItemActions {
    child: AnyElement,
}

impl ItemActions {
    pub fn new(child: impl IntoElement) -> Self {
        Self {
            child: child.into_any_element(),
        }
    }
}

impl RenderOnce for ItemActions {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div().flex_none().child(self.child)
    }
}

/// A generic row primitive with media, content, and trailing actions slots.
#[derive(IntoElement, RegisterComponent)]
pub struct Item {
    media: Option<AnyElement>,
    content: Option<AnyElement>,
    actions: Option<AnyElement>,
}

impl Item {
    pub fn new() -> Self {
        Self {
            media: None,
            content: None,
            actions: None,
        }
    }

    pub fn media(mut self, media: impl IntoElement) -> Self {
        self.media = Some(media.into_any_element());
        self
    }

    pub fn content(mut self, content: impl IntoElement) -> Self {
        self.content = Some(content.into_any_element());
        self
    }

    pub fn actions(mut self, actions: impl IntoElement) -> Self {
        self.actions = Some(actions.into_any_element());
        self
    }
}

impl Default for Item {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderOnce for Item {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        h_flex()
            .id("item")
            .w_full()
            .items_center()
            .gap_3()
            .px_3()
            .py_2()
            .rounded_md()
            .hover(|style| style.bg(cx.theme().list_hover))
            .children(self.media)
            .children(self.content)
            .children(self.actions)
    }
}

impl Component for Item {
    fn scope() -> ComponentScope {
        ComponentScope::DataDisplay
    }

    fn description() -> Option<&'static str> {
        Some("A generic row primitive with media, content, and trailing actions slots.")
    }

    fn preview(_window: &mut Window, cx: &mut App) -> Option<AnyElement> {
        let example_avatar = "https://avatars.githubusercontent.com/u/1714999?v=4";

        Some(
            v_flex()
                .gap_2()
                .w(px(360.))
                .child(
                    Item::new()
                        .media(ItemMedia::new(Avatar::new().src(example_avatar).size(px(40.))))
                        .content(
                            ItemContent::new("Jane Cooper")
                                .description("Regional Paradigm Technician"),
                        )
                        .actions(Button::new("item-action").icon(IconName::Ellipsis)),
                )
                .child(
                    Item::new()
                        .media(ItemMedia::new(
                            Icon::new(IconName::File)
                                .small()
                                .text_color(cx.theme().muted),
                        ))
                        .content(
                            ItemContent::new("project-plan.pdf").description("Updated 2 hours ago"),
                        ),
                )
                .into_any_element(),
        )
    }
}
