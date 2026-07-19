#![feature(iter_collect_into)]

pub mod state_view;
pub mod virtual_list;
pub mod window_dialog;
pub mod ext_table;
pub mod empty_state;

pub fn init(cx: &mut gpui::App) {
    ext_table::init(cx);
    window_dialog::init(cx);
}
