//! File-system and temporary-storage ports.

use std::path::Path;

use async_trait::async_trait;

#[derive(Debug, thiserror::Error)]
pub enum FsError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("other: {0}")]
    Other(String),
}

#[derive(Debug, thiserror::Error)]
pub enum TempError {
    #[error("failed to allocate temp directory: {0}")]
    AllocationFailed(String),
}

/// Basic asynchronous file-system operations.
#[async_trait]
pub trait FileSystem: Send + Sync {
    async fn read(&self, path: &Path) -> Result<Vec<u8>, FsError>;
    async fn write(&self, path: &Path, data: &[u8]) -> Result<(), FsError>;
    async fn remove(&self, path: &Path) -> Result<(), FsError>;
    async fn create_dir(&self, path: &Path) -> Result<(), FsError>;
    async fn exists(&self, path: &Path) -> Result<bool, FsError>;
}

/// Temporary storage allocation.
///
/// The returned path is owned by the adapter; callers receive only a reference
/// path to use while the allocation is alive.
#[async_trait]
pub trait TempStorage: Send + Sync {
    async fn allocate(&self, prefix: &str) -> Result<std::path::PathBuf, TempError>;
}
