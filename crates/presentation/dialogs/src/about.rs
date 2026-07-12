use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::v_flex;
use bit7z_pres_components::window_dialog::{
    DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle,
    open_window_dialog, WindowDialogOptions, CloseAction,
};

pub struct AboutContent;

impl Render for AboutContent {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .gap(px(12.))
            .child(
                DialogHeader::new()
                    .child(DialogTitle::new().child("About bit7z Archiver")),
            )
            .child(
                DialogContent::new().child(
                    v_flex()
                        // .items_center()
                        // .justify_center()
                        .h_full()
                        .gap_1()
                        .child(
                            div()
                                .text_lg()
                                .font_weight(FontWeight::BOLD)
                                .child("bit7z Archiver"),
                        )
                        .child(
                            DialogDescription::new()
                                .child(format!("v{}", env!("CARGO_PKG_VERSION"))),
                        )
                        .child(
                            DialogDescription::new()
                                .child(env!("CARGO_PKG_DESCRIPTION").to_string()),
                        )
                        .child(
                            DialogDescription::new()
                                .child(format!("LICENSE: {}", env!("CARGO_PKG_LICENSE"))),
                        ),
                ),
            )
            .child(
                DialogFooter::new()
                // .justify_center()
                .child(
                    Button::new("ok")
                        .label("OK")
                        .primary()
                        .cursor_pointer()
                        .on_click(|_, window, _cx| {
                            window.remove_window();
                        }),
                ),
            )
    }
}

pub struct AboutDialog;

impl AboutDialog {
    pub fn open(cx: &mut App) {
        open_window_dialog(
            cx,
            WindowDialogOptions {
                title: "About".into(),
                width: px(380.),
                height: Some(px(220.)),
                min_width: None,
                min_height: None,
                kind: WindowKind::Dialog,
                close_action: CloseAction::RemoveWindow,
                window_decorations: Some(WindowDecorations::Client),
                window_background: WindowBackgroundAppearance::Opaque,
            },
            |_window, cx| cx.new(|_| AboutContent),
        );
    }
}
