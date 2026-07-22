//! Local executor that maps jobs to adapter calls.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use bit7z_domain::vfs::OverlayVfs;
use crate::executor::{ExecutionContext, Executor};
use crate::job::{Job, JobKind, JobResult};

/// Local executor that runs jobs synchronously on the current async task.
///
/// This executor does not spawn sub-tasks; it calls adapters directly. It is
/// suitable for the first version of the runtime before more sophisticated
/// task scheduling is needed.
pub struct LocalExecutor;

impl LocalExecutor {
    pub fn new() -> Self {
        Self
    }

    fn run_job(
        job: Job,
        ctx: ExecutionContext,
    ) -> Pin<Box<dyn Future<Output = JobResult> + Send>> {
        Box::pin(async move {
            if ctx.cancellation.is_cancelled() {
                return JobResult::Cancelled;
            }

            match job.kind {
                JobKind::OpenArchive { path } => open_archive(ctx, path).await,
                JobKind::CreateArchive { path, format } => create_archive(ctx, path, format).await,
                JobKind::SaveArchive { session_id } => save_archive(ctx, session_id).await,
                JobKind::Extract {
                    session_id,
                    indices,
                    destination,
                } => extract(ctx, session_id, indices, destination).await,
                JobKind::Test { session_id } => test(ctx, session_id).await,
                JobKind::Preview { session_id, index } => preview(ctx, session_id, index).await,
            }
        })
    }
}

impl Default for LocalExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl Executor for LocalExecutor {
    fn execute(&self, job: Job, ctx: ExecutionContext) -> Pin<Box<dyn Future<Output = JobResult> + Send>> {
        Self::run_job(job, ctx)
    }
}

async fn open_archive(ctx: ExecutionContext, path: std::path::PathBuf) -> JobResult {
    let session = match ctx.ports.reader.open(&path, None) {
        Ok(s) => s,
        Err(e) => return JobResult::Failed(e.to_string()),
    };

    let entries = match ctx.ports.reader.read_entries(&session) {
        Ok(e) => e,
        Err(e) => return JobResult::Failed(e.to_string()),
    };

    let vfs = OverlayVfs::build(session.path.clone(), Some(session.format), &entries);
    let state = bit7z_domain::vfs::SessionState {
        session: session.clone(),
        vfs,
        dirty_tree: Default::default(),
        edit_queue: Default::default(),
        metadata_cache: Default::default(),
    };
    ctx.ports.session_store.insert(state);

    JobResult::Ok
}

async fn create_archive(
    ctx: ExecutionContext,
    path: std::path::PathBuf,
    format: bit7z_domain::archive::ArchiveFormat,
) -> JobResult {
    match ctx.ports.writer.create(&path, format, None) {
        Ok(session) => {
            let vfs = OverlayVfs::build(path.clone(), Some(format), &[]);
            let state = bit7z_domain::vfs::SessionState {
                session: session.clone(),
                vfs,
                dirty_tree: Default::default(),
                edit_queue: Default::default(),
                metadata_cache: Default::default(),
            };
            ctx.ports.session_store.insert(state);
            JobResult::Ok
        }
        Err(e) => JobResult::Failed(e.to_string()),
    }
}

async fn save_archive(ctx: ExecutionContext, session_id: bit7z_domain::archive::SessionId) -> JobResult {
    let state = match ctx.ports.session_store.get(session_id) {
        Some(s) => s,
        None => {
            return JobResult::Failed(format!("session {} not found", session_id));
        }
    };

    let snapshot: Vec<_> = state.vfs.list_page(0, usize::MAX).items;
    let changeset = state.vfs.generate_changeset();

    match ctx.ports.writer.commit(&state.session, &snapshot, &changeset) {
        Ok(()) => JobResult::Ok,
        Err(e) => JobResult::Failed(e.to_string()),
    }
}

async fn extract(
    ctx: ExecutionContext,
    session_id: bit7z_domain::archive::SessionId,
    indices: Vec<u32>,
    destination: std::path::PathBuf,
) -> JobResult {
    let state = match ctx.ports.session_store.get(session_id) {
        Some(s) => s,
        None => {
            return JobResult::Failed(format!("session {} not found", session_id));
        }
    };

    let options = bit7z_domain::repository::ExtractOptions {
        overwrite_mode: bit7z_domain::archive::OverwriteMode::Overwrite,
        cancel: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        paused: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        notifier: Arc::new(NoopNotifier),
    };

    match ctx
        .ports
        .reader
        .extract(&state.session, &indices, &destination, &options)
    {
        Ok(()) => JobResult::Ok,
        Err(e) => JobResult::Failed(e.to_string()),
    }
}

async fn test(ctx: ExecutionContext, session_id: bit7z_domain::archive::SessionId) -> JobResult {
    let state = match ctx.ports.session_store.get(session_id) {
        Some(s) => s,
        None => {
            return JobResult::Failed(format!("session {} not found", session_id));
        }
    };

    match ctx.ports.reader.test(&state.session) {
        Ok(_result) => JobResult::Ok,
        Err(e) => JobResult::Failed(e.to_string()),
    }
}

async fn preview(
    ctx: ExecutionContext,
    session_id: bit7z_domain::archive::SessionId,
    index: u32,
) -> JobResult {
    let state = match ctx.ports.session_store.get(session_id) {
        Some(s) => s,
        None => {
            return JobResult::Failed(format!("session {} not found", session_id));
        }
    };

    match ctx.ports.reader.extract_to_buffer(&state.session, index) {
        Ok(_data) => JobResult::Ok,
        Err(e) => JobResult::Failed(e.to_string()),
    }
}

struct NoopNotifier;

impl bit7z_domain::repository::ProgressNotifier for NoopNotifier {
    fn notify(&self, _update: &bit7z_domain::repository::ProgressUpdate) {}
}
