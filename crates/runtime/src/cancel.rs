//! Cancellation token for operations.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Token that can be checked and cancelled.
#[derive(Debug, Clone)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }

    pub fn as_atomic(&self) -> Arc<AtomicBool> {
        self.cancelled.clone()
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}
