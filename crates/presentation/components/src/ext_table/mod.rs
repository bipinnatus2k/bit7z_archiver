use gpui::{px, App, Pixels};
use gpui_component::*;

mod column;
mod data_table;
mod delegate;
mod loading;
mod state;
mod table;
mod actions;

pub use column::*;
pub use data_table::*;
pub use delegate::*;
pub use state::*;

pub fn init(cx: &mut App) {
    data_table::init(cx);
}

#[inline]
pub(crate) fn measure_enable_gpui() -> bool {
    std::env::var("ZED_MEASUREMENTS").is_ok() || std::env::var("GPUI_MEASUREMENTS").is_ok()
}

const WIDTH: Pixels = px(4. * 2. + 8.);
