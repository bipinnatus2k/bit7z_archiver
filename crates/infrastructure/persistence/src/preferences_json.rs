use bit7z_domain::preferences::*;
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

    pub fn path(&self) -> &PathBuf {
        &self.path
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

#[cfg(test)]
mod tests {
    use super::*;
    use bit7z_domain::archive::ArchiveFormat;
    use std::sync::atomic::{AtomicU32, Ordering};

    static TEST_COUNTER: AtomicU32 = AtomicU32::new(0);

    fn unique_test_path() -> PathBuf {
        let n = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        std::env::temp_dir().join(format!("bit7z_test_prefs_{}.json", n))
    }

    #[test]
    fn test_load_returns_default_when_missing() {
        let path = unique_test_path();
        let _ = std::fs::remove_file(&path);
        let repo = JsonPreferencesRepository { path: path.clone() };
        let prefs = repo.load().unwrap();
        assert_eq!(prefs.window.width, 1200);
        assert_eq!(prefs.archive.default_format, ArchiveFormat::SevenZip);
        assert!(prefs.archive.recent_files.is_empty());
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        let path = unique_test_path();
        let repo = JsonPreferencesRepository { path: path.clone() };
        let mut prefs = Preferences::default();
        prefs.window.width = 1920;
        prefs.window.height = 1080;
        prefs.archive.default_format = ArchiveFormat::Zip;
        prefs.archive.recent_files.push("test.7z".into());
        repo.save(&prefs).unwrap();

        let loaded = repo.load().unwrap();
        assert_eq!(loaded.window.width, 1920);
        assert_eq!(loaded.window.height, 1080);
        assert_eq!(loaded.archive.default_format, ArchiveFormat::Zip);
        assert_eq!(loaded.archive.recent_files, vec!["test.7z"]);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_load_corrupt_json_returns_error() {
        let path = unique_test_path();
        std::fs::write(&path, b"not valid json").unwrap();
        let repo = JsonPreferencesRepository { path: path.clone() };
        let result = repo.load();
        assert!(matches!(result, Err(PreferencesError::Parse(_))));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_save_creates_directory() {
        let deep_path = std::env::temp_dir()
            .join("bit7z_test_deep")
            .join("nested")
            .join("prefs.json");
        let repo = JsonPreferencesRepository {
            path: deep_path.clone(),
        };
        let prefs = Preferences::default();
        repo.save(&prefs).unwrap();
        assert!(deep_path.exists());
        let _ = std::fs::remove_dir_all(std::env::temp_dir().join("bit7z_test_deep"));
    }
}
