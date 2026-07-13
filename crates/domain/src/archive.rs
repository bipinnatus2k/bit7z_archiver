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
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) size: u64,
    pub(crate) compressed_size: u64,
    pub(crate) is_directory: bool,
    pub(crate) is_encrypted: bool,
    pub(crate) is_symlink: bool,
    pub(crate) modified: Option<DateTime<Utc>>,
    pub(crate) created: Option<DateTime<Utc>>,
    pub(crate) accessed: Option<DateTime<Utc>>,
    pub(crate) crc: Option<u32>,
    pub(crate) attributes: Option<u32>,
    pub(crate) posix_attrib: Option<u32>,
    pub(crate) host_os: Option<u8>,
    pub(crate) compression_method: Option<String>,
    pub(crate) comment: Option<String>,
    pub(crate) user: Option<String>,
    pub(crate) group: Option<String>,
    pub(crate) extension: Option<String>,
    pub(crate) hardlink: Option<String>,
    /// Original index in the archive (for preview/extraction).
    pub(crate) original_index: u32,
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
    pub fn name(&self) -> &str { &self.name }
    pub fn path(&self) -> &str { &self.path }
    pub fn size(&self) -> u64 { self.size }
    pub fn compressed_size(&self) -> u64 { self.compressed_size }
    pub fn is_directory(&self) -> bool { self.is_directory }
    pub fn is_encrypted(&self) -> bool { self.is_encrypted }
    pub fn is_symlink(&self) -> bool { self.is_symlink }
    pub fn original_index(&self) -> u32 { self.original_index }
    pub fn mtime(&self) -> Option<i64> { self.modified.map(|dt| dt.timestamp()) }
    pub fn ctime(&self) -> Option<i64> { self.created.map(|dt| dt.timestamp()) }
    pub fn atime(&self) -> Option<i64> { self.accessed.map(|dt| dt.timestamp()) }
    pub fn crc(&self) -> u32 { self.crc.unwrap_or(0) }
    pub fn attributes(&self) -> u32 { self.attributes.unwrap_or(0) }
    pub fn posix_attrib(&self) -> u32 { self.posix_attrib.unwrap_or(0) }
    pub fn host_os(&self) -> u8 { self.host_os.unwrap_or(0) }
    pub fn compression_method(&self) -> Option<&str> { self.compression_method.as_deref() }
    pub fn comment(&self) -> Option<&str> { self.comment.as_deref() }
    pub fn user(&self) -> Option<&str> { self.user.as_deref() }
    pub fn group(&self) -> Option<&str> { self.group.as_deref() }
    pub fn extension(&self) -> Option<&str> { self.extension.as_deref() }
    pub fn hardlink(&self) -> Option<&str> { self.hardlink.as_deref() }

    pub fn compression_ratio(&self) -> f64 {
        if self.size == 0 {
            0.0
        } else {
            1.0 - (self.compressed_size as f64 / self.size as f64)
        }
    }

    pub fn builder() -> ArchiveEntryBuilder {
        ArchiveEntryBuilder::default()
    }
}

pub struct ArchiveEntryBuilder {
    name: String,
    path: String,
    size: u64,
    compressed_size: u64,
    is_directory: bool,
    is_encrypted: bool,
    is_symlink: bool,
    modified: Option<DateTime<Utc>>,
    created: Option<DateTime<Utc>>,
    accessed: Option<DateTime<Utc>>,
    crc: Option<u32>,
    attributes: Option<u32>,
    posix_attrib: Option<u32>,
    host_os: Option<u8>,
    compression_method: Option<String>,
    comment: Option<String>,
    user: Option<String>,
    group: Option<String>,
    extension: Option<String>,
    hardlink: Option<String>,
    original_index: u32,
}

