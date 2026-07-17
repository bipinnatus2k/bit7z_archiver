pub mod buttons;
pub mod command;
pub mod gallery;
pub mod group;
pub mod model;
pub mod panel;
pub mod quick_access;
pub mod ribbon;
pub mod search;
pub mod tab_bar;
pub mod theme;

use gpui::App;
pub use command::{Command, CommandContext, CommandId, CommandRegistry, ContextPredicate};
pub use model::{
    GalleryItem, QuickAccessItem, RibbonDisplayMode, RibbonGroup, RibbonItemKind, RibbonModel,
    RibbonTab,
};
pub use ribbon::Ribbon;
pub use theme::{ribbon_theme, ribbon_theme_or_default, set_ribbon_theme, RibbonTheme};

pub fn init(cx: &mut App) {
    // Set the ANSYS-blue theme globally
    set_ribbon_theme(cx, RibbonTheme::default());

}
