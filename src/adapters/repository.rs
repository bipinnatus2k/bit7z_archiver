use crate::adapters::bit7z;
use crate::application::progress::ProgressUpdate;
use crate::domain::archive::*;
use crate::domain::repository::*;
use chrono::DateTime;
use crossbeam::channel::Sender;
use std::path::Path;
use std::sync::Mutex;

/// Detect writer format from archive path extension.
fn detect_writer_format(path: &Path) -> bit7z::WriterFormat {
    path.extension()
        .and_then(|ext| {
            let ext = ext.to_string_lossy().to_lowercase();
            match ext.as_str() {
                "7z" => Some(bit7z::WriterFormat::SevenZip),
                "zip" => Some(bit7z::WriterFormat::Zip),
                "tar" => Some(bit7z::WriterFormat::Tar),
                "gz" | "tgz" => Some(bit7z::WriterFormat::GZip),
                "bz2" | "tbz" | "tbz2" => Some(bit7z::WriterFormat::BZip2),
                "xz" | "txz" => Some(bit7z::WriterFormat::Xz),
                _ => None,
            }
        })
        .unwrap_or(bit7z::WriterFormat::SevenZip)
}

/// Populate extended fields on an ArchiveEntry using the FFI item accessors.
fn populate_item_details(entry: &mut ArchiveEntry, raw: *mut std::ffi::c_void, index: u32) {
    entry.crc = Some(unsafe { crate::ffi::bit7z_item_crc(raw as *mut _, index) });

    let item_ptr = unsafe { crate::ffi::bit7z_item_from_reader(raw as *mut _, index) };
    if item_ptr.is_null() { return; }

    let mtime = unsafe { crate::ffi::bit7z_item_mtime(item_ptr) };
    if mtime > 0 { entry.modified = DateTime::from_timestamp(mtime as i64, 0); }

    let ctime = unsafe { crate::ffi::bit7z_item_ctime(item_ptr) };
    if ctime > 0 { entry.created = DateTime::from_timestamp(ctime as i64, 0); }

    let atime = unsafe { crate::ffi::bit7z_item_atime(item_ptr) };
    if atime > 0 { entry.accessed = DateTime::from_timestamp(atime as i64, 0); }

    entry.attributes = Some(unsafe { crate::ffi::bit7z_item_attributes(item_ptr) });
    entry.host_os = Some(unsafe { crate::ffi::bit7z_item_host_os(item_ptr) });
    entry.posix_attrib = Some(unsafe { crate::ffi::bit7z_item_posix_attrib(item_ptr) });
    entry.is_symlink = unsafe { crate::ffi::bit7z_item_is_symlink(item_ptr) != 0 };

    let mut buf: Vec<u8> = vec![0u8; 256];

    let ret = unsafe {
        crate::ffi::bit7z_item_compression_method(item_ptr, buf.as_mut_ptr() as *mut _, 256)
    };
    if ret >= 0 {
        let s = unsafe { std::ffi::CStr::from_ptr(buf.as_ptr() as *const _) };
        let s = s.to_string_lossy().into_owned();
        if !s.is_empty() { entry.compression_method = Some(s); }
    }

    buf.fill(0);
    let ret = unsafe { crate::ffi::bit7z_item_comment(item_ptr, buf.as_mut_ptr() as *mut _, 256) };
    if ret >= 0 {
        let s = unsafe { std::ffi::CStr::from_ptr(buf.as_ptr() as *const _) };
        let s = s.to_string_lossy().into_owned();
        if !s.is_empty() { entry.comment = Some(s); }
    }

    buf.fill(0);
    let ret = unsafe { crate::ffi::bit7z_item_user(item_ptr, buf.as_mut_ptr() as *mut _, 256) };
    if ret >= 0 {
        let s = unsafe { std::ffi::CStr::from_ptr(buf.as_ptr() as *const _) };
        let s = s.to_string_lossy().into_owned();
        if !s.is_empty() { entry.user = Some(s); }
    }

    buf.fill(0);
    let ret = unsafe { crate::ffi::bit7z_item_group(item_ptr, buf.as_mut_ptr() as *mut _, 256) };
    if ret >= 0 {
        let s = unsafe { std::ffi::CStr::from_ptr(buf.as_ptr() as *const _) };
        let s = s.to_string_lossy().into_owned();
        if !s.is_empty() { entry.group = Some(s); }
    }
}

