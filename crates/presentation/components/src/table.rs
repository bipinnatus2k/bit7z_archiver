use bit7z_pres_theme::Theme;
use gpui::*;
use std::rc::Rc;

pub struct Column {
    pub label: SharedString,
    pub width: Option<Pixels>,
}

impl Column {
    pub fn new(label: impl Into<SharedString>, width: Option<Pixels>) -> Self {
        Self { label: label.into(), width }
    }
}

pub fn sortable_header(
    columns: &[Column],
    sorted_column: Option<u32>,
    sort_ascending: bool,
    on_sort: Rc<dyn Fn(u32, &mut App)>,
    cx: &App,
) -> impl IntoElement {
    let theme = cx.global::<Theme>();
    let mut row = div().flex().flex_row().gap_2().px_2().py_1()
        .bg(theme.surface).font_weight(FontWeight::BOLD);

    for (i, col) in columns.iter().enumerate() {
        let is_sorted = sorted_column == Some(i as u32);
        let label: SharedString = if is_sorted {
            if sort_ascending {
                SharedString::from(format!("{} {}\u{25b2}", col.label, ""))
            } else {
                SharedString::from(format!("{} {}\u{25bc}", col.label, ""))
            }
        } else {
            col.label.clone()
        };

        let os = on_sort.clone();
        let idx = i as u32;
        let mut cell = div().cursor_pointer();
        if let Some(w) = col.width {
            cell = cell.w(w);
        } else {
            cell = cell.flex_1().min_w(px(0.));
        }
        cell = cell
            .child(label)
            .on_mouse_down(MouseButton::Left, move |_: &MouseDownEvent, _window: &mut Window, app_cx: &mut App| {
                os(idx, app_cx);
            });
        row = row.child(cell);
    }

    row
}
