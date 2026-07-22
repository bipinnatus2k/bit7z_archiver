//! Runtime-backed archive application services.
//!
//! These services replace the legacy `ArchiveRepository`-based use cases by
//! constructing `CapabilityRequest`s, resolving them through a
//! `CapabilityResolver`, and submitting `OperationRequest`s to the runtime.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use bit7z_capability::{
    Availability, BackendId, Capability, CapabilityId, CapabilityKind, CapabilityRegistry,
    CapabilityRequest, CapabilityResolver, DefaultCapabilityResolver,
};
use bit7z_domain::archive::{
    ArchiveEntry, ArchiveFormat, ArchiveHandle, ArchiveSession, ChangeSet, EncryptionConfig, Page,
    Password, TestResult,
};
use bit7z_domain::repository::{ArchiveError, ArchiveProperties, ExtractOptions, WriteOptions};
use bit7z_infra_persistence::adapters::{
    Bit7zReaderAdapter, Bit7zWriterAdapter, InMemorySessionStore,
};
use bit7z_ports::fs::{FileSystem, FsError, TempError, TempStorage};
use bit7z_runtime::{
    JobResult, OperationHandle, OperationKind, OperationRequest, PortSet, Runtime, RuntimeBuilder,
    RuntimeContext,
};

use bit7z_runtime::job::Priority;

const BIT7Z_BACKEND_ID: u64 = 1;

/// Builds a runtime wired to the bit7z backend and an in-memory session store.
///
/// This is the production wiring function. Callers that need a different set of
/// ports (e.g. for tests) should use [`RuntimeBuilder`] directly.
pub fn build_bit7z_runtime(
    lib: bit7z_infra_bit7z::Library,
) -> (Arc<Runtime>, Arc<dyn CapabilityResolver>) {
    let registry = Arc::new(CapabilityRegistry::new());
    register_bit7z_capabilities(&registry);
    let resolver: Arc<dyn CapabilityResolver> =
        Arc::new(DefaultCapabilityResolver::new(registry.clone()));

    let lib = Arc::new(lib);
    let reader: Arc<dyn bit7z_ports::ArchiveReader> =
        Arc::new(Bit7zReaderAdapter::new_shared(lib.clone()));
    let writer: Arc<dyn bit7z_ports::ArchiveWriter> = Arc::new(Bit7zWriterAdapter::new_shared(lib));
    let session_store: Arc<dyn bit7z_ports::session::SessionStore> =
        Arc::new(InMemorySessionStore::new());

    let ports = PortSet {
        reader: reader.clone(),
        writer: writer.clone(),
        session_store: session_store.clone(),
        fs: Arc::new(StubFileSystem),
        temp_storage: Arc::new(StubTempStorage),
    };

    let ex = Arc::new(smol::Executor::new());
    let context = RuntimeContext {
        executor: ex.clone(),
    };

    let runtime = Arc::new(RuntimeBuilder::new().build(ports, context));

    // Keep the executor alive for the lifetime of the process. In a future
    // revision this will be tied to application lifecycle and shutdown.
    std::thread::spawn(move || smol::block_on(ex.run(futures::future::pending::<()>())));

    (runtime, resolver)
}

fn register_bit7z_capabilities(registry: &CapabilityRegistry) {
    let formats = vec![
        ArchiveFormat::SevenZip,
        ArchiveFormat::Zip,
        ArchiveFormat::Tar,
        ArchiveFormat::TarGz,
        ArchiveFormat::TarXz,
        ArchiveFormat::TarBz2,
    ];

    for (id, kind) in [
        (1, CapabilityKind::Read),
        (2, CapabilityKind::Write),
        (3, CapabilityKind::Extract),
        (4, CapabilityKind::Test),
        (5, CapabilityKind::Preview),
        (6, CapabilityKind::Hash),
        (7, CapabilityKind::Compress),
        (8, CapabilityKind::Encrypt),
    ] {
        registry.register(Capability {
            id: CapabilityId(id),
            backend: BackendId(BIT7Z_BACKEND_ID),
            kind,
            formats: formats.clone(),
            availability: Availability::Always,
            metadata: Default::default(),
            supports_encryption: kind == CapabilityKind::Encrypt || kind == CapabilityKind::Write,
            supports_solid: kind == CapabilityKind::Write || kind == CapabilityKind::Compress,
            supports_streaming: kind == CapabilityKind::Extract,
            supports_incremental: kind == CapabilityKind::Write,
        });
    }
}

