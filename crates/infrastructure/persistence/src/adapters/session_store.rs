use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use bit7z_domain::archive::SessionId;
use bit7z_domain::vfs::SessionState;
use bit7z_ports::session::{SessionRef, SessionStore};

/// In-memory session store.
///
/// This is the default implementation used while a session is actively being
/// edited. Persistence or long-lived sessions can be layered on top later.
#[derive(Debug, Default)]
pub struct InMemorySessionStore {
    states: RwLock<HashMap<SessionId, SessionState>>,
}

impl InMemorySessionStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl SessionStore for InMemorySessionStore {
    fn insert(&self, state: SessionState) {
        self.states.write().unwrap().insert(state.session.id, state);
    }

    fn get(&self, id: SessionId) -> Option<SessionRef> {
        self.states.read().unwrap().get(&id).cloned().map(Arc::new)
    }

    fn remove(&self, id: SessionId) -> Option<SessionState> {
        self.states.write().unwrap().remove(&id)
    }

    fn contains(&self, id: SessionId) -> bool {
        self.states.read().unwrap().contains_key(&id)
    }

    fn all(&self) -> Vec<SessionId> {
        self.states.read().unwrap().keys().copied().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bit7z_domain::archive::{ArchiveFormat, ArchiveSession};
    use bit7z_domain::vfs::OverlayVfs;
    use std::path::PathBuf;

    fn sample_state(id: SessionId) -> SessionState {
        let session = ArchiveSession {
            id,
            path: PathBuf::from("test.7z"),
            format: ArchiveFormat::SevenZip,
            password: None,
        };
        let vfs = OverlayVfs::build(PathBuf::from("test.7z"), None, &[]);
        SessionState {
            session,
            vfs,
            dirty_tree: Default::default(),
            edit_queue: Default::default(),
            metadata_cache: Default::default(),
        }
    }

    #[test]
    fn test_insert_and_get() {
        let store = InMemorySessionStore::new();
        let state = sample_state(1);
        store.insert(state.clone());

        let got = store.get(1).unwrap();
        assert_eq!(got.session.id, 1);
    }

    #[test]
    fn test_remove() {
        let store = InMemorySessionStore::new();
        let state = sample_state(2);
        store.insert(state);
        assert!(store.contains(2));

        store.remove(2);
        assert!(!store.contains(2));
        assert!(store.get(2).is_none());
    }
}
