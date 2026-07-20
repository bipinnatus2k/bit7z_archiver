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

/// Find the 7-Zip shared library on the current platform.
pub fn find_7z_library() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let candidates = [
            r"C:\Program Files\7-Zip\7z.dll",
            r"C:\Program Files (x86)\7-Zip\7z.dll",
        ];
        for p in &candidates {
            if std::path::Path::new(p).exists() {
                return Some(PathBuf::from(p));
            }
        }
        if let Ok(paths) = std::env::var("PATH") {
            for dir in std::env::split_paths(&paths) {
                let candidate = dir.join("7z.dll");
                if candidate.exists() {
                    return Some(candidate);
                }
            }
        }
    }
    #[cfg(target_os = "linux")]
    {
        let lib_names = ["lib7zip.so", "libp7zip.so", "lib7z.so"];
        for name in &lib_names {
            for dir in ["/usr/lib", "/usr/local/lib", "/usr/lib/x86_64-linux-gnu"] {
                let candidate = PathBuf::from(dir).join(name);
                if candidate.exists() {
                    return Some(candidate);
                }
            }
        }
    }
    None
}

/// Open a file picker dialog for archive files using `rfd`.
pub fn pick_archive_file() -> Option<PathBuf> {
    println!("call rfd");
    rfd::FileDialog::new()
        .add_filter(
            "All Archive Format",
            &[
                "7z", "zip", "rar", "tar", "tar.gz", "tar.xz", "tar.bz2", "gz", "bz2", "xz",
            ],
        )
        .add_filter("All Files",&["*"])
        .set_title("Open archive file")
        .pick_file()
}

/// Open a folder picker dialog using `rfd`.
pub fn pick_folder() -> Option<std::path::PathBuf> {
    rfd::FileDialog::new().pick_folder()
}

/// Open a file picker dialog for selecting multiple files using `rfd`.
pub fn pick_files() -> Option<Vec<std::path::PathBuf>> {
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