struct StubFileSystem;

#[async_trait::async_trait]
impl FileSystem for StubFileSystem {
    async fn read(&self, _path: &Path) -> Result<Vec<u8>, FsError> {
        Err(FsError::Other("stub filesystem".into()))
    }
    async fn write(&self, _path: &Path, _data: &[u8]) -> Result<(), FsError> {
        Err(FsError::Other("stub filesystem".into()))
    }
    async fn remove(&self, _path: &Path) -> Result<(), FsError> {
        Err(FsError::Other("stub filesystem".into()))
    }
    async fn create_dir(&self, _path: &Path) -> Result<(), FsError> {
        Err(FsError::Other("stub filesystem".into()))
    }
    async fn exists(&self, _path: &Path) -> Result<bool, FsError> {
        Ok(false)
    }
}

struct StubTempStorage;

#[async_trait::async_trait]
impl TempStorage for StubTempStorage {
    async fn allocate(&self, prefix: &str) -> Result<PathBuf, TempError> {
        Err(TempError::AllocationFailed(prefix.to_string()))
    }
}

/// Runtime-backed archive service.
///
/// This is the primary facade the presentation layer uses to open, create,
/// extract, test, modify, and inspect archives. It submits operations to the
/// runtime and queries the session manager for synchronous navigation.
pub struct ArchiveService {
    runtime: Arc<Runtime>,
    resolver: Arc<dyn CapabilityResolver>,
    sessions: std::sync::Mutex<std::collections::HashMap<u64, ArchiveSession>>,
}

impl ArchiveService {
    pub fn new(runtime: Arc<Runtime>, resolver: Arc<dyn CapabilityResolver>) -> Self {
        Self {
            runtime,
            resolver,
            sessions: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }

    fn resolve_format(&self, path: &Path) -> ArchiveFormat {
        // First version defaults to Zip for unknown extensions; bit7z format
        // detection will be wired in a follow-up.
        match path.extension().and_then(|e| e.to_str()) {
            Some("7z") => ArchiveFormat::SevenZip,
            Some("zip") => ArchiveFormat::Zip,
            Some("tar") => ArchiveFormat::Tar,
            Some("tar.gz") => ArchiveFormat::TarGz,
            Some("tar.bz2") => ArchiveFormat::TarBz2,
            Some("tar.xz") => ArchiveFormat::TarXz,
            _ => ArchiveFormat::Zip,
        }
    }

    fn wait(&self, handle: OperationHandle) -> Result<JobResult, ArchiveError> {
        self.runtime
            .wait(handle)
            .ok_or_else(|| ArchiveError::Internal("runtime disappeared".into()))
    }

    fn lookup_session(&self, archive: &ArchiveHandle) -> Result<ArchiveSession, ArchiveError> {
        self.sessions
            .lock()
            .unwrap()
            .get(&archive.id)
            .cloned()
            .ok_or_else(|| ArchiveError::NotFound(format!("archive handle {}", archive.id)))
    }
}

impl ArchiveService {
    pub fn open(
        &self,
        path: &Path,
        _password: Option<&Password>,
    ) -> Result<ArchiveHandle, ArchiveError> {
        let format = self.resolve_format(path);
        let descriptor = self
            .resolver
            .resolve(CapabilityRequest {
                kind: CapabilityKind::Read,
                format,
                constraints: vec![],
                session_id: None,
            })
            .map_err(|e| ArchiveError::Internal(e.to_string()))?;

        let handle = self.runtime.submit(OperationRequest {
            kind: OperationKind::OpenArchive {
                path: path.to_path_buf(),
            },
            descriptor,
            priority: Priority::User,
        });

        match self.wait(handle)? {
            JobResult::Ok => {
                let session = find_session_by_path(&self.runtime, path)
                    .ok_or_else(|| ArchiveError::Internal("session not stored".into()))?;
                let archive_handle = ArchiveHandle::new_reader().with_path(path.to_path_buf());
                self.sessions
                    .lock()
                    .unwrap()
                    .insert(archive_handle.id, session);
                Ok(archive_handle)
            }
            JobResult::Tested(_) => Err(ArchiveError::Internal("unexpected test result".into())),
            JobResult::Failed(msg) => Err(ArchiveError::Internal(msg)),
            JobResult::Cancelled => Err(ArchiveError::Canceled),
        }
    }

