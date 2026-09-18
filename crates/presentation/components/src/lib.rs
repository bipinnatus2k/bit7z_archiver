pub mod state_view;
pub mod virtual_list;
pub mod window_dialog;
pub mod toolbar;
pub mod ext_table;
pub mod empty_state;
mod component_prelude;
mod file_input;
mod prelude;
mod media_object;
mod item;
mod styles;

// pub use components::*;
pub use prelude::*;
pub use styles::*;
// pub use traits::animation_ext::*;

pub fn init(cx: &mut gpui::App) {
    ext_table::init(cx);
    window_dialog::init(cx);
}
