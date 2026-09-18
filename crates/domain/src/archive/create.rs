use std::path::PathBuf;

/// Output file info for creation.
#[derive(Debug, Clone)]
pub struct CreateFileItem {
    pub path: PathBuf,
    pub is_directory: bool,
    pub size: Option<u64>,
}

impl CreateFileItem {
    pub fn display_name(&self) -> String {
        self.path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| self.path.to_string_lossy().to_string())
    }
}