    pub fn create(
        &self,
        path: &Path,
        format: ArchiveFormat,
        _encryption: Option<&EncryptionConfig>,
    ) -> Result<ArchiveHandle, ArchiveError> {
        let descriptor = self
            .resolver
            .resolve(CapabilityRequest {
                kind: CapabilityKind::Write,
                format,
                constraints: vec![],
                session_id: None,
            })
            .map_err(|e| ArchiveError::Internal(e.to_string()))?;

        let handle = self.runtime.submit(OperationRequest {
            kind: OperationKind::CreateArchive {
                path: path.to_path_buf(),
                format,
            },
            descriptor,
            priority: Priority::User,
        });

        match self.wait(handle)? {
            JobResult::Ok => {
                let session = find_session_by_path(&self.runtime, path)
                    .ok_or_else(|| ArchiveError::Internal("session not stored".into()))?;
                let archive_handle = ArchiveHandle::new_writer()
                    .with_path(path.to_path_buf())
                    .with_format(format);
                self.sessions
                    .lock()
                    .unwrap()
                    .insert(archive_handle.id, session);
                Ok(archive_handle)
            }
            JobResult::Tested(_) => Err(ArchiveError::Internal("unexpected test result".into())),
            JobResult::Failed(msg) => Err(ArchiveError::Internal(msg)),
            JobResult::Cancelled => Err(ArchiveError::Canceled),
        }
    }

    pub fn list_page(
        &self,
        archive: &ArchiveHandle,
        offset: usize,
        limit: usize,
    ) -> Result<Page<ArchiveEntry>, ArchiveError> {
        let session = self.lookup_session(archive)?;
        let state = self
            .runtime
            .session_manager
            .get(session.id)
            .ok_or_else(|| ArchiveError::NotFound(format!("session {}", session.id)))?;
        Ok(state.vfs.list_page(offset, limit))
    }

    pub fn get_properties(
        &self,
        archive: &ArchiveHandle,
    ) -> Result<ArchiveProperties, ArchiveError> {
        let session = self.lookup_session(archive)?;
        let _state = self
            .runtime
            .session_manager
            .get(session.id)
            .ok_or_else(|| ArchiveError::NotFound(format!("session {}", session.id)))?;
        // Properties are not yet computed by the runtime store; return defaults.
        Ok(ArchiveProperties::default())
    }

    pub fn extract(
        &self,
        archive: &ArchiveHandle,
        indices: &[u32],
        dest: &Path,
        _options: &ExtractOptions,
    ) -> Result<(), ArchiveError> {
        let session = self.lookup_session(archive)?;
        let descriptor = self
            .resolver
            .resolve(CapabilityRequest {
                kind: CapabilityKind::Extract,
                format: session.format,
                constraints: vec![],
                session_id: Some(session.id),
            })
            .map_err(|e| ArchiveError::Internal(e.to_string()))?;

        let handle = self.runtime.submit(OperationRequest {
            kind: OperationKind::Extract {
                session_id: session.id,
                indices: indices.to_vec(),
                destination: dest.to_path_buf(),
            },
            descriptor,
            priority: Priority::User,
        });

        match self.wait(handle)? {
            JobResult::Ok => Ok(()),
            JobResult::Tested(_) => Err(ArchiveError::Internal("unexpected test result".into())),
            JobResult::Failed(msg) => Err(ArchiveError::Internal(msg)),
            JobResult::Cancelled => Err(ArchiveError::Canceled),
        }
    }

