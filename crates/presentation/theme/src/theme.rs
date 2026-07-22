
use gpui::{Action, App, SharedString, Window};
use gpui_component::{Theme, ThemeMode, ThemeRegistry, scroll::ScrollbarShow};
use serde::{Deserialize, Serialize};
use gpui_component::ActiveTheme;
use bit7z_domain::preferences::DarkMode;
use bit7z_pres_settings::SettingsStore;
#[cfg(feature = "embedded-theme")]
use crate::embedded_themes;

// const STATE_FILE: &str = "target/state.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ThemeState {
    theme: SharedString,
    dark_mode: ThemeMode,
    scrollbar_show: Option<ScrollbarShow>,
}

impl ThemeState {
    
    pub fn new(theme: Option<SharedString>, dark_mode: Option<ThemeMode>, scrollbar_show: Option<ScrollbarShow>) -> Self {
        Self {
            theme: if theme.is_some() { theme.unwrap() } else { "Default Light".into() },
            dark_mode: if dark_mode.is_some() { dark_mode.unwrap() } else { ThemeMode::Light },
            scrollbar_show: if scrollbar_show.is_some() { scrollbar_show } else {None},
        }
    }
}

impl Default for ThemeState {
    fn default() -> Self {
        Self {
            theme: "Default Light".into(),
            dark_mode: ThemeMode::Light,
            scrollbar_show: None,
        }
    }
}

pub fn init(w: &mut Window, cx: &mut App) {
    #[cfg(feature = "embedded-theme")]
    {
        tracing::info!("Loading embedded themes for WASM...");
        let embedded = embedded_themes::embedded_themes();
        let registry = ThemeRegistry::global_mut(cx);

        for (name, content) in embedded {
            if let Err(e) = registry.load_themes_from_str(content) {
                tracing::error!("Failed to load embedded theme {}: {}", name, e);
            } else {
                tracing::info!("Loaded embedded theme: {}", name);
            }
        }
    }

    // Apply persisted theme
    let prefs = SettingsStore::get_mut(cx);
    let theme_mode = match prefs.prefs.ui.night_mode {
        DarkMode::Light => {
            ThemeMode::Light
        }
        DarkMode::Dark => {
            ThemeMode::Dark
        }
        DarkMode::System => {
            if w.appearance() == gpui::WindowAppearance::Dark {
                ThemeMode::Dark
            } else {
                ThemeMode::Light
            }
        }
    };

    let state = if cfg!(not(feature = "embedded-theme")) {
        ThemeState::new(None, Some(theme_mode), None)
    } else {
        ThemeState::default()
    };

    #[cfg(not(feature = "embedded-theme"))]
    if let Err(err) =
        ThemeRegistry::watch_dir(std::path::PathBuf::from("./themes"), cx, move |cx| {
            if let Some(theme) = ThemeRegistry::global(cx)
                .themes()
                .get(&state.theme)
                .cloned()
            {
                Theme::global_mut(cx).apply_config(&theme);
            }
        })
    {
        tracing::error!("Failed to watch themes directory: {}", err);
    }

    if let Some(scrollbar_show) = state.scrollbar_show {
        Theme::global_mut(cx).scrollbar_show = scrollbar_show;
    }
    cx.refresh_windows();

    #[cfg(not(target_family = "wasm"))]
    cx.observe_global::<Theme>(|cx| {
        let state = ThemeState {
            dark_mode: cx.theme().mode.clone(),
            theme: cx.theme().theme_name().clone(),
            scrollbar_show: Some(cx.theme().scrollbar_show),
        };

        // SettingsStore::update_and_save(prefs,|x| {
        //     // x.ui.night_mode = state.dark_mode.into();
        //     // x
        // })
    })
        .detach();

    cx.on_action(|switch: &SwitchTheme, cx| {
        let theme_name = switch.0.clone();
        if let Some(theme_config) = ThemeRegistry::global(cx).themes().get(&theme_name).cloned() {
            Theme::global_mut(cx).apply_config(&theme_config);
        }
        cx.refresh_windows();
    });
    cx.on_action(|switch: &SwitchThemeMode, cx| {
        let mode = switch.0;
        Theme::change(mode, None, cx);
        cx.refresh_windows();
    });
}

#[derive(Action, Clone, PartialEq)]
#[action(namespace = themes, no_json)]
pub struct SwitchTheme(pub SharedString);

#[derive(Action, Clone, PartialEq)]
#[action(namespace = themes, no_json)]
pub struct SwitchThemeMode(pub ThemeMode);