/// FFI-backed implementation of ArchiveRepository using the C wrapper layer.
pub struct Bit7zRepository {
    lib: Mutex<bit7z::Library>,
    progress_sender: Mutex<Option<Sender<ProgressUpdate>>>,
}

unsafe impl Send for Bit7zRepository {}
unsafe impl Sync for Bit7zRepository {}

impl Bit7zRepository {
    pub fn new(lib: bit7z::Library) -> Self {
        Self { lib: Mutex::new(lib), progress_sender: Mutex::new(None) }
    }

    /// Lock the library mutex, recovering from poison if a previous holder panicked.
    fn lock_lib(&self) -> Result<std::sync::MutexGuard<'_, bit7z::Library>, ArchiveError> {
        self.lib.lock().or_else(|poisoned| {
            log::warn!("Mutex was poisoned, recovering library handle");
            Ok(poisoned.into_inner())
        })
    }
}

impl ArchiveRepository for Bit7zRepository {
    fn set_progress_sender(&self, tx: Sender<ProgressUpdate>) {
        if let Ok(mut guard) = self.progress_sender.lock() {
            *guard = Some(tx);
        }
    }

    fn open(&self, path: &Path, password: Option<&Password>) -> Result<ArchiveHandle, ArchiveError> {
        let lib = self.lock_lib()?;
        let path_str = path.to_str()
            .ok_or_else(|| ArchiveError::Internal("Non-UTF-8 path".into()))?;

        // Detect if archive has encrypted headers (static check without opening)
        let is_header_encrypted = lib.is_header_encrypted(path_str);

        // Return specific error if password is required but not provided
        if is_header_encrypted && password.is_none() {
            return Err(ArchiveError::EncryptedArchiveRequiresPassword);
        }

        let reader = bit7z::ArchiveReader::open(&lib, path_str, password)
            .map_err(|e| ArchiveError::Internal(e))?;

        // Check if opened archive has any encrypted items
        let has_encrypted_items = if is_header_encrypted {
            true // Header encrypted implies items are encrypted
        } else {
            reader.has_encrypted_items() // Check via instance method
        };

        let raw_handle = reader.into_raw();

        // Detect format from extension
        let format = path.extension().and_then(|ext| {
            let ext = ext.to_string_lossy().to_lowercase();
            match ext.as_str() {
                "7z" => Some(ArchiveFormat::SevenZip),
                "zip" => Some(ArchiveFormat::Zip),
                "tar" => Some(ArchiveFormat::Tar),
                "gz" | "tgz" => Some(ArchiveFormat::TarGz),
                "bz2" | "tbz" | "tbz2" => Some(ArchiveFormat::TarBz2),
                "xz" | "txz" => Some(ArchiveFormat::TarXz),
                "rar" => Some(ArchiveFormat::Rar),
                _ => None,
            }
        });

        let mut handle = ArchiveHandle::new_reader(raw_handle as *mut std::ffi::c_void)
            .with_path(path.to_path_buf())
            .with_format_opt(format);
        handle.set_encryption_info(is_header_encrypted, has_encrypted_items);
        Ok(handle)
    }

