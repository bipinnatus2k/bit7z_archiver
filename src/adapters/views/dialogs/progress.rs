use crate::theme::Theme;
use crossbeam::channel::{unbounded, Receiver};
use gpui::prelude::FluentBuilder as _;
use gpui::*;

pub enum ProgressEvent {
    Canceled,
}

pub struct ProgressDialog {
    pub title: String,
    pub message: String,
    pub current: u64,
    pub total: u64,
    pub is_complete: bool,
    pub error: Option<String>,
}

impl ProgressDialog {
    pub fn new(title: String) -> Self {
        Self { title, message: String::new(), current: 0, total: 1, is_complete: false, error: None }
    }

    pub fn open(cx: &mut AsyncApp, title: String) -> Receiver<ProgressEvent> {
        let (tx, rx) = unbounded::<ProgressEvent>();
        cx.spawn(async move |cx| {
            let _ = cx.open_window(
                WindowOptions {
                    titlebar: None,
                    window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                        point(px(200.), px(200.)),
                        size(px(400.), px(180.)),
                    ))),
                    window_background: WindowBackgroundAppearance::Opaque,
                    window_decorations: Some(WindowDecorations::Client),
                    ..Default::default()
                },
                move |window, cx| {
                    let dialog = cx.new(|_cx| ProgressDialog::new(title));
                    cx.new(|cx| gpui_component::Root::new(dialog, window, cx))
                },
            );
        }).detach();
        rx
    }

    pub fn update(&mut self, message: &str, current: u64, total: u64, err: Option<String>) {
        self.message = message.to_string();
        self.current = current;
        self.total = total;
        self.is_complete = err.is_some() || current >= total;
        self.error = err;
    }
}

impl Render for ProgressDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let pct = if self.total > 0 { (self.current as f64 / self.total as f64 * 100.0) as u32 } else { 0 };

        div().flex().flex_col().gap_3().p_4()
            .child(div().font_weight(FontWeight::BOLD).child(self.title.clone()))
            .child(div().text_sm().child(self.message.clone()))
            .child(
                div().flex().flex_row().gap_2().items_center()
                    .child(
                        div().flex_1().h(px(20.)).bg(theme.surface).rounded_md().overflow_hidden()
                            .child(
                                div().h_full().bg(theme.primary).rounded_md().w(px(pct as f32 * 4.0))
                            )
                    )
                    .child(div().text_sm().child(format!("{}%", pct)))
            )
            .when(self.error.is_some(), |el| {
                el.child(div().text_sm().text_color(theme.error).child(self.error.clone().unwrap_or_default()))
            })
    }
}
