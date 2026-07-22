use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use bit7z_domain::archive::{
    ArchiveEntry, ArchiveFormat, ArchiveSession, Password, SessionId, TestFailure,
    TestFailureReason, TestResult,
};
use bit7z_domain::repository::{ArchiveError, ExtractOptions};
use bit7z_domain::vfs::VfsMetadata;
use bit7z_ports::ArchiveReader;

use bit7z_infra_bit7z as bit7z;
use bit7z_infra_progress::ProgressSender;

use crate::adapters::ffi_util::{
    detect_writer_format, read_all_entries, writer_format_to_archive_format,
};

/// FFI-backed reader adapter.
pub struct Bit7zReaderAdapter {
    lib: Arc<bit7z::Library>,
    handles: Mutex<HashMap<SessionId, bit7z::FfiHandle>>,
}

impl Bit7zReaderAdapter {
    pub fn new(lib: bit7z::Library) -> Self {
        Self {
            lib: Arc::new(lib),
            handles: Mutex::new(HashMap::new()),
        }
    }

    pub fn new_shared(lib: Arc<bit7z::Library>) -> Self {
        Self {
            lib,
            handles: Mutex::new(HashMap::new()),
        }
    }

    /// Returns the raw FFI pointer for a session.
    #[allow(dead_code)]
    fn raw_ptr(&self, session_id: SessionId) -> Result<*mut std::ffi::c_void, ArchiveError> {
        let guard = self
            .handles
            .lock()
            .map_err(|_| ArchiveError::Internal("[Bit7zReaderAdapter] lock poisoned".into()))?;
        guard.get(&session_id).map(|h| h.ptr()).ok_or_else(|| {
            ArchiveError::Internal(format!(
                "[Bit7zReaderAdapter] session {} not found",
                session_id
            ))
        })
    }

    pub fn close(&self, session_id: SessionId) {
        if let Ok(mut guard) = self.handles.lock() {
            guard.remove(&session_id);
        }
    }

    /// Extract with explicit progress sender.
    #[allow(dead_code)]
    pub fn extract_with_progress(
        &self,
        session: &ArchiveSession,
        indices: &[u32],
        dest: &Path,
        progress: ProgressSender,
    ) -> Result<(), ArchiveError> {
        use std::sync::Mutex as StdMutex;
        use std::sync::atomic::{AtomicU64, Ordering};

        let raw_ptr = self.raw_ptr(session.id)?;

        let dest_str = dest.to_str().ok_or_else(|| {
            ArchiveError::Internal(format!(
                "[extract_with_progress] destination path is not valid UTF-8: {}",
                dest.display()
            ))
        })?;

        struct ExtractCtx {
            progress: ProgressSender,
            current_file: StdMutex<String>,
            current_file_size: AtomicU64,
        }

        extern "C" fn extract_progress_callback(
            processed: u64,
            total: u64,
            ctx: *mut std::ffi::c_void,
        ) -> i32 {
            let ctx = unsafe { &*(ctx as *const ExtractCtx) };
            let file = ctx
                .current_file
                .lock()
                .ok()
                .map(|g| g.clone())
                .unwrap_or_default();
            let file = if file.is_empty() { None } else { Some(file) };
            let _file_total = ctx.current_file_size.load(Ordering::Relaxed);
            let _ = ctx.progress.send(bit7z_domain::repository::ProgressUpdate {
                file_current: processed,
                file_total: total,
                current_file: file,
                items_done: 0,
                items_total: 0,
                bytes_done: processed,
                bytes_total: total,
                error: None,
            });
            1
        }

        extern "C" fn extract_file_callback(
            name: *const std::ffi::c_char,
            size: u64,
            ctx: *mut std::ffi::c_void,
        ) {
            let ctx = unsafe { &*(ctx as *const ExtractCtx) };
            let name = unsafe {
                std::ffi::CStr::from_ptr(name)
                    .to_string_lossy()
                    .into_owned()
            };
            if let Ok(mut guard) = ctx.current_file.lock() {
                *guard = name;
            }
            ctx.current_file_size.store(size, Ordering::Relaxed);
        }

        let ctx = Box::into_raw(Box::new(ExtractCtx {
            progress,
            current_file: StdMutex::new(String::new()),
            current_file_size: AtomicU64::new(0),
        })) as *mut std::ffi::c_void;

        let ret = unsafe {
            let reader = std::mem::ManuallyDrop::new(bit7z::ArchiveReader::from_raw(
                bit7z::Handle::from_raw(raw_ptr),
            ));
            reader.extract_to_cb(
                indices,
                dest_str,
                ctx,
                None,
                Some(extract_progress_callback),
                Some(extract_file_callback),
            )
        };

        unsafe {
            drop(Box::from_raw(ctx as *mut ExtractCtx));
        }

        ret.map_err(|e| ArchiveError::Internal(format!("[extract_with_progress] {}", e)))
    }
}