    pub fn extract_to_buffer(
        &self,
        archive: &ArchiveHandle,
        index: u32,
    ) -> Result<Vec<u8>, ArchiveError> {
        let session = self.lookup_session(archive)?;
        self.runtime
            .ports
            .reader
            .extract_to_buffer(&session, index)
            .map_err(|e| ArchiveError::Internal(e.to_string()))
    }

    pub fn test(&self, archive: &ArchiveHandle) -> Result<TestResult, ArchiveError> {
        let session = self.lookup_session(archive)?;
        let descriptor = self
            .resolver
            .resolve(CapabilityRequest {
                kind: CapabilityKind::Test,
                format: session.format,
                constraints: vec![],
                session_id: Some(session.id),
            })
            .map_err(|e| ArchiveError::Internal(e.to_string()))?;

        let handle = self.runtime.submit(OperationRequest {
            kind: OperationKind::Test {
                session_id: session.id,
            },
            descriptor,
            priority: Priority::User,
        });

        match self.wait(handle)? {
            JobResult::Tested(result) => Ok(result),
            JobResult::Ok => Ok(TestResult {
                total: 0,
                passed: 0,
                failed: vec![],
            }),
            JobResult::Failed(msg) => Err(ArchiveError::Internal(msg)),
            JobResult::Cancelled => Err(ArchiveError::Canceled),
        }
    }

    pub fn close(&self, archive: &ArchiveHandle) {
        if let Ok(session) = self.lookup_session(archive) {
            let _ = self.runtime.session_manager.close(session.id);
        }
        self.sessions.lock().unwrap().remove(&archive.id);
    }

    pub fn plan_changes(
        &self,
        archive: &ArchiveHandle,
        change_set: &ChangeSet,
    ) -> Result<bit7z_domain::plan::ExecutionPlan, ArchiveError> {
        let session = self.lookup_session(archive)?;
        let state = self
            .runtime
            .session_manager
            .get(session.id)
            .ok_or_else(|| ArchiveError::NotFound(format!("session {}", session.id)))?;
        let snapshot = state.vfs.list_page(0, usize::MAX).items;
        Ok(bit7z_domain::plan::plan_changes(&snapshot, change_set))
    }

    pub fn apply_changes(
        &self,
        archive: &ArchiveHandle,
        _plan: &bit7z_domain::plan::ExecutionPlan,
        _options: &WriteOptions,
    ) -> Result<(), ArchiveError> {
        let session = self.lookup_session(archive)?;
        let descriptor = self
            .resolver
            .resolve(CapabilityRequest {
                kind: CapabilityKind::Write,
                format: session.format,
                constraints: vec![],
                session_id: Some(session.id),
            })
            .map_err(|e| ArchiveError::Internal(e.to_string()))?;

        let handle = self.runtime.submit(OperationRequest {
            kind: OperationKind::SaveArchive {
                session_id: session.id,
            },
            descriptor,
            priority: Priority::User,
        });

        match self.wait(handle)? {
            JobResult::Ok => Ok(()),
            JobResult::Tested(_) => Err(ArchiveError::Internal("unexpected test result".into())),
            JobResult::Failed(msg) => Err(ArchiveError::Internal(msg)),
            JobResult::Cancelled => Err(ArchiveError::Canceled),
        }
    }

