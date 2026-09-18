//! Abstract ports for the bit7z archiver backend.
//!
//! This crate contains only trait definitions. The concrete types referenced by
//! these traits (`ArchiveSession`, `ArchiveEntry`, `ChangeSet`, etc.) live in
//! the `bit7z-domain` crate.

use std::path::Path;

use bit7z_domain::archive::{
    ArchiveEntry, ArchiveFormat, ArchiveSession, ChangeSet, EncryptionConfig, Password, TestResult,
};
use bit7z_domain::archive::progress::{ArchiveError, ArchiveProperties, ExtractOptions};
use bit7z_domain::vfs::VfsMetadata;

pub mod detection;
pub mod fs;
pub mod progress;
pub mod session;

/// Reader-side archive operations.
pub trait ArchiveReader: Send + Sync {
    /// Open an existing archive and return a lightweight session handle.
    fn open(
        &self,
        path: &Path,
        password: Option<&Password>,
    ) -> Result<bit7z_domain::archive::ArchiveSession, ArchiveError>;

    /// Check whether the archive at `path` is encrypted, without opening it.
    ///
    /// A return value of `true` means the archive uses encryption and a password
    /// will be required to open it. `false` means either the archive is not
    /// encrypted or the backend cannot determine this statically.
    ///
    /// The default implementation returns `Ok(false)`.
    fn check_encrypted(&self, _path: &Path) -> Result<bool, ArchiveError> {
        Ok(false)
    }

    /// Read all entries from the archive.
    fn read_entries(
        &self,
        session: &bit7z_domain::archive::ArchiveSession,
    ) -> Result<Vec<ArchiveEntry>, ArchiveError>;

    /// Read metadata for a single entry by its original archive index.
    fn read_metadata(
        &self,
        session: &bit7z_domain::archive::ArchiveSession,
        index: u32,
    ) -> Result<VfsMetadata, ArchiveError>;

    /// Extract selected entries to a destination directory.
    fn extract(
        &self,
        session: &bit7z_domain::archive::ArchiveSession,
        indices: &[u32],
        dest: &Path,
        options: &ExtractOptions,
    ) -> Result<(), ArchiveError>;

    /// Extract a single entry into memory.
    fn extract_to_buffer(
        &self,
        session: &bit7z_domain::archive::ArchiveSession,
        index: u32,
    ) -> Result<Vec<u8>, ArchiveError>;

    /// Test archive integrity.
    fn test(
        &self,
        session: &bit7z_domain::archive::ArchiveSession,
    ) -> Result<TestResult, ArchiveError>;
}

/// Writer-side archive operations.
pub trait ArchiveWriter: Send + Sync {
    /// Create a new empty archive and return a session handle.
    fn create(
        &self,
        path: &Path,
        format: ArchiveFormat,
        encryption: Option<&EncryptionConfig>,
    ) -> Result<bit7z_domain::archive::ArchiveSession, ArchiveError>;

    /// Commit a changeset against the current snapshot of the archive.
    fn commit(
        &self,
        session: &ArchiveSession,
        snapshot: &[ArchiveEntry],
        changeset: &ChangeSet,
    ) -> Result<(), ArchiveError>;
}

/// Cryptographic operations used by checksum/test workflows.
pub trait CryptoProvider: Send + Sync {
    /// Compute a hash for the provided data.
    fn hash(&self, data: &[u8], algorithm: HashAlgorithm) -> Result<String, CryptoError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashAlgorithm {
    Crc32,
    Md5,
    Sha1,
    Sha256,
}

#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("unsupported algorithm: {0:?}")]
    UnsupportedAlgorithm(HashAlgorithm),
    #[error("crypto operation failed: {0}")]
    OperationFailed(String),
}

/// Optional helper trait for adapters that can report archive-level properties.
pub trait ArchivePropertiesProvider: Send + Sync {
    fn properties(
        &self,
        session: &ArchiveSession,
    ) -> Result<ArchiveProperties, ArchiveError>;
}
