//! Convenience builder for assembling a [`Runtime`] with a chosen set of ports.
//!
//! The runtime crate intentionally stays backend-agnostic; this builder only wires
//! together the scheduler, resource manager, executor, and session manager once
//! the caller supplies concrete port implementations.

use std::sync::Arc;

use crate::PortSet;
use crate::{
    DefaultJobManager, DefaultScheduler, DefaultSessionManager, Executor, LocalExecutor,
    ResourceManager, Runtime, RuntimeContext, Scheduler, SessionManager, SimpleResourceManager,
};

/// Builder for constructing a [`Runtime`] instance.
///
/// Defaults to a `LocalExecutor`, a `DefaultScheduler` with concurrency 4, and a
/// `SimpleResourceManager` with concurrency 4.
pub struct RuntimeBuilder {
    scheduler: Option<Arc<dyn Scheduler>>,
    executor: Option<Arc<dyn Executor>>,
    session_manager: Option<Arc<dyn SessionManager>>,
    resource_manager: Option<Arc<dyn ResourceManager>>,
    concurrency: usize,
}

impl Default for RuntimeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl RuntimeBuilder {
    pub fn new() -> Self {
        Self {
            scheduler: None,
            executor: None,
            session_manager: None,
            resource_manager: None,
            concurrency: 4,
        }
    }

    /// Set the maximum number of concurrent jobs.
    pub fn concurrency(mut self, value: usize) -> Self {
        self.concurrency = value;
        self
    }

    /// Provide a custom scheduler.
    pub fn scheduler(mut self, scheduler: Arc<dyn Scheduler>) -> Self {
        self.scheduler = Some(scheduler);
        self
    }

    /// Provide a custom executor.
    pub fn executor(mut self, executor: Arc<dyn Executor>) -> Self {
        self.executor = Some(executor);
        self
    }

    /// Provide a custom session manager.
    pub fn session_manager(mut self, session_manager: Arc<dyn SessionManager>) -> Self {
        self.session_manager = Some(session_manager);
        self
    }

    /// Provide a custom resource manager.
    pub fn resource_manager(mut self, resource_manager: Arc<dyn ResourceManager>) -> Self {
        self.resource_manager = Some(resource_manager);
        self
    }

