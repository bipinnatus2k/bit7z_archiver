use std::path::PathBuf;

/// Trait for platform-specific file dialog operations.
pub trait DialogProvider: Send + Sync {
    fn pick_archive_file(&self) -> Option<PathBuf>;
    fn pick_folder(&self) -> Option<PathBuf>;
    fn pick_files(&self) -> Option<Vec<PathBuf>>;
}

/// Returns the default dialog provider using `rfd` (Rust File Dialogs).
pub fn dialog_provider() -> Box<dyn DialogProvider> {
    Box::new(RfdDialogProvider)
}

/// Open a file picker dialog for archive files using `rfd`.
pub fn pick_archive_file() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter(
            "Archives",
            &[
                "7z", "zip", "rar", "tar", "tar.gz", "tar.xz", "tar.bz2", "gz", "bz2", "xz",
            ],
        )
        .pick_file()
}

/// Open a folder picker dialog using `rfd`.
pub fn pick_folder() -> Option<PathBuf> {
    rfd::FileDialog::new().pick_folder()
}

/// Open a file picker dialog for selecting multiple files using `rfd`.
pub fn pick_files() -> Option<Vec<PathBuf>> {
    rfd::FileDialog::new().pick_files()
}

/// Dialog provider using `rfd` (Rust File Dialogs) for all platforms.
pub struct RfdDialogProvider;

impl DialogProvider for RfdDialogProvider {
    fn pick_archive_file(&self) -> Option<PathBuf> {
        pick_archive_file()
    }

    fn pick_folder(&self) -> Option<PathBuf> {
        pick_folder()
    }

    fn pick_files(&self) -> Option<Vec<PathBuf>> {
        pick_files()
    }
}
