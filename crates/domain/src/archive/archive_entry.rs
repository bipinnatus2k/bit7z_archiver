use chrono::{DateTime, Utc};

/// An entry (file or directory) inside a compressed archive.
#[derive(Debug, Clone, PartialEq)]
pub struct ArchiveEntry {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub compressed_size: u64,
    pub is_directory: bool,
    pub is_encrypted: bool,
    pub is_symlink: bool,
    pub modified: Option<DateTime<Utc>>,
    pub created: Option<DateTime<Utc>>,
    pub accessed: Option<DateTime<Utc>>,
    pub crc: Option<u32>,
    pub attributes: Option<u32>,
    pub posix_attrib: Option<u32>,
    pub host_os: Option<u8>,
    pub compression_method: Option<String>,
    pub comment: Option<String>,
    pub user: Option<String>,
    pub group: Option<String>,
    pub extension: Option<String>,
    pub hardlink: Option<String>,
    /// Original index in the archive (for preview/extraction).
    pub original_index: u32,
}

impl Default for ArchiveEntry {
    fn default() -> Self {
        Self {
            name: String::new(),
            path: String::new(),
            size: 0,
            compressed_size: 0,
            is_directory: false,
            is_encrypted: false,
            is_symlink: false,
            modified: None,
            created: None,
            accessed: None,
            crc: None,
            attributes: None,
            posix_attrib: None,
            host_os: None,
            compression_method: None,
            comment: None,
            user: None,
            group: None,
            extension: None,
            hardlink: None,
            original_index: 0,
        }
    }
}

impl ArchiveEntry {
    pub fn compression_ratio(&self) -> f64 {
        if self.size == 0 || self.compressed_size == 0 {
            0.0
        } else {
            1.0 - (self.compressed_size as f64 / self.size as f64)
        }
    }
}