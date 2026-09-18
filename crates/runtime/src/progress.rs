//! Progress reporter that forwards backend updates to the runtime event stream.

use bit7z_domain::archive::progress::ProgressUpdate;
use bit7z_ports::progress::ProgressReporter;

use crate::{DefaultJobManager, OperationHandle};

/// Progress reporter that emits [`crate::OperationEvent::Progress`] events.
///
/// It is created per-job and holds the job handle and a clone of the job
/// manager so that backend updates are published to runtime subscribers.
#[derive(Clone)]
pub struct RuntimeProgressReporter {
    handle: OperationHandle,
    manager: DefaultJobManager,
}

impl RuntimeProgressReporter {
    pub fn new(handle: OperationHandle, manager: DefaultJobManager) -> Self {
        Self { handle, manager }
    }
}

impl ProgressReporter for RuntimeProgressReporter {
    fn report(&self, update: ProgressUpdate) {
        let percent = if update.items_total > 0 {
            ((update.items_done * 100) / update.items_total) as u32
        } else if update.bytes_total > 0 {
            ((update.bytes_done * 100) / update.bytes_total) as u32
        } else {
            0
        }
        .min(100);
        self.manager.emit_progress(self.handle, percent);
    }
}
