use bit7z_domain::archive::*;
use bit7z_domain::repository::*;
use std::ops::Range;
use std::sync::Arc;

pub struct TestArchiveUseCase {
    repo: Arc<dyn ArchiveRepository>,
}

impl TestArchiveUseCase {
    pub fn new(repo: Arc<dyn ArchiveRepository>) -> Self {
        Self { repo }
    }

    /// Test entire archive (FFI-backed, fast).
    pub fn execute(&self, archive: &ArchiveHandle) -> Result<TestResult, ArchiveError> {
        let ctx = OpCtx {
            cancel: CancellationToken::new(),
            pause: PauseToken::new(),
            progress: Arc::new(NoopSink),
        };
        let report = self.repo.test(archive, &[], &ctx)?;
        Ok(TestResult {
            total: report.total as usize,
            passed: (report.total as usize).saturating_sub(report.failed.len()),
            failed: report.failed,
        })
    }
}

pub struct TestEntriesUseCase {
    repo: Arc<dyn ArchiveRepository>,
}

impl TestEntriesUseCase {
    pub fn new(repo: Arc<dyn ArchiveRepository>) -> Self {
        Self { repo }
    }

    /// Test specific entries by extracting each and comparing CRC.
    /// `indices: None` means test every entry in the archive.
    pub fn execute(
        &self,
        archive: &ArchiveHandle,
        indices: Option<&[u32]>,
        progress: Option<Arc<dyn ProgressSink>>,
    ) -> Result<TestResult, ArchiveError> {
        fn notify(
            progress: &Option<Arc<dyn ProgressSink>>,
            _items_done: u64,
            _items_total: u64,
            bytes_done: u64,
            bytes_total: u64,
            current_file: Option<String>,
            _file_total: u64,
            _error: Option<String>,
        ) {
            if let Some(sink) = progress {
                sink.on_progress(bytes_done, bytes_total);
                if let Some(path) = current_file {
                    sink.on_file(&path);
                }
            }
        }

        let count = match self.repo.properties(archive).ok() {
            Some(p) => p.items_count() as usize,
            None => return Err(ArchiveError::Internal("failed to read properties".into())),
        };

        let idx_list: Vec<u32> = match indices {
            Some(i) => i.to_vec(),
            None => (0..count as u32).collect(),
        };

        // Flatten directories: expand directory entries into their child
        // file indices so nested files are tested too.
        let mut expanded: Vec<u32> = Vec::new();
        let mut total_bytes: u64 = 0;
        for &index in &idx_list {
            let page = match self.repo.list(archive, index as usize..index as usize + 1) {
                Ok(p) => p,
                Err(_) => {
                    expanded.push(index);
                    continue;
                }
            };
            if page.items.is_empty() {
                expanded.push(index);
                continue;
            }
            let entry = &page.items[0];
            if !entry.is_directory() {
                total_bytes += entry.size();
                expanded.push(index);
                continue;
            }

            // Recursively collect all file indices under this directory.
            let mut files: Vec<u32> = Vec::new();
            let dir_path = if entry.path().ends_with('/') {
                entry.path().to_string()
            } else {
                format!("{}/", entry.path())
            };
            let mut stack = vec![dir_path];
            while let Some(dir_path) = stack.pop() {
                match self.repo.list_dir(archive, &dir_path, 0..usize::MAX) {
                    Ok(page) => {
                        for child in page.items {
                            if child.is_directory() {
                                let child_path = if child.path().ends_with('/') {
                                    child.path().to_string()
                                } else {
                                    format!("{}/", child.path())
                                };
                                stack.push(child_path);
                            } else {
                                total_bytes += child.size();
                                files.push(child.original_index());
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
            if !files.is_empty() {
                expanded.extend(files);
            } else {
                expanded.push(index);
            }
        }
        expanded.sort();
        expanded.dedup();

        let mut passed = 0usize;
        let mut failed = Vec::new();
        let total = expanded.len();
        let mut bytes_processed: u64 = 0;

        for (done, &index) in expanded.iter().enumerate() {
            let items_done = (done + 1) as u64;
            let items_total = total as u64;

            let page = match self.repo.list(archive, index as usize..index as usize + 1) {
                Ok(p) => p,
                Err(e) => {
                    failed.push(TestFailure {
                        entry_path: format!("index {}", index),
                        error: format!("{}", e),
                        index: index as usize,
                        path: String::new(),
                        reason: TestFailureReason::ReadError(format!("{}", e)),
                    });
                    notify(&progress, items_done, items_total, bytes_processed, total_bytes, None, 0, Some(format!("{}", e)));
                    continue;
                }
            };
            if page.items.is_empty() {
                failed.push(TestFailure {
                    entry_path: format!("index {}", index),
                    error: "entry not found".into(),
                    index: index as usize,
                    path: String::new(),
                    reason: TestFailureReason::ReadError("entry not found".into()),
                });
                notify(&progress, items_done, items_total, bytes_processed, total_bytes, None, 0, Some("entry not found".into()));
                continue;
            }
            let entry = &page.items[0];

            if entry.is_directory() {
                passed += 1;
                notify(&progress, items_done, items_total, bytes_processed, total_bytes, Some(entry.path().to_string()), entry.size(), None);
                continue;
            }

            let data = match self.repo.extract_to_buffer(archive, index) {
                Ok(d) => d,
                Err(e) => {
                    failed.push(TestFailure {
                        entry_path: entry.path().to_string(),
                        error: format!("{}", e),
                        index: index as usize,
                        path: entry.path().to_string(),
                        reason: TestFailureReason::ReadError(format!("{}", e)),
                    });
                    notify(&progress, items_done, items_total, bytes_processed, total_bytes, Some(entry.path().to_string()), entry.size(), Some(format!("{}", e)));
                    continue;
                }
            };

            bytes_processed += entry.size();

            let computed_crc = crc32fast::hash(&data);

            let stored_crc = entry.crc();
            if stored_crc != 0 {
                if computed_crc != stored_crc {
                    failed.push(TestFailure {
                        entry_path: entry.path().to_string(),
                        error: format!(
                            "CRC mismatch: expected {:08x}, got {:08x}",
                            stored_crc, computed_crc
                        ),
                        index: index as usize,
                        path: entry.path().to_string(),
                        reason: TestFailureReason::CrcMismatch {
                            expected: stored_crc,
                            actual: computed_crc,
                        },
                    });
                    notify(&progress, items_done, items_total, bytes_processed, total_bytes, Some(entry.path().to_string()), entry.size(), Some("CRC mismatch".into()));
                    continue;
                }
            }

            passed += 1;
            notify(&progress, items_done, items_total, bytes_processed, total_bytes, Some(entry.path().to_string()), entry.size(), None);
        }

        Ok(TestResult {
            total,
            passed,
            failed,
        })
    }
}

#[cfg(test)]
mod tests {
use super::*;
use bit7z_domain::repository::test_utils::MockArchiveRepository;
use std::sync::Arc;

    #[test]
    fn test_test_archive_all_passed() {
        let result = TestResult { total: 5, passed: 5, failed: vec![] };
        let mock = MockArchiveRepository::with_count(5).with_test_result(result);
        let repo: Arc<dyn ArchiveRepository> = Arc::new(mock);
        let uc = TestArchiveUseCase::new(repo);
        let handle = ArchiveHandle::new(0);
        let res = uc.execute(&handle).unwrap();
        assert_eq!(res.total, 5);
        assert_eq!(res.passed, 5);
        assert!(res.failed.is_empty());
    }

    #[test]
    fn test_test_archive_some_failed() {
        let failures = vec![
            TestFailure {
                entry_path: "bad.txt".into(), error: "CRC mismatch".into(),
                index: 1, path: "bad.txt".into(),
                reason: TestFailureReason::CrcMismatch { expected: 0x1234, actual: 0x5678 },
            },
        ];
        let result = TestResult { total: 3, passed: 2, failed: failures };
        let mock = MockArchiveRepository::with_count(3).with_test_result(result);
        let repo: Arc<dyn ArchiveRepository> = Arc::new(mock);
        let uc = TestArchiveUseCase::new(repo);
        let handle = ArchiveHandle::new(0);
        let res = uc.execute(&handle).unwrap();
        assert_eq!(res.total, 3);
        assert_eq!(res.passed, 2);
        assert_eq!(res.failed.len(), 1);
    }

    #[test]
    fn test_test_archive_error_propagated() {
        struct FailTest;
        impl ArchiveRepository for FailTest {
            fn open(&self, _: &std::path::Path, _: Option<&Password>) -> Result<ArchiveHandle, ArchiveError> { Err(ArchiveError::UnsupportedOperation) }
            fn create(&self, _: &std::path::Path, _: ArchiveFormat, _: Option<&EncryptionConfig>) -> Result<ArchiveHandle, ArchiveError> { Err(ArchiveError::UnsupportedOperation) }
            fn list(&self, _: &ArchiveHandle, _: Range<usize>) -> Result<Page<ArchiveEntry>, ArchiveError> { Ok(Page::new(vec![], 0, Some(0))) }
            fn list_dir(&self, _: &ArchiveHandle, _: &str, _: Range<usize>) -> Result<Page<ArchiveEntry>, ArchiveError> { Ok(Page::new(vec![], 0, Some(0))) }
            fn properties(&self, _: &ArchiveHandle) -> Result<ArchiveProperties, ArchiveError> { Ok(ArchiveProperties::default()) }
            fn extract(&self, _: &ArchiveHandle, _: &ExtractRequest, _: &OpCtx) -> Result<ExtractReport, ArchiveError> { Ok(ExtractReport::default()) }
            fn extract_to_buffer(&self, _: &ArchiveHandle, _: u32) -> Result<Vec<u8>, ArchiveError> { Err(ArchiveError::UnsupportedOperation) }
            fn plan(&self, _: &ArchiveHandle, _: &ChangeSet) -> Result<bit7z_domain::plan::ExecutionPlan, ArchiveError> { Err(ArchiveError::UnsupportedOperation) }
            fn apply(&self, _: &ArchiveHandle, _: &bit7z_domain::plan::ExecutionPlan, _: &WriteOptions, _: &OpCtx) -> Result<(), ArchiveError> { Err(ArchiveError::UnsupportedOperation) }
            fn test(&self, _: &ArchiveHandle, _: &[u32], _: &OpCtx) -> Result<TestReport, ArchiveError> { Err(ArchiveError::Internal("test failed".into())) }
            fn close(&self, _: &ArchiveHandle) {}
        }
        let uc = TestArchiveUseCase::new(Arc::new(FailTest));
        let handle = ArchiveHandle::new(0);
        let result = uc.execute(&handle);
        assert!(matches!(result, Err(ArchiveError::Internal(ref msg)) if msg == "test failed"));
    }
}
