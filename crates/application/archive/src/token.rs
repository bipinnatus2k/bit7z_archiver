use std::sync::Arc;

use bit7z_runtime::{
    JobResult, OperationEventReceiver, OperationHandle, OperationState, Runtime, RuntimeError,
};

/// Token returned by CommandExecutor for an async operation.
///
/// The holder can cancel the operation, poll its state, subscribe to
/// progress events, or block until it completes.
pub struct OperationToken {
    pub(crate) handle: OperationHandle,
    pub(crate) runtime: Arc<Runtime>,
}

impl OperationToken {
    pub fn cancel(&self) -> Result<(), RuntimeError> {
        self.runtime.cancel(self.handle)
    }

    pub fn state(&self) -> Option<OperationState> {
        self.runtime.state(self.handle)
    }

    pub fn subscribe(&self) -> OperationEventReceiver {
        let (tx, rx) = futures::channel::mpsc::unbounded();
        self.runtime.subscribe(tx);
        rx
    }

    pub fn wait(self) -> Option<JobResult> {
        self.runtime.wait(self.handle)
    }
}
