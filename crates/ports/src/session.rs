//! Session storage port.

use std::sync::Arc;
use bit7z_domain::archive::session::SessionState;
use bit7z_domain::vfs::SessionState;

/// Opaque reference to a session state.
pub type SessionRef = Arc<SessionState>;

/// Storage backend for runtime session state.
pub trait SessionStore: Send + Sync {
    /// Insert or replace a session state.
    fn insert(&self, state: SessionState);

    /// Get a reference to the session state, if it exists.
    fn get(&self, id: bit7z_domain::archive::SessionId) -> Option<SessionRef>;

    /// Remove a session state and return it.
    fn remove(&self, id: bit7z_domain::archive::SessionId) -> Option<SessionState>;

    /// Returns true if the store contains the session.
    fn contains(&self, id: bit7z_domain::archive::SessionId) -> bool;

    /// Return the ids of all stored sessions.
    fn all(&self) -> Vec<bit7z_domain::archive::SessionId>;
}
