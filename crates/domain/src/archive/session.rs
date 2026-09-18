use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use crate::archive::archive_format::ArchiveFormat;
use crate::password::Password;
use crate::vfs::{DirtyTree, EditQueue, OverlayVfs, VfsMetadata, VfsNodeId};

static NEXT_ARCHIVE_ID: AtomicU64 = AtomicU64::new(1);

pub fn next_archive_id() -> u64 {
    NEXT_ARCHIVE_ID.fetch_add(1, Ordering::Relaxed)
}

/// Stable identifier for an archive editing session.
pub type SessionId = u64;

/// Lightweight handle to an opened archive session.
///
/// The actual state (VFS, dirty tree, edit queue) is stored separately by the
/// runtime session manager. `ArchiveSession` is cheap to clone and pass around.
#[derive(Debug, Clone)]
pub struct ArchiveSession {
    pub id: SessionId,
    pub path: PathBuf,
    pub format: ArchiveFormat,
    pub password: Option<Password>,
}

impl ArchiveSession {
    pub fn new(path: PathBuf, format: ArchiveFormat) -> Self {
        Self {
            id: next_archive_id(),
            path,
            format,
            password: None,
        }
    }

    pub fn with_password(mut self, password: Password) -> Self {
        self.password = Some(password);
        self
    }
}

/// Complete in-memory state for an archive editing session.
#[derive(Debug, Clone)]
pub struct SessionState {
    pub session: ArchiveSession,
    pub vfs: OverlayVfs,
    pub dirty_tree: DirtyTree,
    pub edit_queue: EditQueue,
    pub metadata_cache: HashMap<VfsNodeId, VfsMetadata>,
}
