//! Runtime execution framework for the bit7z archiver.
//!
//! The runtime is a general-purpose job scheduling and execution layer. It is
//! not tied to archive concepts; archive specifics live in the Domain and Ports
//! layers.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

pub mod builder;
pub mod cancel;
pub mod executor;
pub mod executor_impl;
pub mod job;
pub mod manager;
pub mod progress;
pub mod progress_bridge;
pub mod resource;
pub mod scheduler;
pub mod session;

pub use builder::RuntimeBuilder;
pub use progress_bridge::ProgressBridge;
pub use cancel::CancellationToken;
pub use executor::{ExecutionContext, Executor, PortSet};
pub use executor_impl::LocalExecutor;
pub use job::{
    Job, JobGraph, JobHandle, JobId, JobKind, JobResult, JobState, OperationHandle, OperationKind,
    OperationRequest, OperationState,
};
pub use manager::DefaultJobManager;
pub use resource::{
    ResourceCapacity, ResourceError, ResourceManager, ResourceToken, SimpleResourceManager,
};
pub use scheduler::{DefaultScheduler, Scheduler};
pub use session::{DefaultSessionManager, SessionManager, SessionManagerError};

/// Top-level runtime facade.
pub struct Runtime {
    pub job_manager: Arc<dyn JobManager>,
    pub scheduler: Arc<dyn Scheduler>,
    pub executor: Arc<dyn Executor>,
    pub session_manager: Arc<dyn SessionManager>,
    pub resource_manager: Arc<dyn ResourceManager>,
    pub ports: PortSet,
    pub context: RuntimeContext,
}

impl Runtime {
    /// Submit an operation request and receive a handle.
    pub fn submit(&self, request: OperationRequest) -> OperationHandle {
        self.job_manager.submit(request)
    }

    /// Cancel an in-flight operation.
    pub fn cancel(&self, handle: OperationHandle) -> Result<(), RuntimeError> {
        self.job_manager.cancel(handle)
    }

    /// Query the current state of an operation.
    pub fn state(&self, handle: OperationHandle) -> Option<OperationState> {
        self.job_manager.state(handle)
    }

    /// Subscribe to runtime-wide operation events.
    pub fn subscribe(&self, sender: OperationEventSender) {
        self.job_manager.subscribe(sender);
    }

    /// Block the current thread until the operation completes.
    ///
    /// This is intended for tests and synchronous callers; production code should
    /// subscribe to events or await the handle through the async API.
    pub fn wait(&self, handle: OperationHandle) -> Option<JobResult> {
        smol::block_on(self.wait_async(handle))
    }

    /// Returns a future that resolves when the operation completes.
    ///
    /// The future subscribes to runtime events and yields the job result for the
    /// requested handle.
    pub fn wait_async(
        &self,
        handle: OperationHandle,
    ) -> impl std::future::Future<Output = Option<JobResult>> + '_ {
        use futures::StreamExt;

        let mut rx_events = {
            let (tx_bridge, rx_bridge) = futures::channel::mpsc::unbounded::<OperationEvent>();
            self.subscribe(tx_bridge);
            rx_bridge
        };

        async move {
            // First, check if the job is already completed.
            if let Some(state) = self.state(handle) {
                if matches!(
                    state.state,
                    JobState::Completed | JobState::Failed | JobState::Cancelled
                ) {
                    return Some(state.result.clone().unwrap_or_else(|| match state.state {
                        JobState::Completed => JobResult::Ok,
                        JobState::Failed => JobResult::Failed("failed".into()),
                        JobState::Cancelled => JobResult::Cancelled,
                        _ => unreachable!(),
                    }));
                }
            }

            while let Some(event) = rx_events.next().await {
                if let OperationEvent::Completed {
                    handle: event_handle,
                    result,
                } = event
                {
                    if event_handle == handle {
                        return Some(result);
                    }
                }
            }

            None
        }
    }
}

/// Shared runtime context passed to jobs.
#[derive(Debug, Clone)]
pub struct RuntimeContext {
    pub executor: Arc<smol::Executor<'static>>,
}

/// Event emitted by the runtime about operation lifecycle.
#[derive(Debug, Clone)]
pub enum OperationEvent {
    Submitted {
        handle: OperationHandle,
    },
    Started {
        handle: OperationHandle,
    },
    Progress {
        handle: OperationHandle,
        percent: u32,
    },
    Completed {
        handle: OperationHandle,
        result: JobResult,
    },
    Cancelled {
        handle: OperationHandle,
    },
}

pub type OperationEventSender = futures::channel::mpsc::UnboundedSender<OperationEvent>;
pub type OperationEventReceiver = futures::channel::mpsc::UnboundedReceiver<OperationEvent>;

/// Manager for submitted operations.
pub trait JobManager: Send + Sync {
    fn submit(&self, operation: OperationRequest) -> OperationHandle;
    fn cancel(&self, handle: OperationHandle) -> Result<(), RuntimeError>;
    fn state(&self, handle: OperationHandle) -> Option<OperationState>;
    fn subscribe(&self, sender: OperationEventSender);
}

/// Errors originating from the runtime framework.
#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("operation not found: {0:?}")]
    OperationNotFound(OperationHandle),
    #[error("session error: {0}")]
    Session(#[from] SessionManagerError),
    #[error("resource error: {0}")]
    Resource(#[from] ResourceError),
    #[error("execution error: {0}")]
    Execution(String),
}

/// Type alias for executor futures.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
