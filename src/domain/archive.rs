use chrono::{DateTime, Utc};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ARCHIVE_ID: AtomicU64 = AtomicU64::new(1);

pub(crate) fn next_archive_id() -> u64 {
    NEXT_ARCHIVE_ID.fetch_add(1, Ordering::Relaxed)
}

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

/// Opaque handle to an opened archive.
/// The raw FFI pointer is managed by the adapter layer (Bit7zRepository).
///
/// Note: ArchiveHandle is Clone because it's just metadata (id, path, format).
/// The actual resource is tracked by id in the repository. After close() is called,
/// the repository removes the entry, and subsequent operations on any clone will
/// fail with "handle not found" - this is safe and expected behavior.
#[derive(Debug, Clone)]
pub struct ArchiveHandle {
    pub(crate) id: u64,
    pub(crate) is_writer: bool,
    pub(crate) path: Option<PathBuf>,
    pub(crate) format: Option<ArchiveFormat>,
    pub(crate) is_header_encrypted: bool,
    pub(crate) has_encrypted_items: bool,
}

impl ArchiveHandle {
    pub fn new_reader() -> Self {
        Self {
            id: next_archive_id(),
            is_writer: false,
            path: None,
            format: None,
            is_header_encrypted: false,
            has_encrypted_items: false,
        }
    }

    pub fn new_writer() -> Self {
        Self {
            id: next_archive_id(),
            is_writer: true,
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

/// A password that is zeroized on drop and redacted in debug output.
#[derive(Clone)]
pub struct Password(SecretString);

impl Password {
    pub fn new(password: impl Into<String>) -> Self {
        Self(password.into().into())
    }

    pub fn as_str(&self) -> &str {
        self.0.expose_secret()
    }

    pub fn is_empty(&self) -> bool {
        self.0.expose_secret().is_empty()
    }
}

impl fmt::Debug for Password {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Password([redacted])")
    }
}

/// Configuration for creating encrypted archives.
#[derive(Debug, Clone)]
pub struct EncryptionConfig {
    pub password: Password,
    pub method: EncryptionMethod,
    pub encrypt_filenames: bool,
}

/// Overwrite mode for extraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverwriteMode {
    Ask,
    Overwrite,
    Skip,
    RenameExtracted,
}

impl OverwriteMode {
    pub fn label(&self) -> &'static str {
        match self {
            OverwriteMode::Ask => "Ask",
            OverwriteMode::Overwrite => "Overwrite",
            OverwriteMode::Skip => "Skip",
            OverwriteMode::RenameExtracted => "Rename extracted",
        }
    }

    pub fn all() -> Vec<OverwriteMode> {
        vec![OverwriteMode::Ask, OverwriteMode::Overwrite, OverwriteMode::Skip, OverwriteMode::RenameExtracted]
    }
    
    pub fn index(&self) -> &'static i32 {
        match self {
            OverwriteMode::Ask => &0,
            OverwriteMode::Overwrite => &1,
            OverwriteMode::Skip => &2,
            OverwriteMode::RenameExtracted => &3
        }
    }
}

/// Result of an archive integrity test.
#[derive(Debug, Clone)]
pub struct TestResult {
    pub total: usize,
    pub passed: usize,
    pub failed: Vec<TestFailure>,
}

#[derive(Debug, Clone)]
pub struct TestFailure {
    pub entry_path: String,
    pub error: String,
    pub index: usize,
    pub path: String,
    pub reason: TestFailureReason,
}

#[derive(Debug, Clone)]
pub enum TestFailureReason {
    CrcMismatch { expected: u32, actual: u32 },
    ReadError(String),
    UnsupportedOperation,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compression_ratio_zero_size() {
        let entry = ArchiveEntry {
            name: String::new(),
            path: String::new(),
            size: 0,
            compressed_size: 0,
            is_directory: false,
            is_encrypted: false,
            is_symlink: false,
            modified: None,
            original_index: 0,
            ..Default::default()
        };
        assert_eq!(entry.compression_ratio(), 0.0);
    }

    #[test]
    fn test_compression_ratio_no_compression() {
        let entry = ArchiveEntry {
            name: "test.bin".into(),
            path: "test.bin".into(),
            size: 1000,
            compressed_size: 1000,
            ..Default::default()
        };
        assert_eq!(entry.compression_ratio(), 0.0);
    }

    #[test]
    fn test_compression_ratio_positive() {
        let entry = ArchiveEntry {
            name: "test.txt".into(),
            path: "test.txt".into(),
            size: 1000,
            compressed_size: 300,
            ..Default::default()
        };
        let ratio = entry.compression_ratio();
        assert!((ratio - 0.7).abs() < 0.001, "Expected ~0.7, got {}", ratio);
    }

    #[test]
    fn test_archive_format_extension() {
        assert_eq!(ArchiveFormat::SevenZip.extension(), "7z");
        assert_eq!(ArchiveFormat::Zip.extension(), "zip");
        assert_eq!(ArchiveFormat::Tar.extension(), "tar");
    }

    #[test]
    fn test_archive_format_supports_encryption() {
        assert!(ArchiveFormat::SevenZip.supports_encryption());
        assert!(ArchiveFormat::Zip.supports_encryption());
        assert!(!ArchiveFormat::Tar.supports_encryption());
        assert!(!ArchiveFormat::Rar.supports_encryption());
    }

    #[test]
    fn test_archive_format_is_writable() {
        assert!(ArchiveFormat::SevenZip.is_writable());
        assert!(!ArchiveFormat::Rar.is_writable());
    }

    #[test]
    fn test_archive_format_display_name() {
        assert_eq!(ArchiveFormat::SevenZip.display_name(), "7z");
        assert_eq!(ArchiveFormat::TarGz.display_name(), "Tar.gz");
    }

    mod test_result_logic {
        use super::*;

        #[test]
        fn test_all_passed() {
            let r = TestResult { total: 10, passed: 10, failed: vec![] };
            assert_eq!(r.total, 10);
            assert_eq!(r.passed, 10);
            assert!(r.failed.is_empty());
        }

        #[test]
        fn test_all_failed() {
            let failures: Vec<TestFailure> = (0..3).map(|i| TestFailure {
                entry_path: format!("f{}.txt", i),
                error: "CRC mismatch".into(),
                index: i,
                path: format!("f{}.txt", i),
                reason: TestFailureReason::CrcMismatch { expected: i as u32, actual: 99 },
            }).collect();
            let r = TestResult { total: 3, passed: 0, failed: failures };
            assert_eq!(r.total, 3);
            assert_eq!(r.passed, 0);
            assert_eq!(r.failed.len(), 3);
        }

        #[test]
        fn test_mixed() {
            let r = TestResult {
                total: 5,
                passed: 4,
                failed: vec![TestFailure {
                    entry_path: "bad.txt".into(), error: "err".into(),
                    index: 1, path: "bad.txt".into(),
                    reason: TestFailureReason::ReadError("err".into()),
                }],
            };
            assert_eq!(r.passed, r.total - r.failed.len());
        }

        #[test]
        fn test_empty() {
            let r = TestResult { total: 0, passed: 0, failed: vec![] };
            assert_eq!(r.total, 0);
            assert_eq!(r.passed, 0);
        }
    }
}