    fn create(&self, path: &Path, format: ArchiveFormat,
              encryption: Option<&EncryptionConfig>) -> Result<ArchiveHandle, ArchiveError> {
        let lib = self.lock_lib()?;
        let path_str = path.to_str()
            .ok_or_else(|| ArchiveError::Internal("Non-UTF-8 path".into()))?;

        let writer_format = match format {
            ArchiveFormat::SevenZip => bit7z::WriterFormat::SevenZip,
            ArchiveFormat::Zip => bit7z::WriterFormat::Zip,
            ArchiveFormat::Tar => bit7z::WriterFormat::Tar,
            ArchiveFormat::TarGz => bit7z::WriterFormat::GZip,
            ArchiveFormat::TarBz2 => bit7z::WriterFormat::BZip2,
            ArchiveFormat::TarXz => bit7z::WriterFormat::Xz,
            ArchiveFormat::Rar => return Err(ArchiveError::UnsupportedOperation),
        };

        let password = encryption.map(|e| e.password.as_str());
        let writer = bit7z::Writer::create(&lib, writer_format)
            .map_err(|e| ArchiveError::Internal(e))?;

        // Set encryption if provided
        if let Some(enc) = encryption {
            if !enc.password.is_empty() {
                writer.set_password(enc.password.as_str());
            }
        }

        let raw_handle = writer.into_raw();
        Ok(ArchiveHandle::new_writer(raw_handle as *mut std::ffi::c_void)
            .with_path(path.to_path_buf())
            .with_format(format))
    }

    fn list_page(&self, archive: &ArchiveHandle, offset: usize, limit: usize)
                 -> Result<Page<ArchiveEntry>, ArchiveError> {
        let raw = archive.raw as usize;
        let count = unsafe { crate::ffi::bit7z_reader_item_count(raw as *mut _) };

        let mut entries = Vec::new();
        let start = (offset as u32).min(count);
        let end = (start + limit as u32).min(count);

        for i in start..end {
            use std::ffi::CStr;
            let p = unsafe { crate::ffi::bit7z_item_path(raw as *mut _, i) };
            let n = unsafe { crate::ffi::bit7z_item_name(raw as *mut _, i) };
            let path_s = if p.is_null() { String::new() }
                         else { unsafe { CStr::from_ptr(p).to_string_lossy().into_owned() } };
            let name_s = if n.is_null() { String::new() }
                         else { unsafe { CStr::from_ptr(n).to_string_lossy().into_owned() } };
            let size = unsafe { crate::ffi::bit7z_item_size(raw as *mut _, i) };
            let csize = unsafe { crate::ffi::bit7z_item_packed_size(raw as *mut _, i) };
            let is_dir = unsafe { crate::ffi::bit7z_item_is_dir(raw as *mut _, i) != 0 };
            let is_enc = unsafe { crate::ffi::bit7z_item_is_encrypted(raw as *mut _, i) != 0 };

            let mut entry = ArchiveEntry {
                name: name_s, path: path_s,
                size, compressed_size: csize,
                is_directory: is_dir, is_encrypted: is_enc,
                original_index: i,
                ..Default::default()
            };
            populate_item_details(&mut entry, raw as *mut std::ffi::c_void, i);
            entries.push(entry);
        }
        Ok(Page::new(entries, offset, Some(count as usize)))
    }

    fn get_properties(&self, archive: &ArchiveHandle) -> Result<ArchiveProperties, ArchiveError> {
        let raw = archive.raw as usize;
        let count = unsafe { crate::ffi::bit7z_reader_item_count(raw as *mut _) };
        let mut folders = 0u32;
        let mut files = 0u32;
        let mut total_size = 0u64;
        let mut packed_size = 0u64;
        for i in 0..count {
            let is_dir = unsafe { crate::ffi::bit7z_item_is_dir(raw as *mut _, i) != 0 };
            if is_dir { folders += 1; } else { files += 1; }
            total_size += unsafe { crate::ffi::bit7z_item_size(raw as *mut _, i) };
            packed_size += unsafe { crate::ffi::bit7z_item_packed_size(raw as *mut _, i) };
        }
        Ok(ArchiveProperties {
            items_count: count,
            folders_count: folders,
            files_count: files,
            total_size, packed_size,
            is_encrypted: archive.is_header_encrypted(),
            has_encrypted_items: archive.has_encrypted_items(),
            ..Default::default()
        })
    }

