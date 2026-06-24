use crate::application::progress::{ProgressSender, ProgressUpdate};
use crate::domain::archive::*;
use crate::domain::repository::*;
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
        self.repo.test(archive)
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
        progress: Option<ProgressSender>,
    ) -> Result<TestResult, ArchiveError> {
        let count = match self.repo.get_properties(archive).ok() {
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
            let page = match self.repo.list_page(archive, index as usize, 1) {
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
                match self.repo.list_directory(archive, &dir_path) {
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
            }
        }
        expanded.sort();
        expanded.dedup();

        let mut passed = 0usize;
        let mut failed = Vec::new();
        let total = expanded.len();
        let mut bytes_processed: u64 = 0;

        for (done, &index) in expanded.iter().enumerate() {
            let page = match self.repo.list_page(archive, index as usize, 1) {
                Ok(p) => p,
                Err(e) => {
                    failed.push(TestFailure {
                        entry_path: format!("index {}", index),
                        error: format!("{}", e),
                        index: index as usize,
                        path: String::new(),
                        reason: TestFailureReason::ReadError(format!("{}", e)),
                    });
                    if let Some(ref tx) = progress {
                        let _ = tx.send(ProgressUpdate {
                            file_current: 0,
                            file_total: 0,
                            current_file: None,
                            items_done: (done + 1) as u64,
                            items_total: total as u64,
                            bytes_done: bytes_processed,
                            bytes_total: total_bytes,
                            error: Some(format!("{}", e)),
                        });
                    }
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
                if let Some(ref tx) = progress {
                    let _ = tx.send(ProgressUpdate {
                        file_current: 0,
                        file_total: 0,
                        current_file: None,
                        items_done: (done + 1) as u64,
                        items_total: total as u64,
                        bytes_done: bytes_processed,
                        bytes_total: total_bytes,
                        error: Some("entry not found".into()),
                    });
                }
                continue;
            }
            let entry = &page.items[0];

            // Directories without children (already expanded above) auto-pass
            if entry.is_directory {
                passed += 1;
                if let Some(ref tx) = progress {
                    let _ = tx.send(ProgressUpdate {
                        file_current: 0,
                        file_total: entry.size,
                        current_file: Some(entry.path.clone()),
                        items_done: (done + 1) as u64,
                        items_total: total as u64,
                        bytes_done: bytes_processed,
                        bytes_total: total_bytes,
                        error: None,
                    });
                }
                continue;
            }

            // Extract to buffer
            let data = match self.repo.extract_to_buffer(archive, index) {
                Ok(d) => d,
                Err(e) => {
                    failed.push(TestFailure {
                        entry_path: entry.path.clone(),
                        error: format!("{}", e),
                        index: index as usize,
                        path: entry.path.clone(),
                        reason: TestFailureReason::ReadError(format!("{}", e)),
                    });
                    if let Some(ref tx) = progress {
                        let _ = tx.send(ProgressUpdate {
                            file_current: 0,
                            file_total: entry.size,
                            current_file: Some(entry.path.clone()),
                            items_done: (done + 1) as u64,
                            items_total: total as u64,
                            bytes_done: bytes_processed,
                            bytes_total: total_bytes,
                            error: Some(format!("{}", e)),
                        });
                    }
                    continue;
                }
            };

            bytes_processed += entry.size;

            // Compute CRC
            let computed_crc = crc32fast::hash(&data);

            // Compare with stored CRC if available
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
                    if let Some(ref tx) = progress {
                        let _ = tx.send(ProgressUpdate {
                            file_current: 0,
                            file_total: entry.size,
                            current_file: Some(entry.path.clone()),
                            items_done: (done + 1) as u64,
                            items_total: total as u64,
                            bytes_done: bytes_processed,
                            bytes_total: total_bytes,
                            error: Some("CRC mismatch".into()),
                        });
                    }
                    continue;
                }
            }

            passed += 1;

            if let Some(ref tx) = progress {
                let _ = tx.send(ProgressUpdate {
                    file_current: 0,
                    file_total: entry.size,
                    current_file: Some(entry.path.clone()),
                    items_done: (done + 1) as u64,
                    items_total: total as u64,
                    bytes_done: bytes_processed,
                    bytes_total: total_bytes,
                    error: None,
                });
            }
        }

        Ok(TestResult {
            total,
            passed,
            failed,
        })
    }
}
