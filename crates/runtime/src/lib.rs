//! Runtime execution framework for the bit7z archiver.
//!
//! The runtime is a general-purpose job scheduling and execution layer. It is
//! not tied to archive concepts; archive specifics live in the Domain and Ports
//! layers.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use bit7z_capability::{ExecutionDescriptor, ResourceClaim};
use bit7z_domain::archive::SessionId;
use bit7z_ports::session::SessionRef;

pub mod cancel;
pub mod executor;
pub mod job;
pub mod resource;
pub mod scheduler;
pub mod session;

pub use cancel::CancellationToken;
pub use executor::{ExecutionContext, Executor};
pub use job::{Job, JobGraph, JobHandle, JobId, JobKind, JobResult, JobState, OperationHandle, OperationKind, OperationRequest, OperationState};
pub use resource::{ResourceCapacity, ResourceError, ResourceManager, ResourceToken};
pub use scheduler::Scheduler;
pub use session::{SessionManager, SessionManagerError};

/// Top-level runtime facade.
pub struct Runtime {
    pub job_manager: Arc<dyn JobManager>,
    pub scheduler: Arc<dyn Scheduler>,
    pub executor: Arc<dyn Executor>,
    pub session_manager: Arc<dyn SessionManager>,
    pub resource_manager: Arc<dyn ResourceManager>,
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
}

/// Shared runtime context passed to jobs.
#[derive(Debug, Clone)]
pub struct RuntimeContext {
    pub executor: Arc<smol::Executor<'static>>,
}

/// Event emitted by the runtime about operation lifecycle.
#[derive(Debug, Clone)]
pub enum OperationEvent {
    Submitted { handle: OperationHandle },
    Started { handle: OperationHandle },
    Progress { handle: OperationHandle, percent: u32 },
    Completed { handle: OperationHandle, result: JobResult },
    Cancelled { handle: OperationHandle },
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