    fn extract(&self, archive: &ArchiveHandle, indices: &[u32], dest: &Path)
               -> Result<(), ArchiveError> {
        let raw = archive.raw as usize;
        let c_dest = std::ffi::CString::new(dest.to_str().ok_or_else(|| {
            ArchiveError::Internal("Invalid destination path".into())
        })?).map_err(|e| ArchiveError::Internal(e.to_string()))?;
        let ret: i32 = unsafe {
            crate::ffi::bit7z_reader_extract_to(
                raw as *mut _, indices.as_ptr(), indices.len() as u32, c_dest.as_ptr(),
            )
        };
        if ret != 0 { Err(ArchiveError::Internal("Extraction failed".into())) }
        else { Ok(()) }
    }

    fn extract_to_buffer(&self, archive: &ArchiveHandle, index: u32)
                         -> Result<Vec<u8>, ArchiveError> {
        let raw = archive.raw as usize;
        let mut out_data: *mut std::ffi::c_void = std::ptr::null_mut();
        let mut out_size: i64 = 0;
        let ret = unsafe {
            crate::adapters::bit7z::bit7z_reader_extract_to_buffer_c(
                raw as *mut _,
                index,
                &mut out_data,
                &mut out_size,
            )
        };
        if ret != 0 || out_data.is_null() || out_size <= 0 {
            return Err(ArchiveError::Internal("Extract buffer failed".into()));
        }
        let slice = unsafe { std::slice::from_raw_parts(out_data as *const u8, out_size as usize) };
        let result = slice.to_vec();
        unsafe { crate::ffi::bit7z_reader_free_buffer(out_data as *mut autocxx::c_void); }
        Ok(result)
    }

    fn add(&self, archive: &mut ArchiveHandle, files: &[std::path::PathBuf], password: Option<&Password>)
           -> Result<(), ArchiveError> {
        // Save archive path before closing the reader to avoid holding a borrow
        // across the mutable update to archive.raw.
        let archive_path = archive.path.clone()
            .ok_or_else(|| ArchiveError::Internal("No path in archive handle".into()))?;
        let archive_path_str = archive_path.to_str()
            .ok_or_else(|| ArchiveError::Internal("Non-UTF-8 path".into()))?
            .to_string();

        // Close the reader to release any file locks before opening the writer
        // on the same physical file.
        if !archive.is_writer && !archive.raw.is_null() {
            let raw = archive.raw as usize;
            unsafe { crate::ffi::bit7z_reader_close(raw as *mut _); }
            archive.raw = std::ptr::null_mut();
        }

        let lib = self.lock_lib()?;
        let format = detect_writer_format(&archive_path);
        let pw_str = password.map(|p| p.as_str().to_string());

        let writer = bit7z::Writer::open(&lib, &archive_path_str, format, pw_str.as_deref())
            .map_err(|e| ArchiveError::Internal(e))?;
        writer.set_update_mode(bit7z::UpdateMode::Append);

        for path in files {
            let path_str = path.to_str()
                .ok_or_else(|| ArchiveError::Internal("Non-UTF-8 path".into()))?;
            if path.is_dir() {
                writer.add_directory(path_str)
                    .map_err(|e| ArchiveError::Internal(e))?;
            } else {
                writer.add_file(path_str)
                    .map_err(|e| ArchiveError::Internal(e))?;
            }
        }

        writer.compress_to(&archive_path_str)
            .map_err(|e| ArchiveError::Internal(e))?;

        // Explicitly close the writer before re-opening the reader on the
        // same file.
        drop(writer);

        // Re-open the reader so the handle remains valid for the caller.
        let reader = bit7z::ArchiveReader::open(&lib, &archive_path_str, password)
            .map_err(|e| ArchiveError::Internal(e))?;
        let has_encrypted = reader.has_encrypted_items();
        archive.raw = reader.into_raw() as *mut std::ffi::c_void;
        archive.has_encrypted_items = has_encrypted;

        drop(lib);

        if let Ok(guard) = self.progress_sender.lock() {
            if let Some(ref tx) = *guard {
                let _ = tx.send(ProgressUpdate {
                    file_current: files.len() as u64,
                    file_total: files.len() as u64,
                    current_file: None,
                    items_done: files.len() as u64,
                    items_total: files.len() as u64,
                    bytes_done: 0,
                    bytes_total: 0,
                    error: None,
                });
            }
        }

        Ok(())
    }

