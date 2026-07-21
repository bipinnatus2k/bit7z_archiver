//! Job types and operation request model.

use bit7z_capability::ExecutionDescriptor;
use bit7z_domain::archive::{ArchiveFormat, ChangeSet, SessionId};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_JOB_ID: AtomicU64 = AtomicU64::new(1);

pub fn next_job_id() -> JobId {
    JobId(NEXT_JOB_ID.fetch_add(1, Ordering::Relaxed))
}

/// Identifier for a job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct JobId(pub u64);

/// Handle returned when submitting an operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OperationHandle(pub JobId);

/// Alias for a job handle.
pub type JobHandle = OperationHandle;

/// Kind of user-level operation.
#[derive(Debug, Clone)]
pub enum OperationKind {
    OpenArchive { path: PathBuf },
    CreateArchive { path: PathBuf, format: ArchiveFormat },
    SaveArchive { session_id: SessionId },
    Extract { session_id: SessionId, indices: Vec<u32>, destination: PathBuf },
    Test { session_id: SessionId },
    Preview { session_id: SessionId, index: u32 },
}

/// Priority of an operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Background,
    Normal,
    User,
}

/// Request submitted by the Application layer to the Runtime.
#[derive(Debug, Clone)]
pub struct OperationRequest {
    pub kind: OperationKind,
    pub descriptor: ExecutionDescriptor,
    pub priority: Priority,
}

/// A job ready for scheduling.
#[derive(Debug, Clone)]
pub struct Job {
    pub id: JobId,
    pub kind: JobKind,
    pub priority: Priority,
    pub session_id: Option<SessionId>,
    pub descriptor: ExecutionDescriptor,
}

/// Internal kind of a job.
#[derive(Debug, Clone)]
pub enum JobKind {
    OpenArchive { path: PathBuf },
    CreateArchive { path: PathBuf, format: ArchiveFormat },
    SaveArchive { session_id: SessionId, changeset: ChangeSet },
    Extract { session_id: SessionId, indices: Vec<u32>, destination: PathBuf },
    Test { session_id: SessionId },
    Preview { session_id: SessionId, index: u32 },
}

/// A graph of jobs with dependencies.
#[derive(Debug, Clone)]
pub struct JobGraph {
    pub nodes: Vec<Job>,
    pub edges: Vec<(JobId, JobId)>,
}

/// State of a job.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobState {
    Pending,
    Running,
    Completed,
    Cancelled,
    Failed,
}

/// State of an operation as exposed to callers.
#[derive(Debug, Clone)]
pub struct OperationState {
    pub handle: OperationHandle,
    pub state: JobState,
    pub progress: Option<u32>,
}

/// Result of executing a job.
#[derive(Debug, Clone)]
pub enum JobResult {
    Ok,
    Cancelled,
    Failed(String),
}
