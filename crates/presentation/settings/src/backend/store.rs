use anyhow::Result;
use bit7z_domain::preferences::{Preferences, PreferencesRepository};
use bit7z_infra_persistence::preferences_json::JsonPreferencesRepository;
use gpui::{App, Global};
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;

pub struct SettingsStore {
    pub prefs: Preferences,
    pub path: PathBuf,
    repo: JsonPreferencesRepository,
    watcher: Option<RecommendedWatcher>,
    dirty: bool,
}

impl SettingsStore {
    pub fn new(repo: JsonPreferencesRepository) -> Self {
        let path = repo.path().to_path_buf();
        let prefs = repo.load().unwrap_or_default();
        SettingsStore {
            prefs,
            path,
            repo,
            watcher: None,
            dirty: false,
        }
    }

    pub fn init(cx: &mut App) {
        let repo = JsonPreferencesRepository::new();
        let mut store = Self::new(repo);
        store.start_watcher(cx);
        cx.set_global(store);
    }

    pub fn get(cx: &App) -> &Self {
        cx.global::<SettingsStore>()
    }

    pub fn get_mut(cx: &mut App) -> &mut Self {
        cx.global_mut::<SettingsStore>()
    }

    pub fn save(&self) -> Result<()> {
        self.repo.save(&self.prefs)?;
        Ok(())
    }

    pub fn update<F>(&mut self, f: F)
    where
        F: FnOnce(&mut Preferences),
    {
        f(&mut self.prefs);
        self.dirty = true;
    }

    pub fn update_and_save<F>(&mut self, f: F)
    where
        F: FnOnce(&mut Preferences),
    {
        f(&mut self.prefs);
        let _ = self.repo.save(&self.prefs);
    }

    fn start_watcher(&mut self, _cx: &mut App) {
        let path = self.path.clone();
        let mut watcher = RecommendedWatcher::new(
            move |event: notify::Result<notify::Event>| {
                if let Ok(event) = event {
                    if event.kind.is_modify() || event.kind.is_create() {
                        log::info!("Preferences file changed");
                    }
                }
            },
            Config::default(),
        )
        .expect("Failed to create file watcher");

        let _ = watcher.watch(&path, RecursiveMode::NonRecursive);
        self.watcher = Some(watcher);
    }
}

impl Global for SettingsStore {}
