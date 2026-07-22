//! Session manager for runtime lifecycle control.

use std::sync::Arc;

use bit7z_domain::archive::{ArchiveSession, SessionId};
use bit7z_domain::vfs::{OverlayVfs, SessionState};
use bit7z_ports::session::{SessionRef, SessionStore};

/// Manages archive session lifecycle without owning domain logic.
pub trait SessionManager: Send + Sync {
    /// Create a new session and store its initial state.
    fn create(
        &self,
        session: ArchiveSession,
        vfs: OverlayVfs,
    ) -> Result<SessionId, SessionManagerError>;

    /// Get a reference to an active session state.
    fn get(&self, id: SessionId) -> Option<SessionRef>;

    /// Close a session and release associated resources.
    fn close(&self, id: SessionId) -> Result<(), SessionManagerError>;

    /// Mark a session as saved after a successful commit.
    fn on_saved(&self, id: SessionId) -> Result<(), SessionManagerError>;

    /// Return the ids of all active sessions.
    fn all(&self) -> Vec<SessionId>;
}

/// Errors from session management.
#[derive(Debug, thiserror::Error)]
pub enum SessionManagerError {
    #[error("session not found: {0}")]
    NotFound(SessionId),
    #[error("session already exists: {0}")]
    AlreadyExists(SessionId),
}

/// Default session manager backed by a `SessionStore` port.
pub struct DefaultSessionManager {
    store: Arc<dyn SessionStore>,
}

impl DefaultSessionManager {
    pub fn new(store: Arc<dyn SessionStore>) -> Self {
        Self { store }
    }
}

impl SessionManager for DefaultSessionManager {
    fn create(
        &self,
        session: ArchiveSession,
        vfs: OverlayVfs,
    ) -> Result<SessionId, SessionManagerError> {
        let id = session.id;
        if self.store.contains(id) {
            return Err(SessionManagerError::AlreadyExists(id));
        }
        let state = SessionState {
            session,
            vfs,
            dirty_tree: Default::default(),
            edit_queue: Default::default(),
            metadata_cache: Default::default(),
        };
        self.store.insert(state);
        Ok(id)
    }

    fn get(&self, id: SessionId) -> Option<SessionRef> {
        self.store.get(id)
    }

    fn close(&self, id: SessionId) -> Result<(), SessionManagerError> {
        self.store
            .remove(id)
            .map(|_| ())
            .ok_or(SessionManagerError::NotFound(id))
    }

    fn on_saved(&self, id: SessionId) -> Result<(), SessionManagerError> {
        let state = self.store.get(id).ok_or(SessionManagerError::NotFound(id))?;
        // In a full implementation this would clear dirty state and rotate the base tree.
        let _ = state;
        Ok(())
    }

    fn all(&self) -> Vec<SessionId> {
        self.store.all()
    }
}
