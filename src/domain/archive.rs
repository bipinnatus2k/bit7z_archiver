use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
    pub crc: Option<u32>,
}

impl ArchiveEntry {
    pub fn compression_ratio(&self) -> f64 {
        if self.size == 0 {
            0.0
        } else {
            1.0 - (self.compressed_size as f64 / self.size as f64)
        }
    }
}

/// Supported archive formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArchiveFormat {
    #[serde(rename = "7z")]
    SevenZip,
    #[serde(rename = "zip")]
    Zip,
    #[serde(rename = "tar")]
    Tar,
    #[serde(rename = "tar.gz")]
    TarGz,
    #[serde(rename = "tar.xz")]
    TarXz,
    #[serde(rename = "tar.bz2")]
    TarBz2,
    #[serde(rename = "rar")]
    Rar,
}

impl ArchiveFormat {
    pub fn extension(&self) -> &str {
        match self {
            ArchiveFormat::SevenZip => "7z",
            ArchiveFormat::Zip => "zip",
            ArchiveFormat::Tar => "tar",
            ArchiveFormat::TarGz => "tar.gz",
            ArchiveFormat::TarXz => "tar.xz",
            ArchiveFormat::TarBz2 => "tar.bz2",
            ArchiveFormat::Rar => "rar",
        }
    }

    pub fn supports_encryption(&self) -> bool {
        matches!(self, ArchiveFormat::SevenZip | ArchiveFormat::Zip)
    }

    pub fn supports_encrypted_filenames(&self) -> bool {
        matches!(self, ArchiveFormat::SevenZip)
    }

    pub fn is_writable(&self) -> bool {
        !matches!(self, ArchiveFormat::Rar)
    }

    pub fn supports_compression_level(&self) -> bool {
        !matches!(self, ArchiveFormat::Tar)
    }

    pub fn display_name(&self) -> &str {
        match self {
            ArchiveFormat::SevenZip => "7z",
            ArchiveFormat::Zip => "Zip",
            ArchiveFormat::Tar => "Tar",
            ArchiveFormat::TarGz => "Tar.gz",
            ArchiveFormat::TarXz => "Tar.xz",
            ArchiveFormat::TarBz2 => "Tar.bz2",
            ArchiveFormat::Rar => "Rar",
        }
    }
}

/// Opaque handle to an opened archive (wraps a raw C++ pointer).
#[derive(Debug, Clone)]
pub struct ArchiveHandle {
    pub(crate) raw: *mut std::ffi::c_void,
    pub(crate) is_writer: bool,
}

// SAFETY: ArchiveHandle is only accessed from the main GPUI thread.
unsafe impl Send for ArchiveHandle {}
unsafe impl Sync for ArchiveHandle {}

impl ArchiveHandle {
    pub fn new_reader(raw: *mut std::ffi::c_void) -> Self {
        Self { raw, is_writer: false }
    }

    pub fn new_writer(raw: *mut std::ffi::c_void) -> Self {
        Self { raw, is_writer: true }
    }
}

/// Paginated result for large archives.
#[derive(Debug, Clone)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub offset: usize,
    pub total: Option<usize>,
}

impl<T> Page<T> {
    pub fn new(items: Vec<T>, offset: usize, total: Option<usize>) -> Self {
        Self { items, offset, total }
    }
}

/// Encryption method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EncryptionMethod {
    #[serde(rename = "aes256")]
    Aes256,
    #[serde(rename = "zipcrypto")]
    ZipCrypto,
}

impl EncryptionMethod {
    pub fn display_name(&self) -> &str {
        match self {
            EncryptionMethod::Aes256 => "AES-256",
            EncryptionMethod::ZipCrypto => "ZipCrypto",
        }
    }

    pub fn available_for(format: ArchiveFormat) -> Vec<Self> {
        match format {
            ArchiveFormat::SevenZip => vec![EncryptionMethod::Aes256],
            ArchiveFormat::Zip => vec![EncryptionMethod::ZipCrypto],
            _ => vec![],
        }
    }
}

/// Configuration for creating encrypted archives.
#[derive(Debug, Clone)]
pub struct EncryptionConfig {
    pub password: String,
    pub method: EncryptionMethod,
    pub encrypt_filenames: bool,
}

/// Result of an archive integrity test.
#[derive(Debug, Clone)]
pub struct TestResult {
    pub total: usize,
    pub passed: usize,
    pub failures: Vec<TestFailure>,
}

#[derive(Debug, Clone)]
pub struct TestFailure {
    pub entry_path: String,
    pub error: String,
}

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
