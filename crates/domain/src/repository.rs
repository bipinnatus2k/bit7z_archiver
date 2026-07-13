use crate::archive::*;
use crate::plan::ExecutionPlan;
use std::ops::Range;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug, Clone)]
pub struct ProgressUpdate {
    pub file_current: u64,
    pub file_total: u64,
    pub current_file: Option<String>,
    pub items_done: u64,
    pub items_total: u64,
    pub bytes_done: u64,
    pub bytes_total: u64,
    pub error: Option<String>,
}

impl Default for ProgressUpdate {
    fn default() -> Self {
        Self {
            file_current: 0,
            file_total: 0,
            current_file: None,
            items_done: 0,
            items_total: 0,
            bytes_done: 0,
            bytes_total: 0,
            error: None,
        }
    }
}

pub trait ProgressNotifier: Send + Sync {
    fn notify(&self, update: &ProgressUpdate);
}

pub struct NoopNotifier;

impl ProgressNotifier for NoopNotifier {
    fn notify(&self, _update: &ProgressUpdate) {}
}

pub struct ExtractOptions {
    pub overwrite_mode: OverwriteMode,
    pub cancel: Arc<AtomicBool>,
    pub paused: Arc<AtomicBool>,
    pub notifier: Arc<dyn ProgressNotifier>,
}

pub struct WriteOptions {
    pub cancel: Arc<AtomicBool>,
    pub paused: Arc<AtomicBool>,
    pub notifier: Arc<dyn ProgressNotifier>,
}

// ============================================================================
// New types for FFI actor model (Track A contract)
// ============================================================================

/// A cancellation token sharable across threads.
/// Set `cancel()` to signal ongoing operations to abort.
#[derive(Clone)]
pub struct CancellationToken(Arc<AtomicBool>);

