use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::dialog::{DialogFooter, DialogHeader, DialogTitle};
use gpui_component::WindowExt;

pub struct AboutDialog;

impl AboutDialog {
    pub fn open(cx: &mut AsyncApp) {
        cx.spawn(async move |cx| {
            let _ = cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                        point(px(300.), px(250.)),
                        size(px(380.), px(220.)),
                    ))),
                    window_background: WindowBackgroundAppearance::Opaque,
                    window_decorations: Some(WindowDecorations::Client),
                    focus:true,
                    kind: WindowKind::Dialog,
                    ..Default::default()
                },
                move |window, cx| {
                    let dialog = cx.new(|_| AboutDialog);
                    cx.new(|cx| gpui_component::Root::new(dialog, window, cx))
                },
            );
        }).detach();
    }
}

impl Render for AboutDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<crate::theme::Theme>().clone();

        div().flex().flex_col().p_4().size_full()
            .child(
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
                            .mt_1()
                            .child("Cross-platform compressed file"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted)
                            .child("viewer and editor"),
                    ),
            )
            .child(
                DialogFooter::new().child(
                    div().flex().flex_row().justify_center().w_full().child(
                        Button::new("ok")
                            .label("OK")
                            .with_variant(gpui_component::button::ButtonVariant::Primary)
                            .on_click(|_, window, cx| {
                                window.close_dialog(cx);
                            }),
                    ),
                ),
            )
    }
}
