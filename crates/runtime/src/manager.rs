//! Default JobManager implementation.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::JobManager;
use crate::OperationEvent;
use crate::RuntimeContext;
use crate::cancel::CancellationToken;
use crate::executor::{ExecutionContext, Executor, PortSet};
use crate::job::{
    Job, JobId, JobKind, JobState, OperationHandle, OperationRequest, OperationState,
};
use crate::resource::{ResourceManager, ResourceToken};
use crate::scheduler::Scheduler;
use crate::session::SessionManager;

/// Default implementation of the JobManager.
///
/// Maintains pending and running jobs, asks the scheduler for the next job to
/// run, and dispatches execution to the executor. Publishes operation events
/// to subscribers.
pub struct DefaultJobManager {
    inner: Arc<Mutex<ManagerInner>>,
    scheduler: Arc<dyn Scheduler>,
    executor: Arc<dyn Executor>,
    session_manager: Arc<dyn SessionManager>,
    resource_manager: Arc<dyn ResourceManager>,
    ports: PortSet,
    context: RuntimeContext,
}

struct ManagerInner {
    next_id: u64,
    pending: Vec<Job>,
    running: HashMap<JobId, RunningJob>,
    completed: HashMap<JobId, OperationState>,
    subscribers: Vec<crate::OperationEventSender>,
}

struct RunningJob {
    job: Job,
    cancellation: CancellationToken,
    resource_token: ResourceToken,
}

impl DefaultJobManager {
    pub fn new(
        scheduler: Arc<dyn Scheduler>,
        executor: Arc<dyn Executor>,
        session_manager: Arc<dyn SessionManager>,
        resource_manager: Arc<dyn ResourceManager>,
        ports: PortSet,
        context: RuntimeContext,
    ) -> Self {
        Self {
            inner: Arc::new(Mutex::new(ManagerInner {
                next_id: 1,
                pending: Vec::new(),
                running: HashMap::new(),
                completed: HashMap::new(),
                subscribers: Vec::new(),
            })),
            scheduler,
            executor,
            session_manager,
            resource_manager,
            ports,
            context,
        }
    }

    fn emit(&self, inner: &mut ManagerInner, event: OperationEvent) {
        inner
            .subscribers
            .retain(|sender| sender.unbounded_send(event.clone()).is_ok());
    }

    /// Emit a progress event for a running operation.
    pub fn emit_progress(&self, handle: OperationHandle, percent: u32) {
        let mut inner = self.inner.lock().unwrap();
        self.emit(&mut *inner, OperationEvent::Progress { handle, percent });
    }

    fn operation_request_to_job(&self, id: JobId, request: OperationRequest) -> Job {
        let kind = match request.kind {
            crate::job::OperationKind::OpenArchive { path, password } => JobKind::OpenArchive { path, password },
            crate::job::OperationKind::CreateArchive { path, format } => {
                JobKind::CreateArchive { path, format }
            }
            crate::job::OperationKind::SaveArchive { session_id } => {
                JobKind::SaveArchive { session_id }
            }
            crate::job::OperationKind::Extract {
                session_id,
                indices,
                destination,
            } => JobKind::Extract {
                session_id,
                indices,
                destination,
            },
            crate::job::OperationKind::Test { session_id } => JobKind::Test { session_id },
            crate::job::OperationKind::Preview { session_id, index } => {
                JobKind::Preview { session_id, index }
            }
        };

        let session_id = job_session_id(&kind);

        Job {
            id,
            kind,
            priority: request.priority,
            session_id,
            descriptor: request.descriptor,
        }
    }

