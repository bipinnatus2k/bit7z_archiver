pub mod backend;
pub mod controls;
pub mod field;
pub mod pages;
pub mod panels;
pub mod renderer;

pub use backend::store::SettingsStore;
pub use gpui_component::setting::*;
pub use gpui_component::ActiveTheme;

use gpui::*;

use std::sync::OnceLock;

use bit7z_rt_globals::PreferencesGlobal;

/// Global entity ID for the settings panel, so controls can trigger re-renders.
static PANEL_ENTITY_ID: OnceLock<EntityId> = OnceLock::new();

pub(crate) fn notify_panel(cx: &mut App) {
    if let Some(&id) = PANEL_ENTITY_ID.get() {
        cx.notify(id);
    }
}

pub fn register_panel_entity(entity: &Entity<panels::SettingsPanel>) {
    PANEL_ENTITY_ID.set(entity.entity_id()).ok();
}

/// Initialize the settings subsystem.
pub fn init(cx: &mut App) {
    SettingsStore::init(cx);
    renderer::SettingFieldRenderer::init(cx);

    // Keep backward compatibility: set PreferencesGlobal so legacy code works
    let prefs = SettingsStore::get(cx).prefs.clone();
    cx.set_global(bit7z_rt_globals::PreferencesGlobal(prefs));
}