impl ArchiveReader for Bit7zReaderAdapter {
    fn open(
        &self,
        path: &Path,
        password: Option<&Password>,
    ) -> Result<ArchiveSession, ArchiveError> {
        let path_str = path.to_str().ok_or_else(|| {
            ArchiveError::Internal(format!(
                "[Bit7zReaderAdapter::open] path is not valid UTF-8: {}",
                path.display()
            ))
        })?;

        let is_rar = path_str.to_lowercase().ends_with(".rar");

        let (is_header_encrypted, reader) = if is_rar {
            let r = match bit7z::ArchiveReader::open(self.lib.as_ref(), path_str, password) {
                Ok(r) => r,
                Err(_) if password.is_none() => {
                    return Err(ArchiveError::EncryptedArchiveRequiresPassword);
                }
                Err(e) => {
                    return Err(ArchiveError::Internal(format!(
                        "[open] failed to open RAR '{}': {}",
                        path_str, e
                    )));
                }
            };
            (false, r)
        } else {
            let enc = self.lib.as_ref().is_header_encrypted(path_str);
            if enc && password.is_none() {
                return Err(ArchiveError::EncryptedArchiveRequiresPassword);
            }
            let r =
                bit7z::ArchiveReader::open(self.lib.as_ref(), path_str, password).map_err(|e| {
                    ArchiveError::Internal(format!(
                        "[open] failed to open archive '{}': {}",
                        path_str, e
                    ))
                })?;
            (enc, r)
        };

        let has_encrypted_items = if is_header_encrypted {
            true
        } else {
            reader.has_encrypted_items()
        };

        let raw = reader.into_raw();
        let format = writer_format_to_archive_format(detect_writer_format(path));

        let session = ArchiveSession {
            id: bit7z_domain::archive::next_archive_id(),
            path: path.to_path_buf(),
            format: format.unwrap_or(ArchiveFormat::SevenZip),
            password: password.cloned(),
        };

        let mut guard = self.handles.lock().map_err(|_| {
            ArchiveError::Internal("[Bit7zReaderAdapter::open] lock poisoned".into())
        })?;
        guard.insert(session.id, bit7z::FfiHandle::reader(raw.as_ptr()));

        // Note: encryption info is not stored in ArchiveSession in the first version.
        let _ = has_encrypted_items;

        Ok(session)
    }

    fn read_entries(&self, session: &ArchiveSession) -> Result<Vec<ArchiveEntry>, ArchiveError> {
        let raw_ptr = self.raw_ptr(session.id)?;
        Ok(read_all_entries(raw_ptr))
    }

    fn read_metadata(
        &self,
        session: &ArchiveSession,
        index: u32,
    ) -> Result<VfsMetadata, ArchiveError> {
        let entries = self.read_entries(session)?;
        entries
            .into_iter()
            .find(|e| e.original_index == index)
            .map(|e| VfsMetadata {
                size: e.size,
                compressed_size: e.compressed_size,
                modified: e.modified,
                created: e.created,
                accessed: e.accessed,
                crc: e.crc,
                is_encrypted: e.is_encrypted,
                is_symlink: e.is_symlink,
                attributes: e.attributes,
                posix_attrib: e.posix_attrib,
                host_os: e.host_os,
                compression_method: e.compression_method,
                comment: e.comment,
                user: e.user,
                group: e.group,
                extension: e.extension,
                hardlink: e.hardlink,
            })
            .ok_or_else(|| {
                ArchiveError::Internal(format!("metadata not found for index {}", index))
            })
    }