impl Default for ArchiveEntryBuilder {
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

impl ArchiveEntryBuilder {
    pub fn name(mut self, v: String) -> Self { self.name = v; self }
    pub fn path(mut self, v: String) -> Self { self.path = v; self }
    pub fn size(mut self, v: u64) -> Self { self.size = v; self }
    pub fn compressed_size(mut self, v: u64) -> Self { self.compressed_size = v; self }
    pub fn is_directory(mut self, v: bool) -> Self { self.is_directory = v; self }
    pub fn is_encrypted(mut self, v: bool) -> Self { self.is_encrypted = v; self }
    pub fn is_symlink(mut self, v: bool) -> Self { self.is_symlink = v; self }
    pub fn modified(mut self, v: Option<DateTime<Utc>>) -> Self { self.modified = v; self }
    pub fn created(mut self, v: Option<DateTime<Utc>>) -> Self { self.created = v; self }
    pub fn accessed(mut self, v: Option<DateTime<Utc>>) -> Self { self.accessed = v; self }
    pub fn crc(mut self, v: Option<u32>) -> Self { self.crc = v; self }
    pub fn attributes(mut self, v: Option<u32>) -> Self { self.attributes = v; self }
    pub fn posix_attrib(mut self, v: Option<u32>) -> Self { self.posix_attrib = v; self }
    pub fn host_os(mut self, v: Option<u8>) -> Self { self.host_os = v; self }
    pub fn compression_method(mut self, v: Option<String>) -> Self { self.compression_method = v; self }
    pub fn comment(mut self, v: Option<String>) -> Self { self.comment = v; self }
    pub fn user(mut self, v: Option<String>) -> Self { self.user = v; self }
    pub fn group(mut self, v: Option<String>) -> Self { self.group = v; self }
    pub fn extension(mut self, v: Option<String>) -> Self { self.extension = v; self }
    pub fn hardlink(mut self, v: Option<String>) -> Self { self.hardlink = v; self }
    pub fn original_index(mut self, v: u32) -> Self { self.original_index = v; self }

