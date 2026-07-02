use crate::theme::Theme;
use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::separator::Separator;
use gpui_component::{ActiveTheme, Icon, IconName, Sizable};

pub struct StatusBar {
    status_text: String,
    show_preview :bool,
}

impl StatusBar {
    pub fn new(status_text: String, show_preview: bool) -> Self {
        Self {
            status_text,
            show_preview,
        }
    }
    
    pub fn default() -> Self { 
        Self { 
            status_text: String::new(),
            show_preview: false,
        }
    }

    pub fn set_status(&mut self, text: &str) {
        self.status_text = text.to_string();
    }
}

impl Render for StatusBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let _theme = cx.global::<Theme>();

        gpui_component::status_bar::StatusBar::new()
            .child(Icon::new(IconName::GalleryVerticalEnd).xsmall())
            .child(self.status_text.clone())
            .child(Separator::vertical())
            // .when(!current_story.is_empty(), |this| {
            //     this.child(current_story.clone())
            // })
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
