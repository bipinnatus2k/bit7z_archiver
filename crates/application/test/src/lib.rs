use bit7z_app_archive::runtime_service::ArchiveService;
use bit7z_domain::archive::{ArchiveHandle, TestFailure, TestFailureReason, TestResult};
use bit7z_domain::repository::{ArchiveError, ProgressUpdate};
use bit7z_infra_progress::ProgressSender;
use std::sync::Arc;

pub struct TestArchiveUseCase {
    service: Arc<ArchiveService>,
}

impl TestArchiveUseCase {
    pub fn new(service: Arc<ArchiveService>) -> Self {
        Self { service }
    }

    /// Test entire archive (FFI-backed, fast).
    pub fn execute(&self, archive: &ArchiveHandle) -> Result<TestResult, ArchiveError> {
        self.service.test(archive)
    }
}

pub struct TestEntriesUseCase {
    service: Arc<ArchiveService>,
}

impl TestEntriesUseCase {
    pub fn new(service: Arc<ArchiveService>) -> Self {
        Self { service }
    }

    /// Test specific entries by extracting each and comparing CRC.
    /// `indices: None` means test every entry in the archive.
    pub fn execute(
        &self,
        archive: &ArchiveHandle,
        indices: Option<&[u32]>,
        progress: Option<ProgressSender>,
    ) -> Result<TestResult, ArchiveError> {
        fn notify(
            progress: &Option<ProgressSender>,
            items_done: u64,
            items_total: u64,
            bytes_done: u64,
            bytes_total: u64,
            current_file: Option<String>,
            file_total: u64,
            error: Option<String>,
        ) {
            if let Some(tx) = progress {
                let _ = tx.send(ProgressUpdate {
                    file_current: 0,
                    file_total,
                    current_file,
                    items_done,
                    items_total,
                    bytes_done,
                    bytes_total,
                    error,
                });
            }
        }

        let count = match self.service.get_properties(archive).ok() {
            Some(p) => p.items_count as usize,
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
            let page = match self.service.list_page(archive, index as usize, 1) {
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
            if !entry.is_directory {
                total_bytes += entry.size;
                expanded.push(index);
                continue;
            }

            // Recursively collect all file indices under this directory.
            let mut files: Vec<u32> = Vec::new();
            let dir_path = if entry.path.ends_with('/') {
                entry.path.clone()
            } else {
                format!("{}/", entry.path)
            };
            let mut stack = vec![dir_path];
            while let Some(dir_path) = stack.pop() {
                match self.service.list_directory(archive, &dir_path) {
                    Ok(children) => {
                        for child in children {
                            if child.is_directory {
                                let child_path = if child.path.ends_with('/') {
                                    child.path.clone()
                                } else {
                                    format!("{}/", child.path)
                                };
                                stack.push(child_path);
                            } else {
                                total_bytes += child.size;
                                files.push(child.original_index);
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

            let page = match self.service.list_page(archive, index as usize, 1) {
                Ok(p) => p,
                Err(e) => {
                    failed.push(TestFailure {
                        entry_path: format!("index {}", index),
                        error: format!("{}", e),
                        index: index as usize,
                        path: String::new(),
                        reason: TestFailureReason::ReadError(format!("{}", e)),
                    });
                    notify(
                        &progress,
                        items_done,
                        items_total,
                        bytes_processed,
                        total_bytes,
                        None,
                        0,
                        Some(format!("{}", e)),
                    );
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
                notify(
                    &progress,
                    items_done,
                    items_total,
                    bytes_processed,
                    total_bytes,
                    None,
                    0,
                    Some("entry not found".into()),
                );
                continue;
            }
            let entry = &page.items[0];

            if entry.is_directory {
                passed += 1;
                notify(
                    &progress,
                    items_done,
                    items_total,
                    bytes_processed,
                    total_bytes,
                    Some(entry.path.clone()),
                    entry.size,
                    None,
                );
                continue;
            }

            let data = match self.service.extract_to_buffer(archive, index) {
                Ok(d) => d,
                Err(e) => {
                    failed.push(TestFailure {
                        entry_path: entry.path.clone(),
                        error: format!("{}", e),
                        index: index as usize,
                        path: entry.path.clone(),
                        reason: TestFailureReason::ReadError(format!("{}", e)),
                    });
                    notify(
                        &progress,
                        items_done,
                        items_total,
                        bytes_processed,
                        total_bytes,
                        Some(entry.path.clone()),
                        entry.size,
                        Some(format!("{}", e)),
                    );
                    continue;
                }
            };

            bytes_processed += entry.size;

            let computed_crc = crc32fast::hash(&data);

            if let Some(stored_crc) = entry.crc {
                if computed_crc != stored_crc {
                    failed.push(TestFailure {
                        entry_path: entry.path.clone(),
                        error: format!(
                            "CRC mismatch: expected {:08x}, got {:08x}",
                            stored_crc, computed_crc
                        ),
                        index: index as usize,
                        path: entry.path.clone(),
                        reason: TestFailureReason::CrcMismatch {
                            expected: stored_crc,
                            actual: computed_crc,
                        },
                    });
                    notify(
                        &progress,
                        items_done,
                        items_total,
                        bytes_processed,
                        total_bytes,
                        Some(entry.path.clone()),
                        entry.size,
                        Some("CRC mismatch".into()),
                    );
                    continue;
                }
            }

            passed += 1;
            notify(
                &progress,
                items_done,
                items_total,
                bytes_processed,
                total_bytes,
                Some(entry.path.clone()),
                entry.size,
                None,
            );
        }

        Ok(TestResult {
            total,
            passed,
            failed,
        })
    }
}