    fn add_file_to_path(&self, archive: &mut ArchiveHandle, file_path: &Path, archive_path: &str, password: Option<&Password>)
                        -> Result<(), ArchiveError> {
        let archive_file_path = archive.path.clone()
            .ok_or_else(|| ArchiveError::Internal("No path in archive handle".into()))?;
        let archive_path_str = archive_file_path.to_str()
            .ok_or_else(|| ArchiveError::Internal("Non-UTF-8 path".into()))?
            .to_string();

        if !archive.is_writer && !archive.raw.is_null() {
            let raw = archive.raw as usize;
            unsafe { crate::ffi::bit7z_reader_close(raw as *mut _); }
            archive.raw = std::ptr::null_mut();
        }

        let lib = self.lock_lib()?;
        let format = detect_writer_format(&archive_file_path);
        let pw_str = password.map(|p| p.as_str().to_string());

        let writer = bit7z::Writer::open(&lib, &archive_path_str, format, pw_str.as_deref())
            .map_err(|e| ArchiveError::Internal(e))?;
        writer.set_update_mode(bit7z::UpdateMode::Append);

        let fs_path = file_path.to_str()
            .ok_or_else(|| ArchiveError::Internal("Non-UTF-8 file path".into()))?;
        writer.add_items(&[(fs_path, archive_path)])
            .map_err(|e| ArchiveError::Internal(e))?;

        writer.compress_to(&archive_path_str)
            .map_err(|e| ArchiveError::Internal(e))?;

        drop(writer);

        let reader = bit7z::ArchiveReader::open(&lib, &archive_path_str, password)
            .map_err(|e| ArchiveError::Internal(e))?;
        let has_encrypted = reader.has_encrypted_items();
        archive.raw = reader.into_raw() as *mut std::ffi::c_void;
        archive.has_encrypted_items = has_encrypted;

        drop(lib);
        Ok(())
    }

    fn delete(&self, archive: &mut ArchiveHandle, indices: &[u32])
               -> Result<(), ArchiveError> {
        let lib = self.lock_lib()?;
        let archive_path = archive.path.as_ref()
            .ok_or_else(|| ArchiveError::Internal("No path in archive handle".into()))?;
        let archive_path_str = archive_path.to_str()
            .ok_or_else(|| ArchiveError::Internal("Non-UTF-8 path".into()))?;
        let format = detect_writer_format(archive_path);

        let editor = bit7z::Editor::open(&lib, archive_path_str, format, None)
            .map_err(|e| ArchiveError::Internal(e))?;

        let mut sorted: Vec<u32> = indices.to_vec();
        sorted.sort_unstable_by(|a, b| b.cmp(a));

        let total = sorted.len() as u64;
        for (i, &index) in sorted.iter().enumerate() {
            editor.delete(index).map_err(|e| ArchiveError::Internal(e))?;
            if let Ok(guard) = self.progress_sender.lock() {
                if let Some(ref tx) = *guard {
                    let _ = tx.send(ProgressUpdate {
                        file_current: i as u64 + 1,
                        file_total: total,
                        current_file: None,
                        items_done: i as u64 + 1,
                        items_total: total,
                        bytes_done: 0,
                        bytes_total: 0,
                        error: None,
                    });
                }
            }
        }

        editor.apply().map_err(|e| ArchiveError::Internal(e))?;
        Ok(())
    }

