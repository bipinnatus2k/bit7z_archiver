//! Executor trait and context.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use bit7z_ports::fs::{FileSystem, TempStorage};
use bit7z_ports::progress::ProgressReporter;
use bit7z_ports::session::SessionStore;
use bit7z_ports::{ArchiveReader, ArchiveWriter};

use crate::RuntimeContext;
use crate::cancel::CancellationToken;
use crate::job::{Job, JobResult};

/// Collection of concrete port implementations available to the runtime.
#[derive(Clone)]
pub struct PortSet {
    pub reader: Arc<dyn ArchiveReader>,
    pub writer: Arc<dyn ArchiveWriter>,
    pub session_store: Arc<dyn SessionStore>,
    pub fs: Arc<dyn FileSystem>,
    pub temp_storage: Arc<dyn TempStorage>,
}

/// Executes a job by breaking it into tasks and running them.
pub trait Executor: Send + Sync {
    fn execute(
        &self,
        job: Job,
        ctx: ExecutionContext,
    ) -> Pin<Box<dyn Future<Output = JobResult> + Send>>;
}

/// Context passed to the executor for a single job.
#[derive(Clone)]
pub struct ExecutionContext {
    pub runtime: RuntimeContext,
    pub ports: PortSet,
    pub cancellation: CancellationToken,
    pub progress: Arc<dyn ProgressReporter>,
}

impl ExecutionContext {
    pub fn new(
        runtime: RuntimeContext,
        ports: PortSet,
        progress: Arc<dyn ProgressReporter>,
    ) -> Self {
        Self {
            runtime,
            ports,
            cancellation: CancellationToken::new(),
            progress,
        }
    }
}
