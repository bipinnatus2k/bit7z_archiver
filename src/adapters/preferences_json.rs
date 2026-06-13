use crate::domain::preferences::*;
use std::path::PathBuf;

pub struct JsonPreferencesRepository {
    path: PathBuf,
}

impl JsonPreferencesRepository {
    pub fn new() -> Self {
        let path = Self::default_path();
        Self { path }
    }

    fn default_path() -> PathBuf {
        if let Some(proj_dirs) = directories::ProjectDirs::from("com", "bit7z", "archiver") {
            proj_dirs.config_dir().join("preferences.json")
        } else {
            PathBuf::from("preferences.json")
        }
    }

    fn ensure_dir(&self) -> Result<(), PreferencesError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(PreferencesError::Write)?;
        }
        Ok(())
    }
}

impl PreferencesRepository for JsonPreferencesRepository {
    fn load(&self) -> Result<Preferences, PreferencesError> {
        if !self.path.exists() {
            return Ok(Preferences::default());
        }
        let data = std::fs::read_to_string(&self.path).map_err(PreferencesError::Read)?;
        serde_json::from_str(&data).map_err(PreferencesError::Parse)
    }

    fn save(&self, prefs: &Preferences) -> Result<(), PreferencesError> {
        self.ensure_dir()?;
        let data = serde_json::to_string_pretty(prefs).map_err(PreferencesError::Parse)?;
        std::fs::write(&self.path, data).map_err(PreferencesError::Write)
    }
}
