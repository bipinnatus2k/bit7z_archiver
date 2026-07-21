//! Executor trait and context.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use bit7z_ports::progress::ProgressReporter;

use crate::cancel::CancellationToken;
use crate::job::{Job, JobResult};
use crate::RuntimeContext;

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
    pub cancellation: CancellationToken,
    pub progress: Arc<dyn ProgressReporter>,
}

impl ExecutionContext {
    pub fn new(runtime: RuntimeContext, progress: Arc<dyn ProgressReporter>) -> Self {
        Self {
            runtime,
            cancellation: CancellationToken::new(),
            progress,
        }
    }
}
