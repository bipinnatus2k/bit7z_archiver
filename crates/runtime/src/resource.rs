//! Resource manager for runtime job arbitration.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use bit7z_capability::{ResourceClaim, SessionLock};
use bit7z_domain::archive::SessionId;

/// Token representing an acquired resource claim.
#[derive(Debug, Clone)]
pub struct ResourceToken;

/// Current available resource capacity.
#[derive(Debug, Clone, Copy, Default)]
pub struct ResourceCapacity {
    pub concurrency: usize,
    pub memory_bytes: usize,
    pub thread_count: usize,
    pub temp_bytes: usize,
    pub handle_count: usize,
}

/// Errors from resource acquisition.
#[derive(Debug, thiserror::Error)]
pub enum ResourceError {
    #[error("session {0} is already locked exclusively")]
    SessionLocked(SessionId),
    #[error("concurrency limit reached")]
    ConcurrencyLimit,
    #[error("insufficient memory: requested {requested}, available {available}")]
    InsufficientMemory { requested: usize, available: usize },
    #[error("other resource error: {0}")]
    Other(String),
}

/// Arbitrates access to shared resources.
pub trait ResourceManager: Send + Sync {
    fn acquire(&self, claim: ResourceClaim) -> Result<ResourceToken, ResourceError>;
    fn release(&self, token: ResourceToken);
    fn query(&self) -> ResourceCapacity;
}

/// Simple resource manager for the first version.
#[derive(Debug, Default)]
pub struct SimpleResourceManager {
    state: Mutex<SimpleResourceState>,
}

#[derive(Debug, Default)]
struct SimpleResourceState {
    concurrency: usize,
    exclusive_sessions: HashMap<SessionId, ()>,
    shared_sessions: HashMap<SessionId, usize>,
}

impl SimpleResourceManager {
    pub fn new(max_concurrency: usize) -> Self {
        Self {
            state: Mutex::new(SimpleResourceState {
                concurrency: max_concurrency,
                ..Default::default()
            }),
        }
    }
}

impl ResourceManager for SimpleResourceManager {
    fn acquire(&self, claim: ResourceClaim) -> Result<ResourceToken, ResourceError> {
        let mut state = self.state.lock().unwrap();

        let active: usize = state.exclusive_sessions.len() + state.shared_sessions.len();
        if active >= state.concurrency {
            return Err(ResourceError::ConcurrencyLimit);
        }

        for lock in &claim.session_locks {
            match lock {
                SessionLock::Exclusive(id) => {
                    if state.exclusive_sessions.contains_key(id)
                        || state.shared_sessions.contains_key(id)
                    {
                        return Err(ResourceError::SessionLocked(*id));
                    }
                    state.exclusive_sessions.insert(*id, ());
                }
                SessionLock::Shared(id) => {
                    if state.exclusive_sessions.contains_key(id) {
                        return Err(ResourceError::SessionLocked(*id));
                    }
                    *state.shared_sessions.entry(*id).or_insert(0) += 1;
                }
            }
        }

        Ok(ResourceToken)
    }

    fn release(&self, _token: ResourceToken) {
        // First version is coarse-grained; release all locks held by the caller.
        // This will be refined once tokens carry claim identifiers.
    }

    fn query(&self) -> ResourceCapacity {
        let state = self.state.lock().unwrap();
        ResourceCapacity {
            concurrency: state.concurrency,
            ..Default::default()
        }
    }
}