    fn rename(&self, archive: &mut ArchiveHandle, index: u32, new_name: &str)
              -> Result<(), ArchiveError> {
        let lib = self.lock_lib()?;
        let archive_path = archive.path.as_ref()
            .ok_or_else(|| ArchiveError::Internal("No path in archive handle".into()))?;
        let archive_path_str = archive_path.to_str()
            .ok_or_else(|| ArchiveError::Internal("Non-UTF-8 path".into()))?;
        let format = detect_writer_format(archive_path);

        let p = unsafe { crate::ffi::bit7z_item_path(archive.raw as *mut _, index) };
        let current_path = if p.is_null() { String::new() }
            else { unsafe { std::ffi::CStr::from_ptr(p).to_string_lossy().into_owned() } };

        let new_path = if let Some(slash_pos) = current_path.rfind('/') {
            format!("{}/{}", &current_path[..slash_pos], new_name)
        } else {
            new_name.to_string()
        };

        let editor = bit7z::Editor::open(&lib, archive_path_str, format, None)
            .map_err(|e| ArchiveError::Internal(e))?;
        editor.rename(index, &new_path)
            .map_err(|e| ArchiveError::Internal(e))?;
        editor.apply().map_err(|e| ArchiveError::Internal(e))?;
        Ok(())
    }

    fn test(&self, archive: &ArchiveHandle) -> Result<TestResult, ArchiveError> {
        let raw = archive.raw as usize;
        let count = unsafe { crate::ffi::bit7z_reader_item_count(raw as *mut _) };
        if count == 0 {
            return Ok(TestResult { total: 0, passed: 0, failed: vec![] });
        }
        // Use the fast C++ built-in test (bit7z BitArchiveReader::test())
        let result = unsafe { crate::ffi::bit7z_reader_test(raw as *mut _) };
        if result.is_null() {
            return Err(ArchiveError::Internal("test call failed".into()));
        }
        let all_ok = unsafe { crate::ffi::bit7z_test_result_all_ok(result) } != 0;
        let total = unsafe { crate::ffi::bit7z_test_result_total(result) };
        let failed_count = unsafe { crate::ffi::bit7z_test_result_failed_count(result) };
        let passed = total.saturating_sub(failed_count);
        let mut failures = Vec::new();
        if !all_ok {
            if failed_count > 0 && total > 0 {
                for i in 0..failed_count.min(total) {
                    failures.push(TestFailure {
                        entry_path: format!("index {}", i),
                        error: "test failed".into(),
                        index: i as usize,
                        path: String::new(),
                        reason: TestFailureReason::ReadError("entry test failed".into()),
                    });
                }
            } else if failed_count > 0 {
                // C++ exception path: total=0, failed_count=1, error in failed_errors[0]
                let error_msg = unsafe {
                    let ptr = crate::ffi::bit7z_test_result_error(result);
                    if ptr.is_null() { "test failed".to_string() }
                    else { std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned() }
                };
                failures.push(TestFailure {
                    entry_path: String::new(),
                    error: error_msg.clone(),
                    index: 0,
                    path: String::new(),
                    reason: TestFailureReason::ReadError(error_msg),
                });
            }
        }
        unsafe { crate::ffi::bit7z_test_result_free(result); }
        Ok(TestResult { total: total as usize, passed: passed as usize, failed: failures })
    }

