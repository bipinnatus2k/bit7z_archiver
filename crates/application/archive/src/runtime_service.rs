//! Runtime-backed archive application services.
//!
//! These services replace the legacy `ArchiveRepository`-based use cases by
//! constructing archive capability requests, resolving them through an
//! archive-aware resolver, and submitting operations to the runtime.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use bit7z_capability::{
    CapabilityRegistry, CapabilityResolver, ExecutionDescriptor, LockKey, Request,
};
use bit7z_domain::archive::{
    ArchiveEntry, ArchiveFormat, ArchiveHandle, ArchiveSession, ChangeSet, EncryptionConfig, Page,
    Password, TestResult,
};
use bit7z_domain::repository::{ArchiveError, ArchiveProperties, ExtractOptions, WriteOptions};
use bit7z_domain::vfs::SessionState;
use bit7z_infra_persistence::adapters::{
    Bit7zReaderAdapter, Bit7zWriterAdapter, InMemorySessionStore,
};
use bit7z_ports::detection::FormatDetector;
use bit7z_ports::fs::{FileSystem, FsError, TempError, TempStorage};
use bit7z_runtime::{
    JobResult, OperationHandle, OperationKind, OperationRequest, PortSet, Runtime, RuntimeBuilder,
    RuntimeContext,
};

use bit7z_runtime::job::Priority;

use crate::auto_format::AutoFormat;
use crate::capability::{
    ArchiveCapMeta, ArchiveCapabilityResolver, ArchiveOpKind, bit7z_formats,
    register_archive_capabilities,
};

const BIT7Z_BACKEND_ID: u64 = 1;

/// Builds a runtime wired to the bit7z backend and an in-memory session store.
///
/// This is the production wiring function. Callers that need a different set of
/// ports (e.g. for tests) should use [`RuntimeBuilder`] directly.
pub fn build_bit7z_runtime(
    lib: bit7z_infra_bit7z::Library,
) -> (
    Arc<Runtime>,
    Arc<dyn CapabilityResolver<ArchiveCapMeta>>,
    Arc<dyn FormatDetector>,
) {
    let registry: Arc<CapabilityRegistry<ArchiveCapMeta>> =
        Arc::new(CapabilityRegistry::new());
    register_archive_capabilities(
        &registry,
        bit7z_capability::BackendId(BIT7Z_BACKEND_ID),
        &bit7z_formats(),
    );
    let resolver: Arc<dyn CapabilityResolver<ArchiveCapMeta>> =
        Arc::new(ArchiveCapabilityResolver::new(registry.clone()));

    let lib = Arc::new(lib);
    let reader: Arc<dyn bit7z_ports::ArchiveReader> =
        Arc::new(Bit7zReaderAdapter::new_shared(lib.clone()));
    let writer: Arc<dyn bit7z_ports::ArchiveWriter> =
        Arc::new(Bit7zWriterAdapter::new_shared(lib));
    let session_store: Arc<dyn bit7z_ports::session::SessionStore> =
        Arc::new(InMemorySessionStore::new());
    let detector: Arc<dyn FormatDetector> = Arc::new(AutoFormat::new());

    let ports = PortSet {
        reader: reader.clone(),
        writer: writer.clone(),
        session_store: session_store.clone(),
        fs: Arc::new(StubFileSystem),
        temp_storage: Arc::new(StubTempStorage),
        detector: Some(detector.clone()),
    };

    let ex = Arc::new(smol::Executor::new());
    let context = RuntimeContext {
        executor: ex.clone(),
    };

    let runtime = Arc::new(RuntimeBuilder::new().build(ports, context));

    // Keep the executor alive for the lifetime of the process.
    std::thread::spawn(move || smol::block_on(ex.run(futures::future::pending::<()>())));

    (runtime, resolver, detector)
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
    resolver: Arc<dyn CapabilityResolver<ArchiveCapMeta>>,
    detector: Option<Arc<dyn FormatDetector>>,
    sessions: std::sync::Mutex<std::collections::HashMap<u64, ArchiveSession>>,
}

