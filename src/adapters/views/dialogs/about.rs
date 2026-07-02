use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::dialog::{DialogFooter, DialogHeader, DialogTitle};
use gpui_component::v_flex;

actions!(about,[CloseAboutDialog]);

pub struct AboutDialog {
    focus_handle: FocusHandle,
}

impl AboutDialog {
    pub fn open(cx: &mut App) {
        let window_size = size(px(380.), px(220.));
        let _ = cx.open_window(
            WindowOptions {
                titlebar: Some(TitlebarOptions {
                    title: Some("About".into()),
                    appears_transparent: true,
                    traffic_light_position: Some(point(px(12.), px(12.))),
                }),
                window_bounds: Some(WindowBounds::centered(window_size, cx)),
                is_resizable: false,
                is_minimizable: false,
                focus:true,
                // kind: WindowKind::Floating,
                // app_id: Some(ReleaseChannel::global(cx).app_id().to_owned()),
                ..Default::default()
            },
            move |window, cx| {
                cx.bind_keys([
                    KeyBinding::new("enter",CloseAboutDialog,"about".into()),
                    KeyBinding::new("esc",CloseAboutDialog,"about".into())
                ]);
                let focus_handle = cx.focus_handle();
                focus_handle.focus(window,cx);

                let dialog = cx.new(|_| AboutDialog{ focus_handle });
                cx.new(|cx| gpui_component::Root::new(dialog, window, cx))
            },
        );
    }
}

impl Render for AboutDialog {

    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<crate::theme::Theme>().clone();

        v_flex()
            .p_4()
            .size_full()
            .track_focus(&self.focus_handle)
            .key_context("about")
            .on_action(|&CloseAboutDialog, window, _app| {
                println!("Enter key hit! close about dialog");
                window.remove_window();
            })
            .child(DialogHeader::new().child(DialogTitle::new().child("About bit7z Archiver")))
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
            .child(
                DialogFooter::new().justify_center().child(
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