    fn list_directory(&self, archive: &ArchiveHandle, path: &str) -> Result<Vec<ArchiveEntry>, ArchiveError> {
        let c_path = std::ffi::CString::new(path)
            .map_err(|_| ArchiveError::Internal("invalid path string".into()))?;
        let list = unsafe { crate::ffi::bit7z_reader_list_directory(archive.raw as *mut _, c_path.as_ptr()) };
        if list.is_null() {
            return Err(ArchiveError::NotFound(path.into()));
        }
        let count = unsafe { crate::ffi::bit7z_item_list_count(list) };
        let mut entries = Vec::with_capacity(count as usize);
        for i in 0..count {
            let path = unsafe {
                let p = crate::ffi::bit7z_item_list_path(list, i);
                if p.is_null() { String::new() }
                else { std::ffi::CStr::from_ptr(p).to_string_lossy().into_owned() }
            };
            let name = path.rsplit('/').next().unwrap_or(&path).to_string();
            let orig_idx = unsafe { crate::ffi::bit7z_item_list_index(list, i) };
            let mut entry = ArchiveEntry {
                name,
                path,
                size: unsafe { crate::ffi::bit7z_item_list_size(list, i) },
                compressed_size: unsafe { crate::ffi::bit7z_item_list_packed_size(list, i) },
                is_directory: unsafe { crate::ffi::bit7z_item_list_is_dir(list, i) != 0 },
                is_encrypted: unsafe { crate::ffi::bit7z_item_list_is_encrypted(list, i) != 0 },
                original_index: orig_idx,
                ..Default::default()
            };
            populate_item_details(&mut entry, archive.raw, orig_idx);
            entries.push(entry);
        }
        unsafe { crate::ffi::bit7z_item_list_free(list); }
        Ok(entries)
    }