    pub fn build(self) -> ArchiveEntry {
        ArchiveEntry {
            name: self.name,
            path: self.path,
            size: self.size,
            compressed_size: self.compressed_size,
            is_directory: self.is_directory,
            is_encrypted: self.is_encrypted,
            is_symlink: self.is_symlink,
            modified: self.modified,
            created: self.created,
            accessed: self.accessed,
            crc: self.crc,
            attributes: self.attributes,
            posix_attrib: self.posix_attrib,
            host_os: self.host_os,
            compression_method: self.compression_method,
            comment: self.comment,
            user: self.user,
            group: self.group,
            extension: self.extension,
            hardlink: self.hardlink,
            original_index: self.original_index,
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

    pub fn all_support_compress() -> Vec<ArchiveFormat> {
        vec![
            ArchiveFormat::SevenZip,
            ArchiveFormat::Zip,
            ArchiveFormat::Tar,
            ArchiveFormat::TarGz,
            ArchiveFormat::TarXz,
            ArchiveFormat::TarBz2,
            // ArchiveFormat::Rar => "Rar",
        ]
    }
}

/// Opaque handle to an opened archive.
/// The raw FFI pointer is managed by the adapter layer (Bit7zRepository).
///
/// Note: ArchiveHandle is Clone because it's just metadata (id, path, format).
/// The actual resource is tracked by id in the repository. After close() is called,
/// the repository removes the entry, and subsequent operations on any clone will
/// fail with "handle not found" - this is safe and expected behavior.
#[must_use = "ArchiveHandle tracks a C++ resource; dropping it loses the reference"]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct ArchiveId(pub(crate) u64);

/// Opaque handle to an opened archive.
#[derive(Debug, Clone)]
pub struct ArchiveHandle {
    pub(crate) id: ArchiveId,
    pub(crate) path: Option<PathBuf>,
    pub(crate) format: Option<ArchiveFormat>,
    pub(crate) is_header_encrypted: bool,
    pub(crate) has_encrypted_items: bool,
}

impl ArchiveHandle {
    /// Construct a new handle with the given raw id.
    ///
    /// Only `ArchiveRepository` implementations (e.g. `RepoSupervisor`) should call this;
    /// a handle constructed with an unregistered id will return `NotOpen` from any operation.
    /// Fields remain private to prevent struct-literal construction.
    pub fn new(id: u64) -> Self {
        Self {
            id: ArchiveId(id),
            path: None,
            format: None,
            is_header_encrypted: false,
            has_encrypted_items: false,
        }
    }

    /// Returns the raw u64 id. Used by repository implementations to look up the
    /// associated actor. The id is a monotonic counter, not a pointer or secret.
    pub fn raw_id(&self) -> u64 {
        self.id.0
    }

    pub fn path(&self) -> Option<&std::path::Path> {
        self.path.as_deref()
    }

    pub fn format(&self) -> Option<ArchiveFormat> {
        self.format
    }

    pub fn with_path(mut self, path: PathBuf) -> Self {
        self.path = Some(path);
        self
    }

    pub fn with_format(mut self, format: ArchiveFormat) -> Self {
        self.format = Some(format);
        self
    }

    pub(crate) fn with_format_opt(mut self, format: Option<ArchiveFormat>) -> Self {
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

    pub fn empty() -> Self { Self("".into()) }
}

impl fmt::Debug for Password {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Password([redacted])")
    }
}

/// Configuration for creating encrypted archives.
#[must_use = "EncryptionConfig contains a password; unused config is likely a bug"]
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
    
    pub fn index(self) -> i32 {
        match self {
            OverwriteMode::Ask => 0,
            OverwriteMode::Overwrite => 1,
            OverwriteMode::Skip => 2,
            OverwriteMode::RenameExtracted => 3,
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

/// A single change to be applied to an archive.
#[derive(Debug, Clone)]
pub enum ArchiveChange {
    Add {
        fs_path: PathBuf,
        archive_path: String,
    },
    Update {
        fs_path: PathBuf,
        archive_path: String,
    },
    Delete {
        index: u32,
    },
    Rename {
        index: u32,
        new_path: String,
    },
}

/// A batch of changes to be applied to an archive atomically.
#[must_use = "ChangeSet must be applied via plan_changes/apply_changes"]
#[derive(Debug, Clone, Default)]
pub struct ChangeSet {
    changes: Vec<ArchiveChange>,
}

impl ChangeSet {
    pub fn new() -> Self {
        Self { changes: Vec::new() }
    }

    pub fn add(&mut self, fs_path: PathBuf, archive_path: String) {
        self.changes.push(ArchiveChange::Add { fs_path, archive_path });
    }

    pub fn update(&mut self, fs_path: PathBuf, archive_path: String) {
        self.changes.push(ArchiveChange::Update { fs_path, archive_path });
    }

    pub fn delete(&mut self, index: u32) {
        self.changes.push(ArchiveChange::Delete { index });
    }

    pub fn rename(&mut self, index: u32, new_path: String) {
        self.changes.push(ArchiveChange::Rename { index, new_path });
    }

    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }

    pub fn len(&self) -> usize {
        self.changes.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &ArchiveChange> {
        self.changes.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compression_ratio_zero_size() {
        let entry = ArchiveEntry::default();
        assert_eq!(entry.compression_ratio(), 0.0);
    }

    #[test]
    fn test_compression_ratio_no_compression() {
        let entry = ArchiveEntry::builder()
            .name("test.bin".into())
            .path("test.bin".into())
            .size(1000)
            .compressed_size(1000)
            .build();
        assert_eq!(entry.compression_ratio(), 0.0);
    }

    #[test]
    fn test_compression_ratio_positive() {
        let entry = ArchiveEntry::builder()
            .name("test.txt".into())
            .path("test.txt".into())
            .size(1000)
            .compressed_size(300)
            .build();
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
