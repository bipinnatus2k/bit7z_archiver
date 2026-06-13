use crate::domain::archive::ArchiveFormat;
use gpui::Global;
use serde::{Deserialize, Serialize};

/// Persistent user preferences.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preferences {
    #[serde(default)]
    pub window: WindowPrefs,
    #[serde(default)]
    pub archive: ArchivePrefs,
    #[serde(default)]
    pub preview: PreviewPrefs,
    #[serde(default)]
    pub ui: UiPrefs,
}

impl Global for Preferences {}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            window: WindowPrefs::default(),
            archive: ArchivePrefs::default(),
            preview: PreviewPrefs::default(),
            ui: UiPrefs::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowPrefs {
    pub width: u32,
    pub height: u32,
    pub maximized: bool,
    pub sidebar_ratio: f32,
}

impl Default for WindowPrefs {
    fn default() -> Self {
        Self {
            width: 1200,
            height: 800,
            maximized: false,
            sidebar_ratio: 0.25,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchivePrefs {
    pub default_format: ArchiveFormat,
    pub default_compression_level: u8,
    pub default_encrypt_filenames: bool,
    pub recent_files: Vec<String>,
}

impl Default for ArchivePrefs {
    fn default() -> Self {
        Self {
            default_format: ArchiveFormat::SevenZip,
            default_compression_level: 5,
            default_encrypt_filenames: true,
            recent_files: Vec::new(),
        }
    }
}

impl ArchivePrefs {
    pub fn add_recent(&mut self, path: String) {
        self.recent_files.retain(|p| p != &path);
        self.recent_files.insert(0, path);
        self.recent_files.truncate(10);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewPrefs {
    pub text_max_bytes: usize,
    pub hex_dump_bytes: usize,
    pub auto_preview: bool,
}

impl Default for PreviewPrefs {
    fn default() -> Self {
        Self {
            text_max_bytes: 1_048_576,
            hex_dump_bytes: 4096,
            auto_preview: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiPrefs {
    pub minimize_to_tray: bool,
    pub confirm_delete: bool,
    pub language: String,
}

impl Default for UiPrefs {
    fn default() -> Self {
        Self {
            minimize_to_tray: true,
            confirm_delete: true,
            language: "en".to_string(),
        }
    }
}

/// Repository trait for preferences persistence.
pub trait PreferencesRepository: Send + Sync {
    fn load(&self) -> Result<Preferences, PreferencesError>;
    fn save(&self, prefs: &Preferences) -> Result<(), PreferencesError>;
}

#[derive(Debug, thiserror::Error)]
pub enum PreferencesError {
    #[error("Failed to read preferences: {0}")]
    Read(#[source] std::io::Error),
    #[error("Failed to write preferences: {0}")]
    Write(#[source] std::io::Error),
    #[error("Failed to parse preferences: {0}")]
    Parse(#[source] serde_json::Error),
}