    pub fn list_directory(
        &self,
        archive: &ArchiveHandle,
        path: &str,
    ) -> Result<Vec<ArchiveEntry>, ArchiveError> {
        let session = self.lookup_session(archive)?;
        let state = self
            .runtime
            .session_manager
            .get(session.id)
            .ok_or_else(|| ArchiveError::NotFound(format!("session {}", session.id)))?;
        Ok(state.vfs.list_directory(path))
    }
}

fn find_session_by_path(runtime: &Runtime, path: &Path) -> Option<ArchiveSession> {
    for id in runtime.session_manager.all() {
        if let Some(state) = runtime.session_manager.get(id) {
            if state.session.path == path {
                return Some(state.session.clone());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn library() -> Option<bit7z_infra_bit7z::Library> {
        let path = bit7z_infra_platform::find_7z_library()?;
        bit7z_infra_bit7z::Library::open(&path.to_string_lossy()).ok()
    }

    fn create_test_archive(path: &std::path::Path, files: &[(&str, &str)]) {
        let lib = library().expect("7-Zip library is required for tests");
        let writer = bit7z_infra_bit7z::Writer::create(&lib, bit7z_infra_bit7z::WriterFormat::Zip)
            .expect("create writer");

        let temp = tempfile::tempdir().unwrap();
        let mut file_paths = Vec::new();
        for (name, content) in files {
            let file_path = temp.path().join(name);
            if let Some(parent) = file_path.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            let mut file = std::fs::File::create(&file_path).unwrap();
            file.write_all(content.as_bytes()).unwrap();
            file_paths.push(file_path.to_string_lossy().to_string());
        }

        let refs: Vec<&str> = file_paths.iter().map(|s| s.as_str()).collect();
        writer.add_files(&refs).expect("add files");
        writer
            .compress_to(&path.to_string_lossy())
            .expect("compress archive");
    }

    #[test]
    fn test_archive_test_reports_counts() {
        let lib = match library() {
            Some(l) => l,
            None => return,
        };
        let temp = tempfile::tempdir().unwrap();
        let archive_path = temp.path().join("test.zip");
        create_test_archive(
            &archive_path,
            &[("a.txt", "hello"), ("b.txt", "world"), ("c.txt", "foo")],
        );

        let (runtime, resolver) = build_bit7z_runtime(lib);
        let service = ArchiveService::new(runtime, resolver);
        let handle = service.open(&archive_path, None).expect("open archive");

        let result = service.test(&handle).expect("test archive");
        assert_eq!(result.total, 3, "test should report 3 entries");
        assert_eq!(result.passed, 3, "all entries should pass");
        assert!(result.failed.is_empty(), "no failures expected");
    }

    #[test]
    fn test_batch_extract_multiple_files() {
        let lib = match library() {
            Some(l) => l,
            None => return,
        };
        let temp = tempfile::tempdir().unwrap();
        let archive_path = temp.path().join("test.zip");
        create_test_archive(
            &archive_path,
            &[("a.txt", "hello"), ("b.txt", "world"), ("c.txt", "foo")],
        );

        let (runtime, resolver) = build_bit7z_runtime(lib);
        let service = ArchiveService::new(runtime, resolver);
        let handle = service.open(&archive_path, None).expect("open archive");

        let dest = temp.path().join("extracted");
        std::fs::create_dir_all(&dest).unwrap();

        let all = service.list_page(&handle, 0, usize::MAX).unwrap().items;
        let indices: Vec<u32> = all
            .iter()
            .filter(|e| !e.is_directory)
            .map(|e| e.original_index)
            .collect();
        assert_eq!(indices.len(), 3);

        let options = ExtractOptions {
            overwrite_mode: bit7z_domain::archive::OverwriteMode::Overwrite,
            cancel: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            paused: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            notifier: Arc::new(bit7z_domain::repository::NoopNotifier),
        };

        service
            .extract(&handle, &indices, &dest, &options)
            .expect("batch extract");

        let extracted_a = dest.join("a.txt");
        let extracted_b = dest.join("b.txt");
        let extracted_c = dest.join("c.txt");
        assert!(extracted_a.exists(), "a.txt should be extracted");
        assert!(extracted_b.exists(), "b.txt should be extracted");
        assert!(extracted_c.exists(), "c.txt should be extracted");
        assert_eq!(std::fs::read_to_string(extracted_a).unwrap(), "hello");
        assert_eq!(std::fs::read_to_string(extracted_b).unwrap(), "world");
        assert_eq!(std::fs::read_to_string(extracted_c).unwrap(), "foo");
    }
}