    fn close(&self, archive: ArchiveHandle) {
        let raw = archive.raw as usize;
        unsafe { crate::ffi::bit7z_reader_close(raw as *mut _); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::Arc;

    /// A mock that fails open for non-existent files and delegates to inner for others.
    struct FailOnMissingRepo {
        inner: Arc<dyn ArchiveRepository>,
    }

    impl ArchiveRepository for FailOnMissingRepo {
    fn open(&self, path: &Path, password: Option<&Password>) -> Result<ArchiveHandle, ArchiveError> {
            if !path.exists() {
                return Err(ArchiveError::NotFound(path.to_string_lossy().to_string()));
            }
            self.inner.open(path, password)
        }
        fn create(&self, path: &Path, format: ArchiveFormat, encryption: Option<&EncryptionConfig>) -> Result<ArchiveHandle, ArchiveError> {
            self.inner.create(path, format, encryption)
        }
        fn list_page(&self, archive: &ArchiveHandle, offset: usize, limit: usize) -> Result<Page<ArchiveEntry>, ArchiveError> {
            self.inner.list_page(archive, offset, limit)
        }
        fn get_properties(&self, archive: &ArchiveHandle) -> Result<ArchiveProperties, ArchiveError> {
            self.inner.get_properties(archive)
        }
        fn extract(&self, archive: &ArchiveHandle, indices: &[u32], dest: &Path) -> Result<(), ArchiveError> {
            self.inner.extract(archive, indices, dest)
        }
        fn extract_to_buffer(&self, archive: &ArchiveHandle, index: u32) -> Result<Vec<u8>, ArchiveError> {
            self.inner.extract_to_buffer(archive, index)
        }
        fn add(&self, archive: &mut ArchiveHandle, files: &[PathBuf], password: Option<&Password>) -> Result<(), ArchiveError> {
            self.inner.add(archive, files, password)
        }
        fn delete(&self, archive: &mut ArchiveHandle, indices: &[u32]) -> Result<(), ArchiveError> {
            self.inner.delete(archive, indices)
        }
        fn rename(&self, archive: &mut ArchiveHandle, index: u32, new_name: &str) -> Result<(), ArchiveError> {
            self.inner.rename(archive, index, new_name)
        }
        fn test(&self, archive: &ArchiveHandle) -> Result<TestResult, ArchiveError> {
            self.inner.test(archive)
        }
        fn close(&self, archive: ArchiveHandle) {
            self.inner.close(archive)
        }
        fn list_directory(&self, archive: &ArchiveHandle, path: &str) -> Result<Vec<ArchiveEntry>, ArchiveError> {
            self.inner.list_directory(archive, path)
        }
    }

    #[test]
    fn test_repository_open_nonexistent_file_returns_error() {
        let inner = crate::domain::repository::test_utils::MockArchiveRepository::arc_with_count(0);
        let repo = FailOnMissingRepo { inner };
        let result = repo.open(Path::new("nonexistent.7z"), None);
        assert!(matches!(result, Err(ArchiveError::NotFound(_))));
    }

    #[test]
    fn test_repository_open_existing_file_succeeds() {
        let inner = crate::domain::repository::test_utils::MockArchiveRepository::arc_with_count(10);
        let repo = FailOnMissingRepo { inner };
        // Use a path that exists (this test file)
        let result = repo.open(Path::new("Cargo.toml"), None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_repository_list_page_returns_entries() {
        use crate::domain::repository::test_utils::MockArchiveRepository;
        let repo = MockArchiveRepository::arc_with_count(10);
        let handle = repo.open(Path::new("test.7z"), None).unwrap();
        let page = repo.list_page(&handle, 0, 5).unwrap();
        assert_eq!(page.items.len(), 5);
        assert_eq!(page.total, Some(10));
    }

    #[test]
    fn test_repository_extract_to_buffer_unsupported() {
        use crate::domain::repository::test_utils::MockArchiveRepository;
        let repo = MockArchiveRepository::arc_with_count(1);
        let handle = repo.open(Path::new("test.7z"), None).unwrap();
        let result = repo.extract_to_buffer(&handle, 0);
        assert!(matches!(result, Err(ArchiveError::UnsupportedOperation)));
    }

    #[test]
    fn test_list_directory_root_returns_top_level_only() {
        use crate::domain::repository::test_utils::MockArchiveRepository;
        let mock = MockArchiveRepository::new(vec![
            ArchiveEntry { name: "a.txt".into(), path: "a.txt".into(), original_index: 0, ..default_entry() },
            ArchiveEntry { name: "dir".into(), path: "dir".into(), original_index: 1, is_directory: true, ..default_entry() },
            ArchiveEntry { name: "inner.txt".into(), path: "dir/inner.txt".into(), original_index: 2, ..default_entry() },
            ArchiveEntry { name: "b.txt".into(), path: "b.txt".into(), original_index: 3, ..default_entry() },
        ]);
        let handle = mock.open(Path::new("t.7z"), None).unwrap();
        let result = mock.list_directory(&handle, "").unwrap();
        assert_eq!(result.len(), 3); // a.txt, dir, b.txt — dir/inner.txt is nested
        assert!(result.iter().any(|e| e.name == "dir" && e.is_directory));
        assert!(result.iter().any(|e| e.name == "a.txt"));
        assert!(result.iter().any(|e| e.name == "b.txt"));
    }

    #[test]
    fn test_list_directory_subdir_returns_children() {
        use crate::domain::repository::test_utils::MockArchiveRepository;
        let mock = MockArchiveRepository::new(vec![
            ArchiveEntry { name: "inner.txt".into(), path: "dir/inner.txt".into(), original_index: 0, ..default_entry() },
            ArchiveEntry { name: "deep.txt".into(), path: "dir/sub/deep.txt".into(), original_index: 1, ..default_entry() },
        ]);
        let handle = mock.open(Path::new("t.7z"), None).unwrap();
        let result = mock.list_directory(&handle, "dir/").unwrap();
        assert_eq!(result.len(), 1); // only inner.txt — deep.txt has another level
        assert_eq!(result[0].name, "inner.txt");
    }

    #[test]
    fn test_list_directory_empty_dir_returns_empty() {
        use crate::domain::repository::test_utils::MockArchiveRepository;
        let mock = MockArchiveRepository::new(vec![
            ArchiveEntry { name: "f.txt".into(), path: "f.txt".into(), original_index: 0, ..default_entry() },
        ]);
        let handle = mock.open(Path::new("t.7z"), None).unwrap();
        let result = mock.list_directory(&handle, "other/").unwrap();
        assert_eq!(result.len(), 0);
    }

    fn default_entry() -> ArchiveEntry {
        ArchiveEntry::default()
    }
}
