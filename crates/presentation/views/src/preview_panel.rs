use bit7z_app_preview::PreviewData;
use bit7z_pres_components::state_view::{empty_view, loading_view};
use gpui::*;
use gpui_component::ActiveTheme;
use gpui_component::scroll::ScrollableElement;

const HEX_DUMP_BYTES: usize = 4096;

pub struct PreviewPanel {
    data: Option<PreviewData>,
    is_loading: bool,
}

impl PreviewPanel {
    pub fn new() -> Self {
        Self { data: None, is_loading: false }
    }

    pub fn set_data(&mut self, data: Option<PreviewData>) {
        self.data = data;
        self.is_loading = false;
    }

    pub fn set_loading(&mut self) {
        self.is_loading = true;
    }
}

impl Render for PreviewPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().flex_1().overflow_hidden().border_t_1().border_color(cx.theme().border).p_2()
            .child(match &self.data {
                None if self.is_loading => loading_view(cx).into_any_element(),
                None => empty_view(cx, "Select a file to preview").into_any_element(),
                Some(PreviewData::Text(text)) => {
                    div().font_family("monospace").text_sm().overflow_y_scrollbar().child(text.clone()).into_any_element()
                }
                Some(PreviewData::Hex(bytes)) => {
                    render_hex_view(bytes, cx).into_any_element()
                }
                Some(PreviewData::Image(bytes)) => {
                    render_image_view(bytes, cx).into_any_element()
                }
                Some(PreviewData::Unsupported(msg)) => {
                    div().text_color(cx.theme().muted).child(msg.clone()).into_any_element()
                }
            })
    }
}

fn render_hex_view(bytes: &[u8], cx: &mut Context<PreviewPanel>) -> impl IntoElement {
    let truncated = if bytes.len() > HEX_DUMP_BYTES { &bytes[..HEX_DUMP_BYTES] } else { bytes };
    let rows = truncated.chunks(16);
    let offset_width = format!("{:X}", truncated.len().max(1)).len().max(4);

    div().font_family("monospace").text_xs().overflow_y_scrollbar().child(
        div().children(rows.enumerate().map(|(i, chunk)| {
            let offset = i * 16;
            let hex_part: String = chunk.iter()
                .enumerate()
                .map(|(j, b)| {
                    if j == 8 { format!(" {:02X}", b) } else { format!("{:02X} ", b) }
                })
                .collect::<Vec<_>>()
                .concat();
            let hex_padded = format!("{:<49}", hex_part);
            let ascii: String = chunk.iter()
                .map(|b| if b.is_ascii_graphic() || *b == b' ' { *b as char } else { '.' })
                .collect();

            div().child(format!("{:0width$X}  {} |{}|", offset, hex_padded, ascii, width = offset_width))
        }))
    )
}

fn render_image_view(bytes: &[u8], cx: &mut Context<PreviewPanel>) -> impl IntoElement {
    let (fmt_desc, has_dimensions) = detect_image_format(bytes);
    let dims = if has_dimensions {
        try_get_dimensions(bytes).map(|(w, h)| format!("{}x{}", w, h)).unwrap_or_default()
    } else {
        String::new()
    };

    let info = if !dims.is_empty() {
        format!("{} — {} — {} bytes", fmt_desc, dims, bytes.len())
    } else {
        format!("{} — {} bytes", fmt_desc, bytes.len())
    };

    div().flex_1().flex_col().gap_2().child(
        div().text_color(cx.theme().muted).text_sm().child(info)
    ).child(
        div().text_color(cx.theme().muted).text_xs().child(
            "TODO(integration): render image via GPUI window::Image from in-memory buffer"
        )
    )
}

fn detect_image_format(bytes: &[u8]) -> (&'static str, bool) {
    if bytes.len() >= 8 && &bytes[..8] == b"\x89PNG\r\n\x1a\n" {
        ("PNG", true)
    } else if bytes.len() >= 3 && &bytes[..3] == b"\xff\xd8\xff" {
        ("JPEG", true)
    } else if bytes.len() >= 6 && &bytes[..6] == b"GIF87a" || bytes.len() >= 6 && &bytes[..6] == b"GIF89a" {
        ("GIF", true)
    } else if bytes.len() >= 4 && &bytes[..4] == b"RIFF" && bytes.len() >= 12 && &bytes[8..12] == b"WEBP" {
        ("WebP", false)
    } else if bytes.len() >= 2 && &bytes[..2] == b"BM" {
        ("BMP", false)
    } else if bytes.len() >= 12 && &bytes[..4] == b"\x00\x00\x01\x00" {
        ("ICO", false)
    } else {
        ("Unknown image format", false)
    }
}

fn try_get_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() >= 24 && &bytes[..8] == b"\x89PNG\r\n\x1a\n" {
        let w = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
        let h = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
        return Some((w, h));
    }
    if bytes.len() >= 4 && &bytes[..2] == b"\xff\xd8" {
        let mut pos = 2;
        while pos + 4 < bytes.len().min(4096) {
            if bytes[pos] == 0xFF {
                let marker = bytes[pos + 1];
                if marker == 0xC0 || marker == 0xC2 {
                    if pos + 9 < bytes.len() {
                        let h = u16::from_be_bytes([bytes[pos + 5], bytes[pos + 6]]);
                        let w = u16::from_be_bytes([bytes[pos + 7], bytes[pos + 8]]);
                        return Some((w as u32, h as u32));
                    }
                    break;
                }
                let seg_len = if pos + 4 < bytes.len() {
                    u16::from_be_bytes([bytes[pos + 2], bytes[pos + 3]]) as usize
                } else { 0 };
                if seg_len < 2 { break; }
                pos += 2 + seg_len;
            } else { break; }
        }
    }
    if bytes.len() >= 10 && (&bytes[..6] == b"GIF87a" || &bytes[..6] == b"GIF89a") {
        let w = u16::from_le_bytes([bytes[6], bytes[7]]);
        let h = u16::from_le_bytes([bytes[8], bytes[9]]);
        return Some((w as u32, h as u32));
    }
    if bytes.len() >= 26 && &bytes[..2] == b"BM" {
        let w = u32::from_le_bytes([bytes[18], bytes[19], bytes[20], bytes[21]]);
        let h = u32::from_le_bytes([bytes[22], bytes[23], bytes[24], bytes[25]]);
        return Some((w, h));
    }
    None
}
