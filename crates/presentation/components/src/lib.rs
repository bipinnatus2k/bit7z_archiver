pub mod state_view;
pub mod table;
pub mod virtual_list;
pub mod window_dialog;

pub mod ext_table;
pub use ext_table::*;

pub(crate) fn init(cx: &mut gpui::App) {
    ext_table::init(cx);
    window_dialog::init(cx);
}