impl ArchiveService {
    pub fn new(
        runtime: Arc<Runtime>,
        resolver: Arc<dyn CapabilityResolver<ArchiveCapMeta>>,
        detector: Option<Arc<dyn FormatDetector>>,
    ) -> Self {
        Self {
            runtime,
            resolver,
            detector,
            sessions: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }

    fn resolve_format(&self, path: &Path) -> ArchiveFormat {
        let name = path.to_string_lossy().to_lowercase();
        if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
            return ArchiveFormat::TarGz;
        }
        if name.ends_with(".tar.xz") || name.ends_with(".txz") {
            return ArchiveFormat::TarXz;
        }
        if name.ends_with(".tar.bz2")
            || name.ends_with(".tbz2")
            || name.ends_with(".tbz")
        {
            return ArchiveFormat::TarBz2;
        }
        match path.extension().and_then(|e| e.to_str()) {
            Some("7z") => ArchiveFormat::SevenZip,
            Some("zip") => ArchiveFormat::Zip,
            Some("tar") => ArchiveFormat::Tar,
            Some("gz") => ArchiveFormat::GZip,
            Some("bz2") => ArchiveFormat::BZip2,
            Some("xz") => ArchiveFormat::Xz,
            Some("wim") => ArchiveFormat::Wim,
            Some("rar") => ArchiveFormat::Rar,
            _ => ArchiveFormat::Zip,
        }
    }

    fn resolve(
        &self,
        kind: ArchiveOpKind,
        format: ArchiveFormat,
        session_id: Option<u64>,
    ) -> Result<ExecutionDescriptor, ArchiveError> {
        self.resolver
            .resolve(&Request {
                meta: ArchiveCapMeta {
                    kind,
                    formats: vec![format],
                    supports_encryption: false,
                    supports_solid: false,
                    supports_streaming: false,
                    supports_incremental: false,
                },
                tags: vec![],
                lock_key: session_id.map(LockKey),
            })
            .map_err(|e| ArchiveError::Internal(e.to_string()))
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
        password: Option<&Password>,
    ) -> Result<ArchiveHandle, ArchiveError> {
        // 1. Try magic-bytes-based format detection.
        let format = if let Some(detector) = &self.detector {
            // Resolve detection capability as a policy check.
            let _ = self
                .resolve(ArchiveOpKind::DetectFormat, self.resolve_format(path), None)
                .map_err(|e| ArchiveError::Internal(e.to_string()))?;
            match detector.detect_format(path) {
                Ok(f) => f,
                Err(_) => self.resolve_format(path), // fallback to extension
            }
        } else {
            self.resolve_format(path)
        };

        // 2. Resolve read capability with detected format.
        let descriptor = self.resolve(ArchiveOpKind::Read, format, None)?;

        let handle = self.runtime.submit(OperationRequest {
            kind: OperationKind::OpenArchive {
                path: path.to_path_buf(),
                password: password.cloned(),
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
        let descriptor = self.resolve(ArchiveOpKind::Write, format, None)?;

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

    pub fn get_session_state(
        &self,
        archive: &ArchiveHandle,
    ) -> Result<SessionState, ArchiveError> {
        let session = self.lookup_session(archive)?;
        self.runtime
            .session_manager
            .get(session.id)
            .ok_or_else(|| ArchiveError::NotFound(format!("session {}", session.id)))
            .map(|ref_state| (*ref_state).clone())
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
        let descriptor = self.resolve(ArchiveOpKind::Extract, session.format, Some(session.id))?;

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
        let descriptor = self.resolve(ArchiveOpKind::Test, session.format, Some(session.id))?;

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
        let descriptor = self.resolve(ArchiveOpKind::Write, session.format, Some(session.id))?;

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

        let (runtime, resolver, detector) = build_bit7z_runtime(lib);
        let service = ArchiveService::new(runtime, resolver, Some(detector));
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

        let (runtime, resolver, detector) = build_bit7z_runtime(lib);
        let service = ArchiveService::new(runtime, resolver, Some(detector));
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