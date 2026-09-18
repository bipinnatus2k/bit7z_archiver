

#[derive(Debug, thiserror::Error)]
pub enum ArchiveError {
    #[error("File not found: {0}")]
    NotFound(String),
    #[error("Unsupported archive format")]
    UnsupportedFormat,
    #[error("Archive is corrupt: {0}")]
    Corrupt(String),
    #[error("Wrong password")]
    WrongPassword,
    #[error("Archive is encrypted - password required")]
    EncryptedArchiveRequiresPassword,
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Internal error: {0}")]
    Internal(String),
    #[error("Operation canceled")]
    Canceled,
    #[error("Operation not supported for this archive format")]
    UnsupportedOperation,
    #[error("Archive is not writable")]
    ReadOnlyArchive,
    #[error("Conflicts detected during operation")]
    Conflict,
}