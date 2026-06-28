use crate::theme::Theme;
use gpui::*;
use gpui_component::{h_flex, Sizable};
use gpui_component::spinner::Spinner;

/// Reusable loading state.
pub fn loading_view(cx: &App) -> impl IntoElement {
    let theme = cx.global::<Theme>();
    h_flex()
        .h_full()
        // .w_full()
        .items_center()
        .gap_2()
        .child(
            Spinner::new()
                .large()
                .color(theme.muted)
        )
        .child("Loading ...")
}

/// Reusable empty state with a message.
pub fn empty_view(cx: &App, message: &str) -> impl IntoElement {
    let theme = cx.global::<Theme>();
    div()
        .h_full().p_8().text_center().text_color(theme.muted)
        .child(message.to_string())
}

/// Reusable error state.
pub fn error_view(cx: &App, message: &str) -> impl IntoElement {
    let theme = cx.global::<Theme>();
    div().h_full().w_full().p_8().text_color(theme.error)
        .child(format!("Error: {}", message))
}
