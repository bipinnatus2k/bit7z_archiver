use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::separator::Separator;
use gpui_component::{ActiveTheme, Icon, IconName, Sizable};

#[derive(IntoElement)]
pub struct StatusBarView {
    status_text: SharedString,
}

impl StatusBarView {
    pub fn new(status_text: impl Into<SharedString>) -> Self {
        Self { status_text: status_text.into() }
    }
}

impl RenderOnce for StatusBarView {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        gpui_component::status_bar::StatusBar::new()
            .child(Icon::new(IconName::GalleryVerticalEnd).xsmall())
            .child(self.status_text)
            .child(Separator::vertical())
            .right(cx.theme().theme_name().clone())
            .right(format!("v{}", env!("CARGO_PKG_VERSION")))
            .right(
                Button::new("assistant")
                    .ghost()
                    .xsmall()
                    .icon(IconName::Github)
                    .on_click(|_, _, cx| {
                        cx.open_url("https://github.com/longbridge/gpui-component")
                    }),
            )
    }
}
