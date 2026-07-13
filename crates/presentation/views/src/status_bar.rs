use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::separator::Separator;
use gpui_component::{ActiveTheme, Icon, IconName, Sizable};
use bit7z_pres_view_models::AppState;

pub struct StatusBar {
    state: AppState,
}

impl StatusBar {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

impl Render for StatusBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let status_text = self.state.status_text();
        gpui_component::status_bar::StatusBar::new()
            .child(Icon::new(IconName::GalleryVerticalEnd).xsmall())
            .child(status_text)
            .child(Separator::vertical())
            .right(cx.theme().theme_name().clone())
            .right(format!("v{}", env!("CARGO_PKG_VERSION")))
            .right(
                Button::new("assistant")
                    .ghost()
                    .xsmall()
                    .icon(IconName::Github)
                    .tooltip("GPUI Component GitHub repository")
                    .on_click(|_, _, cx| {
                        cx.open_url("https://github.com/longbridge/gpui-component")
                    }),
            )
    }
}