    /// Build a [`Runtime`] using the supplied ports.
    ///
    /// Any component not explicitly overridden receives a default implementation.
    pub fn build(self, ports: PortSet, context: RuntimeContext) -> Runtime {
        let scheduler = self
            .scheduler
            .unwrap_or_else(|| Arc::new(DefaultScheduler::new(self.concurrency)));
        let resource_manager = self
            .resource_manager
            .unwrap_or_else(|| Arc::new(SimpleResourceManager::new(self.concurrency)));
        let session_manager = self
            .session_manager
            .unwrap_or_else(|| Arc::new(DefaultSessionManager::new(ports.session_store.clone())));
        let executor = self
            .executor
            .unwrap_or_else(|| Arc::new(LocalExecutor::new()) as Arc<dyn Executor>);

        let job_manager = Arc::new(DefaultJobManager::new(
            scheduler.clone(),
            executor.clone(),
            session_manager.clone(),
            resource_manager.clone(),
            ports.clone(),
            context.clone(),
        ));

        Runtime {
            job_manager,
            scheduler,
            executor,
            session_manager,
            resource_manager,
            ports,
            context,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use bit7z_domain::archive::{
        ArchiveEntry, ArchiveFormat, ArchiveSession, ChangeSet, EncryptionConfig, Password,
        SessionId, TestResult,
    };
    use bit7z_domain::repository::{ArchiveError, ExtractOptions};
    use bit7z_domain::vfs::{SessionState, VfsMetadata};
    use bit7z_ports::fs::{FileSystem, FsError, TempError, TempStorage};
    use bit7z_ports::session::{SessionRef, SessionStore};
    use bit7z_ports::{ArchiveReader, ArchiveWriter};

    use crate::job::Priority;
    use crate::{JobResult, OperationKind, OperationRequest, PortSet, RuntimeContext};

    struct MockReader {
        sessions: Mutex<HashMap<PathBuf, ArchiveSession>>,
    }

    impl MockReader {
        fn new() -> Self {
            Self {
                sessions: Mutex::new(HashMap::new()),
            }
        }
    }

    impl ArchiveReader for MockReader {
        fn open(
            &self,
            path: &Path,
            _password: Option<&Password>,
        ) -> Result<ArchiveSession, ArchiveError> {
            let session = ArchiveSession::new(path.to_path_buf(), ArchiveFormat::Zip);
            self.sessions
                .lock()
                .unwrap()
                .insert(path.to_path_buf(), session.clone());
            Ok(session)
        }

        fn read_entries(
            &self,
            _session: &ArchiveSession,
        ) -> Result<Vec<ArchiveEntry>, ArchiveError> {
            Ok(vec![ArchiveEntry {
                name: "readme.txt".into(),
                path: "readme.txt".into(),
                size: 12,
                original_index: 0,
                ..ArchiveEntry::default()
            }])
        }

        fn read_metadata(
            &self,
            _session: &ArchiveSession,
            _index: u32,
        ) -> Result<VfsMetadata, ArchiveError> {
            Ok(VfsMetadata::default())
        }

        fn extract(
            &self,
            _session: &ArchiveSession,
            _indices: &[u32],
            _dest: &Path,
            _options: &ExtractOptions,
        ) -> Result<(), ArchiveError> {
            Ok(())
        }

        fn extract_to_buffer(
            &self,
            _session: &ArchiveSession,
            _index: u32,
        ) -> Result<Vec<u8>, ArchiveError> {
            Ok(vec![])
        }

        fn test(&self, _session: &ArchiveSession) -> Result<TestResult, ArchiveError> {
            Ok(TestResult {
                total: 0,
                passed: 0,
                failed: vec![],
            })
        }
    }

    struct MockWriter;

    impl ArchiveWriter for MockWriter {
        fn create(
            &self,
            path: &Path,
            format: ArchiveFormat,
            _encryption: Option<&EncryptionConfig>,
        ) -> Result<ArchiveSession, ArchiveError> {
            Ok(ArchiveSession::new(path.to_path_buf(), format))
        }

        fn commit(
            &self,
            _session: &ArchiveSession,
            _snapshot: &[ArchiveEntry],
            _changeset: &ChangeSet,
        ) -> Result<(), ArchiveError> {
            Ok(())
        }
    }

    #[derive(Default)]
    struct MockSessionStore {
        states: Mutex<HashMap<SessionId, SessionState>>,
    }

    impl SessionStore for MockSessionStore {
        fn insert(&self, state: SessionState) {
            self.states.lock().unwrap().insert(state.session.id, state);
        }

        fn get(&self, id: SessionId) -> Option<SessionRef> {
            self.states.lock().unwrap().get(&id).cloned().map(Arc::new)
        }

        fn remove(&self, id: SessionId) -> Option<SessionState> {
            self.states.lock().unwrap().remove(&id)
        }

        fn contains(&self, id: SessionId) -> bool {
            self.states.lock().unwrap().contains_key(&id)
        }

        fn all(&self) -> Vec<SessionId> {
            self.states.lock().unwrap().keys().copied().collect()
        }
    }

    struct MockFs;

    #[async_trait]
    impl FileSystem for MockFs {
        async fn read(&self, _path: &Path) -> Result<Vec<u8>, FsError> {
            Ok(vec![])
        }
        async fn write(&self, _path: &Path, _data: &[u8]) -> Result<(), FsError> {
            Ok(())
        }
        async fn remove(&self, _path: &Path) -> Result<(), FsError> {
            Ok(())
        }
        async fn create_dir(&self, _path: &Path) -> Result<(), FsError> {
            Ok(())
        }
        async fn exists(&self, _path: &Path) -> Result<bool, FsError> {
            Ok(false)
        }
    }

    struct MockTemp;

    #[async_trait]
    impl TempStorage for MockTemp {
        async fn allocate(&self, prefix: &str) -> Result<PathBuf, TempError> {
            Ok(PathBuf::from(prefix))
        }
    }

    #[test]
    fn test_open_archive_job_stores_session() {
        let ex = Arc::new(smol::Executor::new());
        let ex_bg = ex.clone();
        let _bg =
            std::thread::spawn(move || smol::block_on(ex_bg.run(futures::future::pending::<()>())));

        let context = RuntimeContext {
            executor: ex.clone(),
        };
        let mock_store = Arc::new(MockSessionStore::default());
        let ports = PortSet {
            reader: Arc::new(MockReader::new()),
            writer: Arc::new(MockWriter),
            session_store: mock_store.clone(),
            fs: Arc::new(MockFs),
            temp_storage: Arc::new(MockTemp),
        };
        let runtime = Arc::new(RuntimeBuilder::new().build(ports, context));

        let handle = runtime.submit(OperationRequest {
            kind: OperationKind::OpenArchive {
                path: PathBuf::from("test.zip"),
            },
            descriptor: bit7z_capability::ExecutionDescriptor {
                backend: bit7z_capability::BackendId(1),
                capabilities: vec![bit7z_capability::CapabilityId(1)],
                resource_claim: Default::default(),
                policy: bit7z_capability::ExecutionPolicy::Queued,
            },
            priority: Priority::User,
        });

        let result = runtime.wait(handle).expect("job should complete");
        assert!(matches!(result, JobResult::Ok));
        assert_eq!(mock_store.states.lock().unwrap().len(), 1);
    }

    #[test]
    fn test_create_archive_job_stores_session() {
        let ex = Arc::new(smol::Executor::new());
        let ex_bg = ex.clone();
        let _bg =
            std::thread::spawn(move || smol::block_on(ex_bg.run(futures::future::pending::<()>())));

        let context = RuntimeContext {
            executor: ex.clone(),
        };
        let mock_store = Arc::new(MockSessionStore::default());
        let ports = PortSet {
            reader: Arc::new(MockReader::new()),
            writer: Arc::new(MockWriter),
            session_store: mock_store.clone(),
            fs: Arc::new(MockFs),
            temp_storage: Arc::new(MockTemp),
        };
        let runtime = Arc::new(RuntimeBuilder::new().build(ports, context));

        let handle = runtime.submit(OperationRequest {
            kind: OperationKind::CreateArchive {
                path: PathBuf::from("new.zip"),
                format: ArchiveFormat::Zip,
            },
            descriptor: bit7z_capability::ExecutionDescriptor {
                backend: bit7z_capability::BackendId(1),
                capabilities: vec![bit7z_capability::CapabilityId(2)],
                resource_claim: Default::default(),
                policy: bit7z_capability::ExecutionPolicy::Queued,
            },
            priority: Priority::User,
        });

        let result = runtime.wait(handle).expect("job should complete");
        assert!(matches!(result, JobResult::Ok));
        assert_eq!(mock_store.states.lock().unwrap().len(), 1);
    }
}
