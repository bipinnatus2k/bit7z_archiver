//! Progress reporting port.

use bit7z_domain::archive::progress::ProgressUpdate;

/// Receives progress updates from long-running backend operations.
pub trait ProgressReporter: Send + Sync {
    fn report(&self, update: ProgressUpdate);
}

/// No-op progress reporter for tests and fire-and-forget operations.
pub struct NoopProgressReporter;

impl ProgressReporter for NoopProgressReporter {
    fn report(&self, _update: ProgressUpdate) {}
}
