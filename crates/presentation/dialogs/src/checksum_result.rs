use bit7z_app_checksum::ChecksumResult;
use bit7z_pres_theme::Theme;
use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::scroll::ScrollableElement as _;
use gpui_component::v_flex;

pub struct ChecksumResultDialog {
    pub results: Vec<ChecksumResult>,
}

#[derive(Debug, Clone)]
pub enum ChecksumResultEvent {
    Close,
}

impl EventEmitter<ChecksumResultEvent> for ChecksumResultDialog {}

impl ChecksumResultDialog {
    pub fn new(results: Vec<ChecksumResult>) -> Self {
        Self { results }
    }
}

impl Render for ChecksumResultDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>().clone();

        v_flex().gap_3().p_4().w(px(520.))
            .child(div().font_weight(FontWeight::BOLD).text_lg().child("Checksums"))
            .child(
                div().flex().flex_col().gap_2().max_h(px(400.)).overflow_y_scrollbar()
                    .children(self.results.iter().map(|r| {
                        checksum_row(r, &theme).into_any_element()
                    }).collect::<Vec<_>>())
            )
            .child(
                div().flex().flex_row().justify_end().gap_2().pt_2()
                    .child(
                        div().px_3().py_1().rounded_md().bg(theme.primary).cursor_pointer().child("Close")
                            .on_mouse_down(MouseButton::Left, cx.listener(|_this, _e, _window, cx| {
                                cx.emit(ChecksumResultEvent::Close);
                            }))
                    )
            )
    }
}

fn checksum_row(result: &ChecksumResult, theme: &Theme) -> impl IntoElement {
    div().flex().flex_col().gap_1().p_2().border_1().border_color(theme.border).rounded_md()
        .child(div().text_sm().font_weight(FontWeight::MEDIUM).child(result.path.clone()))
        .when_some(result.crc32.as_ref(), |el, v| {
            el.child(checksum_field("CRC32", v, theme))
        })
        .when_some(result.md5.as_ref(), |el, v| {
            el.child(checksum_field("MD5", v, theme))
        })
        .when_some(result.sha1.as_ref(), |el, v| {
            el.child(checksum_field("SHA1", v, theme))
        })
        .when_some(result.sha256.as_ref(), |el, v| {
            el.child(checksum_field("SHA256", v, theme))
        })
}

fn checksum_field(label: &str, value: &str, theme: &Theme) -> impl IntoElement {
    div().flex().flex_row().gap_2()
        .child(div().w(px(60.)).text_xs().text_color(theme.muted).child(label.to_string()))
        .child(div().text_xs().font_family("monospace").child(value.to_string()))
}
