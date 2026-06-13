pub use crate::domain::repository::ArchiveError;
pub use crate::domain::preferences::PreferencesError;

/// Application-level error covering all layers.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    Archive(#[from] ArchiveError),

    #[error("{0}")]
    Preferences(#[from] PreferencesError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}
