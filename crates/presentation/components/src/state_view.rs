use bit7z_pres_theme::Theme;
use gpui::*;
use gpui_component::skeleton::Skeleton;
use gpui_component::{h_flex, v_flex, Icon, IconName, Sizable};
use gpui_component::button::ButtonVariants;
use gpui_component::spinner::Spinner;

pub fn loading_view(cx: &App) -> impl IntoElement {
    let theme = cx.global::<Theme>();
    h_flex()
        .h_full()
        .items_center()
        .justify_center()
        .gap_2()
        .child(Spinner::new().large().color(theme.primary))
        .child(
            div()
                .text_color(theme.muted_foreground)
                .child("Loading..."),
        )
}

pub fn skeleton_view() -> impl IntoElement {
    v_flex()
        .gap_3()
        .p_4()
        .child(Skeleton::new().w_full().h_4().rounded_md())
        .child(Skeleton::new().w(px(250.)).h_4().rounded_md())
        .child(Skeleton::new().w(px(180.)).h_4().rounded_md())
}

pub fn skeleton_table_rows(count: usize) -> impl IntoElement {
    v_flex()
        .gap_2()
        .children((0..count).map(|_| {
            h_flex()
                .gap_4()
                .p_3()
                .child(Skeleton::new().size_8().rounded_full())
                .child(Skeleton::new().w(px(150.)).h_4().rounded_md())
                .child(Skeleton::new().w(px(100.)).h_4().rounded_md())
                .child(Skeleton::new().w(px(80.)).h_4().rounded_md())
        }))
}

pub fn empty_view(cx: &App, message: &str) -> impl IntoElement {
    let theme = cx.global::<Theme>();
    v_flex()
        .h_full()
        .w_full()
        .items_center()
        .justify_center()
        .gap_3()
        .p_8()
        .child(
            Icon::new(IconName::FolderOpen)
                .size_8()
                .text_color(theme.muted),
        )
        .child(
            div()
                .text_center()
                .text_color(theme.muted_foreground)
                .child(message.to_string()),
        )
}

pub fn empty_view_with_action(
    cx: &App,
    message: &str,
    action_label: &str,
    on_action: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let theme = cx.global::<Theme>();
    v_flex()
        .h_full()
        .w_full()
        .items_center()
        .justify_center()
        .gap_3()
        .p_8()
        .child(
            Icon::new(IconName::FolderOpen)
                .size_8()
                .text_color(theme.muted),
        )
        .child(
            div()
                .text_center()
                .text_color(theme.muted_foreground)
                .child(message.to_string()),
        )
        .child(
            gpui_component::button::Button::new("action")
                .primary()
                .label(action_label)
                .on_click(on_action),
        )
}

pub fn error_view(cx: &App, message: &str) -> impl IntoElement {
    let theme = cx.global::<Theme>();
    v_flex()
        .h_full()
        .w_full()
        .items_center()
        .justify_center()
        .gap_3()
        .p_8()
        .child(
            Icon::new(IconName::CircleX)
                .size_8()
                .text_color(theme.error),
        )
        .child(
            div()
                .text_center()
                .text_color(theme.error)
                .child(format!("Error: {}", message)),
        )
}

pub fn error_view_with_retry(
    cx: &App,
    message: &str,
    on_retry: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let theme = cx.global::<Theme>();
    v_flex()
        .h_full()
        .w_full()
        .items_center()
        .justify_center()
        .gap_3()
        .p_8()
        .child(
            Icon::new(IconName::CircleX)
                .size_8()
                .text_color(theme.error),
        )
        .child(
            div()
                .text_center()
                .text_color(theme.error)
                .child(format!("Error: {}", message)),
        )
        .child(
            gpui_component::button::Button::new("retry")
                .label("Retry")
                .on_click(on_retry),
        )
}