impl CancellationToken {
    pub fn new() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }

    pub fn cancel(&self) {
        self.0.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

/// A pause token sharable across threads.
/// Set `pause()` to pause, `resume()` to continue.
#[derive(Clone)]
pub struct PauseToken(Arc<AtomicBool>);

impl PauseToken {
    pub fn new() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }

    pub fn pause(&self) {
        self.0.store(true, Ordering::Relaxed);
    }

    pub fn resume(&self) {
        self.0.store(false, Ordering::Relaxed);
    }

    pub fn is_paused(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }

    /// Blocks until resumed or cancelled. Checks every 50ms.
    pub fn wait_while_paused(&self, cancel: &CancellationToken) {
        while self.is_paused() {
            if cancel.is_cancelled() {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }
}

impl Default for PauseToken {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for receiving progress/file events from FFI callbacks.
/// The `OpCtx` struct holds an `Arc<dyn ProgressSink>` that is called
/// on the actor thread. Implementations must be thread-safe.
pub trait ProgressSink: Send + Sync {
    fn on_progress(&self, processed: u64, total: u64);
    fn on_file(&self, path: &str);
}

/// A no-op sink that discards all progress events.
pub struct NoopSink;

impl ProgressSink for NoopSink {
    fn on_progress(&self, _processed: u64, _total: u64) {}
    fn on_file(&self, _path: &str) {}
}

/// Per-call context for extract/test/apply operations.
/// Contains cancellation, pause, and progress reporting.
pub struct OpCtx {
    pub cancel: CancellationToken,
    pub pause: PauseToken,
    pub progress: Arc<dyn ProgressSink>,
}

/// Parameters for an extract operation.
pub struct ExtractRequest {
    pub indices: Vec<u32>,
    pub dest: PathBuf,
    pub overwrite: OverwriteMode,
}

/// Result of an extract operation.
#[derive(Debug, Clone, Default)]
pub struct ExtractReport {
    pub extracted: u32,
    pub skipped: u32,
    pub bytes: u64,
}

/// Result of a test operation.
#[derive(Debug, Clone)]
pub struct TestReport {
    pub all_ok: bool,
    pub total: u32,
    pub failed: Vec<TestFailure>,
}

// ============================================================================
// Updated ArchiveRepository trait (Track A contract)
// ============================================================================

pub trait ArchiveRepository: Send + Sync {
    fn open(&self, path: &Path, password: Option<&Password>) -> Result<ArchiveHandle, ArchiveError>;
    fn create(&self, path: &Path, format: ArchiveFormat, encryption: Option<&EncryptionConfig>) -> Result<ArchiveHandle, ArchiveError>;
    fn list(&self, h: &ArchiveHandle, range: Range<usize>) -> Result<Page<ArchiveEntry>, ArchiveError>;
    fn list_dir(&self, h: &ArchiveHandle, dir: &str, range: Range<usize>) -> Result<Page<ArchiveEntry>, ArchiveError>;
    fn properties(&self, h: &ArchiveHandle) -> Result<ArchiveProperties, ArchiveError>;
    fn extract(&self, h: &ArchiveHandle, req: &ExtractRequest, ctx: &OpCtx) -> Result<ExtractReport, ArchiveError>;
    fn extract_to_buffer(&self, h: &ArchiveHandle, index: u32) -> Result<Vec<u8>, ArchiveError>;
    fn test(&self, h: &ArchiveHandle, indices: &[u32], ctx: &OpCtx) -> Result<TestReport, ArchiveError>;
    fn plan(&self, h: &ArchiveHandle, changes: &ChangeSet) -> Result<ExecutionPlan, ArchiveError>;
    fn apply(&self, h: &ArchiveHandle, plan: &ExecutionPlan, opts: &WriteOptions, ctx: &OpCtx) -> Result<(), ArchiveError>;
    fn build_archive(&self, h: &ArchiveHandle, files: &[(PathBuf, String)], out_path: &Path, ctx: &OpCtx) -> Result<(), ArchiveError> {
        Err(ArchiveError::UnsupportedOperation)
    }
    fn close(&self, h: &ArchiveHandle);
}

#[derive(Debug, thiserror::Error)]
pub enum ArchiveError {
    #[error("File not found: {0}")]
    NotFound(String),
    #[error("Archive handle is not open")]
    NotOpen,
    #[error("Unsupported archive format")]
    UnsupportedFormat,
    #[error("Archive is corrupt: {0}")]
    Corrupt(String),
    #[error("Wrong password")]
    WrongPassword,
    #[error("Archive is encrypted - password required")]
    EncryptedArchiveRequiresPassword,
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Internal error: {0}")]
    Internal(String),
    #[error("Operation cancelled")]
    Cancelled,
    #[error("Operation not supported for this archive format")]
    UnsupportedOperation,
    #[error("Archive is not writable")]
    ReadOnlyArchive,
    #[error("Conflicts detected during operation")]
    Conflict,
}

/// Archive-level properties from bit7z.
#[derive(Debug, Clone)]
pub struct ArchiveProperties {
    pub(crate) items_count: u32,
    pub(crate) folders_count: u32,
    pub(crate) files_count: u32,
    pub(crate) total_size: u64,
    pub(crate) packed_size: u64,
    pub(crate) is_encrypted: bool,
    pub(crate) has_encrypted_items: bool,
    pub(crate) is_multi_volume: bool,
    pub(crate) is_solid: bool,
    pub(crate) encrypted_names: bool,
    pub(crate) has_comment: bool,
    pub(crate) comment_size: Option<usize>,
    pub(crate) has_recovery_record: bool,
    pub(crate) locked: bool,
    pub(crate) dictionary_size: Option<u64>,
    pub(crate) headers_size: u64,
    pub(crate) volumes_count: u32,
}

impl ArchiveProperties {
    pub fn new(
        items_count: u32,
        folders_count: u32,
        files_count: u32,
        total_size: u64,
        packed_size: u64,
        is_solid: bool,
        is_multi_volume: bool,
        volumes_count: u32,
        headers_size: u64,
        has_comment: bool,
        dictionary_size: Option<u64>,
    ) -> Self {
        Self {
            items_count,
            folders_count,
            files_count,
            total_size,
            packed_size,
            is_encrypted: false,
            has_encrypted_items: false,
            is_multi_volume,
            is_solid,
            encrypted_names: false,
            has_comment,
            comment_size: None,
            has_recovery_record: false,
            locked: false,
            dictionary_size,
            headers_size,
            volumes_count,
        }
    }

    pub fn items_count(&self) -> u32 { self.items_count }
    pub fn folders_count(&self) -> u32 { self.folders_count }
    pub fn files_count(&self) -> u32 { self.files_count }
    pub fn total_size(&self) -> u64 { self.total_size }
    pub fn packed_size(&self) -> u64 { self.packed_size }
    pub fn is_encrypted(&self) -> bool { self.is_encrypted }
    pub fn has_encrypted_items(&self) -> bool { self.has_encrypted_items }
    pub fn is_multi_volume(&self) -> bool { self.is_multi_volume }
    pub fn is_solid(&self) -> bool { self.is_solid }
    pub fn encrypted_names(&self) -> bool { self.encrypted_names }
    pub fn has_comment(&self) -> bool { self.has_comment }
    pub fn comment_size(&self) -> Option<usize> { self.comment_size }
    pub fn has_recovery_record(&self) -> bool { self.has_recovery_record }
    pub fn locked(&self) -> bool { self.locked }
    pub fn dictionary_size(&self) -> Option<u64> { self.dictionary_size }
    pub fn headers_size(&self) -> u64 { self.headers_size }
    pub fn volumes_count(&self) -> u32 { self.volumes_count }
}

impl Default for ArchiveProperties {
    fn default() -> Self {
        Self {
            items_count: 0,
            folders_count: 0,
            files_count: 0,
            total_size: 0,
            packed_size: 0,
            is_encrypted: false,
            has_encrypted_items: false,
            is_multi_volume: false,
            is_solid: false,
            encrypted_names: false,
            has_comment: false,
            comment_size: None,
            has_recovery_record: false,
            locked: false,
            dictionary_size: None,
            headers_size: 0,
            volumes_count: 0,
        }
    }
}

#[cfg(any(test, feature = "testing"))]
#[doc(hidden)]
pub mod test_utils {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::sync::atomic::{AtomicU64, Ordering};

    static MOCK_NEXT_ID: AtomicU64 = AtomicU64::new(1);

    pub struct MockArchiveRepository {
        pub entries: Mutex<Vec<ArchiveEntry>>,
        pub test_result: Mutex<TestResult>,
        pub mock_extract_buffer: bool,
        crate_handle: Mutex<Option<ArchiveHandle>>,
    }

    impl MockArchiveRepository {
        pub fn new(entries: Vec<ArchiveEntry>) -> Self {
            Self {
                entries: Mutex::new(entries),
                test_result: Mutex::new(TestResult { total: 0, passed: 0, failed: vec![] }),
                mock_extract_buffer: false,
                crate_handle: Mutex::new(None),
            }
        }

        pub fn with_count(n: usize) -> Self {
            let entries: Vec<ArchiveEntry> = (0..n).map(|i| ArchiveEntry {
                name: format!("file_{}.txt", i),
                path: format!("file_{}.txt", i),
                size: 100,
                compressed_size: 50,
                crc: Some(i as u32),
                original_index: i as u32,
                ..Default::default()
            }).collect();
            Self {
                entries: Mutex::new(entries),
                test_result: Mutex::new(TestResult { total: n, passed: n, failed: vec![] }),
                mock_extract_buffer: false,
                crate_handle: Mutex::new(None),
            }
        }

        pub fn arc_with_count(n: usize) -> Arc<dyn ArchiveRepository> {
            Arc::new(Self::with_count(n))
        }

        pub fn with_test_result(self, result: TestResult) -> Self {
            *self.test_result.lock().unwrap() = result;
            self
        }

        pub fn with_extract_buffer(mut self, enabled: bool) -> Self {
            self.mock_extract_buffer = enabled;
            self
        }

        pub fn entry_count(&self) -> usize {
            self.entries.lock().unwrap().len()
        }
    }

    impl ArchiveRepository for MockArchiveRepository {
        fn open(&self, _path: &Path, _password: Option<&Password>) -> Result<ArchiveHandle, ArchiveError> {
            let handle = ArchiveHandle::new(MOCK_NEXT_ID.fetch_add(1, Ordering::Relaxed));
            *self.crate_handle.lock().unwrap() = Some(handle.clone());
            Ok(handle)
        }

        fn create(&self, path: &Path, _format: ArchiveFormat, _encryption: Option<&EncryptionConfig>) -> Result<ArchiveHandle, ArchiveError> {
            let handle = ArchiveHandle::new(MOCK_NEXT_ID.fetch_add(1, Ordering::Relaxed))
                .with_path(path.to_path_buf());
            *self.crate_handle.lock().unwrap() = Some(handle.clone());
            Ok(handle)
        }

        fn list(&self, _archive: &ArchiveHandle, range: Range<usize>) -> Result<Page<ArchiveEntry>, ArchiveError> {
            let entries = self.entries.lock().unwrap();
            let items: Vec<ArchiveEntry> = entries.iter().skip(range.start).take(range.end.saturating_sub(range.start)).cloned().collect();
            Ok(Page::new(items, range.start, Some(entries.len())))
        }

        fn list_dir(&self, _archive: &ArchiveHandle, path: &str, range: Range<usize>) -> Result<Page<ArchiveEntry>, ArchiveError> {
            let entries = self.entries.lock().unwrap();
            let prefix = if path.is_empty() { String::new() } else { path.to_string() };
            let plen = prefix.len();
            let matching: Vec<ArchiveEntry> = entries.iter()
                .filter(|e| {
                    if plen == 0 { return !e.path.contains('/'); }
                    e.path.starts_with(&prefix) && e.path[plen..].find('/').is_none()
                })
                .cloned()
                .collect();
            let total = matching.len();
            let items: Vec<ArchiveEntry> = matching.into_iter().skip(range.start).take(range.end.saturating_sub(range.start)).collect();
            Ok(Page::new(items, range.start, Some(total)))
        }

        fn properties(&self, _archive: &ArchiveHandle) -> Result<ArchiveProperties, ArchiveError> {
            let entries = self.entries.lock().unwrap();
            let files = entries.iter().filter(|e| !e.is_directory).count() as u32;
            let folders = entries.iter().filter(|e| e.is_directory).count() as u32;
            Ok(ArchiveProperties {
                items_count: entries.len() as u32,
                folders_count: folders,
                files_count: files,
                total_size: entries.iter().map(|e| e.size).sum(),
                packed_size: entries.iter().map(|e| e.compressed_size).sum(),
                ..Default::default()
            })
        }

        fn extract(&self, _archive: &ArchiveHandle, _req: &ExtractRequest, _ctx: &OpCtx) -> Result<ExtractReport, ArchiveError> {
            Ok(ExtractReport::default())
        }

        fn extract_to_buffer(&self, _archive: &ArchiveHandle, index: u32) -> Result<Vec<u8>, ArchiveError> {
            if !self.mock_extract_buffer {
                return Err(ArchiveError::UnsupportedOperation);
            }
            let entries = self.entries.lock().unwrap();
            let entry = entries.iter().find(|e| e.original_index == index)
                .ok_or_else(|| ArchiveError::NotFound(format!("index {}", index)))?;
            let size = entry.size as usize;
            let fill = (index as u8).wrapping_mul(17);
            Ok(vec![fill; size.max(1)])
        }

        fn plan(&self, _archive: &ArchiveHandle, change_set: &ChangeSet) -> Result<ExecutionPlan, ArchiveError> {
            let entries = self.entries.lock().unwrap();
            Ok(crate::plan::plan_changes(&entries, change_set))
        }

        fn apply(&self, _archive: &ArchiveHandle, plan: &ExecutionPlan, _opts: &WriteOptions, _ctx: &OpCtx) -> Result<(), ArchiveError> {
            let mut entries = self.entries.lock().unwrap();

            for &idx in &plan.deletes {
                if let Some(pos) = entries.iter().position(|e| e.original_index == idx) {
                    entries.remove(pos);
                }
            }

            for &(idx, ref new_path) in &plan.renames {
                if let Some(entry) = entries.iter_mut().find(|e| e.original_index == idx) {
                    let new_name = new_path.rsplit('/').next().unwrap_or(new_path).to_string();
                    entry.name = new_name;
                    if let Some(slash_pos) = entry.path.rfind('/') {
                        entry.path = format!("{}/{}", &entry.path[..slash_pos], new_path);
                    } else {
                        entry.path = new_path.clone();
                    }
                } else {
                    return Err(ArchiveError::NotFound(format!("index {}", idx)));
                }
            }

            let next_idx = entries.len() as u32;
            for (i, (fs_path, archive_path)) in plan.adds.iter().enumerate() {
                let name = fs_path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| format!("file_{}", i));
                entries.push(ArchiveEntry {
                    name,
                    path: archive_path.clone(),
                    size: 100,
                    compressed_size: 50,
                    crc: Some(next_idx + i as u32),
                    original_index: next_idx + i as u32,
                    ..Default::default()
                });
            }

            for (i, e) in entries.iter_mut().enumerate() {
                e.original_index = i as u32;
            }

            Ok(())
        }

        fn test(&self, _archive: &ArchiveHandle, _indices: &[u32], _ctx: &OpCtx) -> Result<TestReport, ArchiveError> {
            let tr = self.test_result.lock().unwrap();
            Ok(TestReport {
                all_ok: tr.passed == tr.total,
                total: tr.total as u32,
                failed: tr.failed.clone(),
            })
        }

        fn build_archive(&self, _archive: &ArchiveHandle, _files: &[(PathBuf, String)], _out_path: &Path, _ctx: &OpCtx) -> Result<(), ArchiveError> {
            Ok(())
        }

        fn close(&self, _archive: &ArchiveHandle) {
            *self.crate_handle.lock().unwrap() = None;
        }
    }
}

#[cfg(test)]
mod archive_properties_tests {
    use super::*;

    #[test]
    fn test_default_properties_are_zero() {
        let props = ArchiveProperties::default();
        assert_eq!(props.items_count(), 0);
        assert_eq!(props.folders_count(), 0);
        assert_eq!(props.files_count(), 0);
        assert_eq!(props.total_size(), 0);
        assert_eq!(props.packed_size(), 0);
        assert!(!props.is_encrypted());
        assert!(!props.has_encrypted_items());
        assert!(!props.is_multi_volume());
        assert!(!props.is_solid());
        assert!(!props.encrypted_names());
        assert!(!props.has_comment());
        assert_eq!(props.comment_size(), None);
        assert!(!props.has_recovery_record());
        assert!(!props.locked());
        assert_eq!(props.dictionary_size(), None);
    }

    #[test]
    fn test_custom_properties() {
        let props = ArchiveProperties {
            items_count: 42, folders_count: 5, files_count: 37,
            total_size: 10240, packed_size: 5120,
            is_encrypted: true, has_encrypted_items: true,
            is_multi_volume: false, is_solid: true,
            encrypted_names: true, has_comment: true,
            comment_size: Some(128), has_recovery_record: false,
            locked: true, dictionary_size: Some(65536),
            headers_size: 1200,
            volumes_count: 1,
        };
        assert_eq!(props.items_count(), 42);
        assert!(props.is_encrypted());
        assert!(props.locked());
        assert_eq!(props.dictionary_size(), Some(65536));
    }

    #[test]
    fn test_files_plus_folders_equals_items() {
        let props = ArchiveProperties {
            items_count: 100, folders_count: 10, files_count: 90,
            ..Default::default()
        };
        assert_eq!(props.files_count() + props.folders_count(), props.items_count());
    }
}