    fn extract(
        &self,
        session: &ArchiveSession,
        indices: &[u32],
        dest: &Path,
        _options: &ExtractOptions,
    ) -> Result<(), ArchiveError> {
        let raw_ptr = self.raw_ptr(session.id)?;
        let dest_str = dest.to_str().ok_or_else(|| {
            ArchiveError::Internal(format!(
                "[Bit7zReaderAdapter::extract] destination path is not valid UTF-8: {}",
                dest.display()
            ))
        })?;

        unsafe {
            let reader = std::mem::ManuallyDrop::new(bit7z::ArchiveReader::from_raw(
                bit7z::Handle::from_raw(raw_ptr),
            ));
            reader
                .extract_to(indices, dest_str)
                .map_err(|e| ArchiveError::Internal(format!("[extract] {}", e)))
        }
    }

    fn extract_to_buffer(
        &self,
        session: &ArchiveSession,
        index: u32,
    ) -> Result<Vec<u8>, ArchiveError> {
        let raw_ptr = self.raw_ptr(session.id)?;
        unsafe {
            let reader = std::mem::ManuallyDrop::new(bit7z::ArchiveReader::from_raw(
                bit7z::Handle::from_raw(raw_ptr),
            ));
            reader
                .extract_to_buffer(index)
                .map_err(|e| ArchiveError::Internal(format!("[extract_to_buffer] {}", e)))
        }
    }

    fn test(&self, session: &ArchiveSession) -> Result<TestResult, ArchiveError> {
        let raw_ptr = self.raw_ptr(session.id)?;
        let count = unsafe { bit7z_ffi::bit7z_reader_item_count(raw_ptr as *mut _) };
        if count == 0 {
            return Ok(TestResult {
                total: 0,
                passed: 0,
                failed: vec![],
            });
        }

        let result = unsafe { bit7z_ffi::bit7z_reader_test(raw_ptr as *mut _) };
        if result.is_null() {
            return Err(ArchiveError::Internal(format!(
                "[Bit7zReaderAdapter::test] bit7z_reader_test returned null for session {}",
                session.id
            )));
        }

        let all_ok = unsafe { bit7z_ffi::bit7z_test_result_all_ok(result) } != 0;
        let total = unsafe { bit7z_ffi::bit7z_test_result_total(result) };
        let failed_count = unsafe { bit7z_ffi::bit7z_test_result_failed_count(result) };

        let mut failed_errors = Vec::new();
        if !all_ok && failed_count > 0 {
            let error_msg = unsafe {
                let ptr = bit7z_ffi::bit7z_test_result_error(result);
                if ptr.is_null() {
                    "test failed".to_string()
                } else {
                    std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned()
                }
            };
            failed_errors.push(error_msg);
        }

        unsafe { bit7z_ffi::bit7z_test_result_free(result) };

        let passed = total.saturating_sub(failed_count);
        let mut failures = Vec::new();
        if !all_ok {
            if failed_count > 0 && total > 0 {
                for i in 0..failed_count.min(total) {
                    let error = failed_errors
                        .first()
                        .cloned()
                        .unwrap_or_else(|| "test failed".into());
                    failures.push(TestFailure {
                        entry_path: format!("index {}", i),
                        error: error.clone(),
                        index: i as usize,
                        path: String::new(),
                        reason: TestFailureReason::ReadError(error),
                    });
                }
            } else if failed_count > 0 {
                let error_msg = failed_errors
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "test failed".into());
                failures.push(TestFailure {
                    entry_path: String::new(),
                    error: error_msg.clone(),
                    index: 0,
                    path: String::new(),
                    reason: TestFailureReason::ReadError(error_msg),
                });
            }
        }

        Ok(TestResult {
            total: total as usize,
            passed: passed as usize,
            failed: failures,
        })
    }
}
