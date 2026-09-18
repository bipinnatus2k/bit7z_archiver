use std::path::PathBuf;
use crate::archive::archive_format::ArchiveFormat;
use crate::archive::session::next_archive_id;

/// Opaque handle to an opened archive.
/// The raw FFI pointer is managed by the adapter layer (Bit7zRepository).
///
/// Note: ArchiveHandle is Clone because it's just metadata (id, path, format).
/// The actual resource is tracked by id in the repository. After close() is called,
/// the repository removes the entry, and subsequent operations on any clone will
/// fail with "handle not found" - this is safe and expected behavior.
#[must_use = "ArchiveHandle tracks a C++ resource; dropping it loses the reference"]
#[derive(Debug, Clone)]
pub struct ArchiveHandle {
    pub id: u64,
    pub path: Option<PathBuf>,
    pub(crate) format: Option<ArchiveFormat>,
    pub(crate) is_header_encrypted: bool,
    pub(crate) has_encrypted_items: bool,
}

impl ArchiveHandle {
    pub fn new_reader() -> Self {
        Self {
            id: next_archive_id(),
            path: None,
            format: None,
            is_header_encrypted: false,
            has_encrypted_items: false,
        }
    }

    pub fn new_writer() -> Self {
        Self {
            id: next_archive_id(),
            path: None,
            format: None,
            is_header_encrypted: false,
            has_encrypted_items: false,
        }
    }

    pub fn with_path(mut self, path: PathBuf) -> Self {
        self.path = Some(path);
        self
    }

    pub fn with_format(mut self, format: ArchiveFormat) -> Self {
        self.format = Some(format);
        self
    }

    pub fn with_format_opt(mut self, format: Option<ArchiveFormat>) -> Self {
        self.format = format;
        self
    }

    pub fn is_header_encrypted(&self) -> bool {
        self.is_header_encrypted
    }

    pub fn has_encrypted_items(&self) -> bool {
        self.has_encrypted_items
    }

    pub fn set_encryption_info(&mut self, is_header_encrypted: bool, has_encrypted_items: bool) {
        self.is_header_encrypted = is_header_encrypted;
        self.has_encrypted_items = has_encrypted_items;
    }
}