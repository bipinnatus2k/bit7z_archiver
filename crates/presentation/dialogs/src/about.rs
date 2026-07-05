use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use bit7z_pres_components::window_dialog::{
    DialogFooter, DialogHeader, DialogTitle,
    open_window_dialog, WindowDialogOptions,
    CloseAction,
};

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
            |dlg, _window, cx| {
                let theme = cx.global::<bit7z_pres_theme::Theme>().clone();

                dlg.header(
                    DialogHeader::new()
                        .child(DialogTitle::new().child("About bit7z Archiver")),
                )
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .flex_col()
                        .items_center()
                        .justify_center()
                        .gap_1()
                        .child(
                            div()
                                .text_lg()
                                .font_weight(FontWeight::BOLD)
                                .child("bit7z Archiver"),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(theme.muted)
                                .child(format!("v{}", env!("CARGO_PKG_VERSION"))),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(theme.muted)
                                .child(format!("{}", env!("CARGO_PKG_DESCRIPTION"))),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(theme.muted)
                                .child(format!("{}", env!("CARGO_PKG_LICENSE"))),
                        ),
                )
                .footer(
                    DialogFooter::new().justify_center().child(
                        Button::new("ok")
                            .label("OK")
                            .primary()
                            .cursor_pointer()
                            .on_click(|_, window, _cx| {
                                window.remove_window();
                            }),
                    ),
                );
            },
        );
    }
}