    fn dispatch(&self, inner: &mut ManagerInner) {
        if inner.pending.is_empty() {
            return;
        }

        let capacity = self.resource_manager.query();
        let running_jobs: Vec<Job> = inner.running.values().map(|r| r.job.clone()).collect();
        let selected = self
            .scheduler
            .select_next(&inner.pending, &running_jobs, capacity);
        if selected.is_empty() {
            return;
        }

        let selected_set: std::collections::HashSet<_> = selected.iter().copied().collect();
        let selected_jobs: Vec<Job> = inner
            .pending
            .drain(..)
            .filter(|j| selected_set.contains(&j.id))
            .collect();

        for job in selected_jobs {
            let token = match self
                .resource_manager
                .acquire(job.descriptor.resource_claim.clone())
            {
                Ok(t) => t,
                Err(_) => {
                    inner.pending.push(job);
                    continue;
                }
            };

            let cancellation = CancellationToken::new();
            let handle = OperationHandle(job.id);
            let running = RunningJob {
                job: job.clone(),
                cancellation: cancellation.clone(),
                resource_token: token,
            };
            inner.running.insert(job.id, running);

            self.emit(&mut *inner, OperationEvent::Started { handle });

            let executor = self.executor.clone();
            let runtime_context = self.context.clone();
            let ports = self.ports.clone();
            let manager = self.clone();
            let progress = Arc::new(crate::progress::RuntimeProgressReporter::new(
                handle,
                manager.clone(),
            ));
            let ctx = ExecutionContext::new(runtime_context, ports, progress);

            let task = executor.execute(job, ctx);
            let fut = async move {
                let result = task.await;
                manager.on_job_complete(handle, result, cancellation);
            };
            self.context.executor.spawn(fut).detach();
        }
    }

    fn on_job_complete(
        &self,
        handle: OperationHandle,
        result: crate::job::JobResult,
        cancellation: CancellationToken,
    ) {
        let mut inner = self.inner.lock().unwrap();
        if let Some(running) = inner.running.remove(&handle.0) {
            self.resource_manager.release(running.resource_token);
        }

        let state = OperationState {
            handle,
            state: match &result {
                crate::job::JobResult::Ok | crate::job::JobResult::Tested(_) => JobState::Completed,
                crate::job::JobResult::Cancelled => {
                    if cancellation.is_cancelled() {
                        JobState::Cancelled
                    } else {
                        JobState::Cancelled
                    }
                }
                crate::job::JobResult::Failed(_) => JobState::Failed,
            },
            progress: None,
            result: Some(result.clone()),
        };
        inner.completed.insert(handle.0, state);

        self.emit(&mut *inner, OperationEvent::Completed { handle, result });

        // Try to dispatch more jobs now that resources are freed.
        drop(inner);
        let mut inner = self.inner.lock().unwrap();
        self.dispatch(&mut *inner);
    }
}

impl Clone for DefaultJobManager {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            scheduler: self.scheduler.clone(),
            executor: self.executor.clone(),
            session_manager: self.session_manager.clone(),
            resource_manager: self.resource_manager.clone(),
            ports: self.ports.clone(),
            context: self.context.clone(),
        }
    }
}

impl JobManager for DefaultJobManager {
    fn submit(&self, operation: OperationRequest) -> OperationHandle {
        let mut inner = self.inner.lock().unwrap();
        let id = JobId(inner.next_id);
        inner.next_id += 1;

        let job = self.operation_request_to_job(id, operation);
        let handle = OperationHandle(id);

        self.emit(&mut *inner, OperationEvent::Submitted { handle });
        inner.pending.push(job);
        inner.pending.sort_by(|a, b| a.priority.cmp(&b.priority));

        self.dispatch(&mut *inner);
        handle
    }

    fn cancel(&self, handle: OperationHandle) -> Result<(), crate::RuntimeError> {
        let inner = self.inner.lock().unwrap();
        if let Some(running) = inner.running.get(&handle.0) {
            running.cancellation.cancel();
            Ok(())
        } else {
            Err(crate::RuntimeError::OperationNotFound(handle))
        }
    }

    fn state(&self, handle: OperationHandle) -> Option<OperationState> {
        let inner = self.inner.lock().unwrap();
        if let Some(state) = inner.completed.get(&handle.0) {
            return Some(state.clone());
        }
        inner.running.get(&handle.0).map(|_| OperationState {
            handle,
            state: JobState::Running,
            progress: None,
            result: None,
        })
    }

    fn subscribe(&self, sender: crate::OperationEventSender) {
        let mut inner = self.inner.lock().unwrap();
        inner.subscribers.push(sender);
    }
}

fn job_session_id(kind: &JobKind) -> Option<bit7z_domain::archive::SessionId> {
    match kind {
        JobKind::OpenArchive { .. } => None,
        JobKind::CreateArchive { .. } => None,
        JobKind::SaveArchive { session_id } => Some(*session_id),
        JobKind::Extract { session_id, .. } => Some(*session_id),
        JobKind::Test { session_id } => Some(*session_id),
        JobKind::Preview { session_id, .. } => Some(*session_id),
    }
}
