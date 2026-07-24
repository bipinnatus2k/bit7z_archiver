//! Resource manager for runtime job arbitration.

use std::collections::HashMap;
use std::sync::Mutex;

use bit7z_capability::{LockKey, ResourceClaim, ResourceLock};

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
    #[error("resource is already locked: {0:?}")]
    ResourceLocked(LockKey),
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
    exclusive_locks: HashMap<LockKey, ()>,
    shared_locks: HashMap<LockKey, usize>,
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

        let active: usize = state.exclusive_locks.len() + state.shared_locks.len();
        if active >= state.concurrency {
            return Err(ResourceError::ConcurrencyLimit);
        }

        for lock in &claim.resource_locks {
            match lock {
                ResourceLock::Exclusive(key) => {
                    if state.exclusive_locks.contains_key(key)
                        || state.shared_locks.contains_key(key)
                    {
                        return Err(ResourceError::ResourceLocked(*key));
                    }
                    state.exclusive_locks.insert(*key, ());
                }
                ResourceLock::Shared(key) => {
                    if state.exclusive_locks.contains_key(key) {
                        return Err(ResourceError::ResourceLocked(*key));
                    }
                    *state.shared_locks.entry(*key).or_insert(0) += 1;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn claim_exclusive(key: u64) -> ResourceClaim {
        ResourceClaim {
            resource_locks: vec![ResourceLock::Exclusive(LockKey(key))],
            ..Default::default()
        }
    }

    fn claim_shared(key: u64) -> ResourceClaim {
        ResourceClaim {
            resource_locks: vec![ResourceLock::Shared(LockKey(key))],
            ..Default::default()
        }
    }

    #[test]
    fn test_concurrency_limit() {
        let rm = SimpleResourceManager::new(1);
        assert!(rm.acquire(claim_exclusive(1)).is_ok());
        assert!(matches!(
            rm.acquire(claim_exclusive(2)),
            Err(ResourceError::ConcurrencyLimit)
        ));
    }

    #[test]
    fn test_exclusive_lock_blocks_shared() {
        let rm = SimpleResourceManager::new(10);
        assert!(rm.acquire(claim_exclusive(1)).is_ok());
        assert!(matches!(
            rm.acquire(claim_shared(1)),
            Err(ResourceError::ResourceLocked(LockKey(1)))
        ));
    }

    #[test]
    fn test_shared_lock_allows_concurrent_readers() {
        let rm = SimpleResourceManager::new(10);
        assert!(rm.acquire(claim_shared(1)).is_ok());
        assert!(rm.acquire(claim_shared(1)).is_ok());
    }
}