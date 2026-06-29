use crate::adapters::bit7z;
use crate::application::progress::{ProgressNotifier, ProgressSender, ProgressUpdate};
use crate::domain::archive::*;
use crate::domain::repository::*;
use chrono::DateTime;
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

#[cfg(target_os = "windows")]
extern "system" {
    fn MessageBoxW(hWnd: *mut std::ffi::c_void, lpText: *const u16, lpCaption: *const u16, uType: u32) -> i32;
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

fn format_time(timestamp: i64) -> String {
    let secs = if timestamp >= 0 { timestamp as u64 } else { 0 };
    let naive = DateTime::from_timestamp(secs as i64, 0).map(|dt| dt.naive_local()).unwrap_or_default();
    naive.format("%Y-%m-%d %H:%M:%S").to_string()
}

#[cfg(not(target_os = "windows"))]
fn show_overwrite_dialog(
    _src_path: &str, _dest_path: &str,
    _existing_size: u64, _src_size: u64,
    _src_mtime: i64, _dest_mtime: i64,
    _global_mode: &AtomicI32,
) -> i32 { 0 }

#[cfg(target_os = "windows")]
fn show_overwrite_dialog(
    src_path: &str, dest_path: &str,
    existing_size: u64, src_size: u64,
    src_mtime: i64, dest_mtime: i64,
    global_mode: &AtomicI32,
) -> i32 {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    let msg = format!(
        "File exists:\n\
         Source: {}\n  Size: {}     Modified: {}\n\
         Dest:   {}\n  Size: {}     Modified: {}\n\n\
         Yes = Overwrite    No = Skip    Cancel = Overwrite",
        src_path, format_bytes(src_size), format_time(src_mtime),
        dest_path, format_bytes(existing_size), format_time(dest_mtime),
    );
    let title = "Confirm Overwrite";
    let msg_wide: Vec<u16> = OsStr::new(&msg).encode_wide().chain(std::iter::once(0)).collect();
    let title_wide: Vec<u16> = OsStr::new(title).encode_wide().chain(std::iter::once(0)).collect();
    let choice = unsafe { MessageBoxW(std::ptr::null_mut(), msg_wide.as_ptr(), title_wide.as_ptr(), 3 | 32) };
    let action = match choice {
        6 => 0,
        7 => 1,
        _ => 0,
    };

    let apply_all_msg = match action {
        0 => "Always overwrite remaining files?",
        _ => "Always skip remaining files?",
    };
    let all_wide: Vec<u16> = OsStr::new(apply_all_msg).encode_wide().chain(std::iter::once(0)).collect();
    let all_title_wide: Vec<u16> = OsStr::new("Apply to All").encode_wide().chain(std::iter::once(0)).collect();
    let apply_all = unsafe { MessageBoxW(std::ptr::null_mut(), all_wide.as_ptr(), all_title_wide.as_ptr(), 4 | 32) };
    if apply_all == 6 {
        global_mode.store(action, Ordering::Relaxed);
    }

    action
}

struct ExtractCtx {
    progress: ProgressSender,
    global_mode: AtomicI32,
    current_file: Mutex<String>,
    current_file_size: AtomicU64,
    cancel: Option<Arc<AtomicBool>>,
    paused: Option<Arc<AtomicBool>>,
}

extern "C" fn extract_progress_callback(
    processed: u64,
    total: u64,
    ctx: *mut std::ffi::c_void,
) -> i32 {
    let ctx = unsafe { &*(ctx as *const ExtractCtx) };

    // Check cancel
    if let Some(ref cancel) = ctx.cancel {
        if cancel.load(Ordering::Relaxed) {
            return 0;
        }
    }

    // Handle pause: spin until unpaused or cancelled
    if let Some(ref paused) = ctx.paused {
        while paused.load(Ordering::Relaxed) {
            if let Some(ref cancel) = ctx.cancel {
                if cancel.load(Ordering::Relaxed) {
                    return 0;
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }

    let file = ctx.current_file.lock().ok().map(|g| g.clone()).unwrap_or_default();
    let file = if file.is_empty() { None } else { Some(file) };
    let file_total = ctx.current_file_size.load(Ordering::Relaxed);
    let _ = ctx.progress.send(ProgressUpdate {
        file_current: processed,
        file_total,
        current_file: file,
        items_done: 0, items_total: 0,
        bytes_done: processed, bytes_total: total,
        error: None,
    });
    1
}

extern "C" fn extract_file_callback(
    path: *const std::ffi::c_char,
    file_size: u64,
    ctx: *mut std::ffi::c_void,
) {
    let ctx = unsafe { &*(ctx as *const ExtractCtx) };
    let name = unsafe { std::ffi::CStr::from_ptr(path) }
        .to_string_lossy().into_owned();
    if let Ok(mut guard) = ctx.current_file.lock() {
        *guard = name;
    }
    ctx.current_file_size.store(file_size, Ordering::Relaxed);
}

extern "C" fn extract_overwrite_callback(
    src: *const std::ffi::c_char,
    dest: *const std::ffi::c_char,
    existing_size: u64,
    src_size: u64,
    src_mtime: i64,
    dest_mtime: i64,
    ctx: *mut std::ffi::c_void,
) -> i32 {
    let ctx = unsafe { &*(ctx as *const ExtractCtx) };
    let mode = ctx.global_mode.load(Ordering::Relaxed);
    match mode {
        0 => 0,
        1 => 1,
        2 => 0,
        _ => {
            let src_path = unsafe { std::ffi::CStr::from_ptr(src) }
                .to_string_lossy().into_owned();
            let dest_path = unsafe { std::ffi::CStr::from_ptr(dest) }
                .to_string_lossy().into_owned();
            show_overwrite_dialog(&src_path, &dest_path, existing_size, src_size, src_mtime, dest_mtime, &ctx.global_mode)
        }
    }
}

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
fn populate_item_details(entry: &mut ArchiveEntry, list: *mut std::ffi::c_void, index: u32) {
    entry.crc = Some(unsafe { crate::ffi::bit7z_item_list_crc(list as *mut _, index) });

    let item_ptr = unsafe { crate::ffi::bit7z_item_list_item(list as *mut _, index) };
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

    buf.fill(0);
    let ret = unsafe { crate::ffi::bit7z_item_extension(item_ptr, buf.as_mut_ptr() as *mut _, 256) };
    if ret >= 0 {
        let s = unsafe { std::ffi::CStr::from_ptr(buf.as_ptr() as *const _) };
        let s = s.to_string_lossy().into_owned();
        if !s.is_empty() { entry.extension = Some(s); }
    }

    buf.fill(0);
    let ret = unsafe { crate::ffi::bit7z_item_hardlink(item_ptr, buf.as_mut_ptr() as *mut _, 256) };
    if ret >= 0 {
        let s = unsafe { std::ffi::CStr::from_ptr(buf.as_ptr() as *const _) };
        let s = s.to_string_lossy().into_owned();
        if !s.is_empty() { entry.hardlink = Some(s); }
    }
}

/// FFI-backed implementation of ArchiveRepository.
/// Raw C++ pointers are stored in an internal map, keyed by ArchiveHandle.id.
pub struct Bit7zRepository {
    lib: Mutex<bit7z::Library>,
    handles: Mutex<HashMap<u64, *mut std::ffi::c_void>>,
    progress_notifier: Mutex<Option<Box<dyn ProgressNotifier>>>,
    overwrite_mode: Mutex<OverwriteMode>,
    cancel: Mutex<Option<Arc<AtomicBool>>>,
    paused: Mutex<Option<Arc<AtomicBool>>>,
}

unsafe impl Send for Bit7zRepository {}
unsafe impl Sync for Bit7zRepository {}

impl Bit7zRepository {
    pub fn new(lib: bit7z::Library) -> Self {
        Self { lib: Mutex::new(lib), handles: Mutex::new(HashMap::new()), progress_notifier: Mutex::new(None), overwrite_mode: Mutex::new(OverwriteMode::Ask), cancel: Mutex::new(None), paused: Mutex::new(None) }
    }

    fn lock_lib(&self) -> Result<std::sync::MutexGuard<'_, bit7z::Library>, ArchiveError> {
        self.lib.lock().map_err(|_| ArchiveError::Internal("[lock_lib] bit7z library mutex poisoned - a prior operation panicked while holding the lock".into()))
    }

    fn get_raw(&self, archive: &ArchiveHandle) -> Result<usize, ArchiveError> {
        self.handles.lock().map_err(|_| ArchiveError::Internal("[get_raw] handles mutex poisoned - a prior operation panicked while holding the lock".into()))?
            .get(&archive.id)
            .copied()
            .ok_or_else(|| ArchiveError::Internal(format!("[get_raw] archive handle {} not found in handles map (stale or double-closed)", archive.id)))
            .map(|p| p as usize)
    }

    fn remove_raw(&self, id: u64) -> Option<*mut std::ffi::c_void> {
        self.handles.lock().ok()?.remove(&id)
    }

    fn insert_raw(&self, id: u64, raw: *mut std::ffi::c_void) {
        if let Ok(mut guard) = self.handles.lock() {
            guard.insert(id, raw);
        }
    }

    fn send_progress(&self, update: ProgressUpdate) {
        if let Ok(guard) = self.progress_notifier.lock() {
            if let Some(ref notifier) = *guard {
                notifier.notify(&update);
            }
        }
    }

    pub fn extract_with_progress(
        &self,
        archive: &ArchiveHandle,
        indices: &[u32],
        dest: &Path,
        progress: ProgressSender,
    ) -> Result<(), ArchiveError> {
        let raw = self.get_raw(archive)?;
        let dest_str = dest.to_str().ok_or_else(|| {
            ArchiveError::Internal(format!("[extract_with_progress] destination path is not valid UTF-8: {}", dest.display()))
        })?;

        let mode = match self.overwrite_mode.lock() {
            Ok(guard) => match *guard {
                OverwriteMode::Overwrite => 0i32,
                OverwriteMode::Skip => 1i32,
                OverwriteMode::RenameExtracted => 2i32,
                OverwriteMode::Ask => -1i32,
            },
            Err(_) => -1i32,
        };
        let cancel = self.cancel.lock().ok().and_then(|g| g.clone());
        let paused = self.paused.lock().ok().and_then(|g| g.clone());
        let ctx = Box::into_raw(Box::new(ExtractCtx { progress, global_mode: AtomicI32::new(mode), current_file: Mutex::new(String::new()), current_file_size: AtomicU64::new(0), cancel, paused })) as *mut std::ffi::c_void;

        let ret = unsafe {
            let reader = std::mem::ManuallyDrop::new(bit7z::ArchiveReader::from_raw(raw));
            reader.extract_to_cb(
                indices,
                dest_str,
                ctx,
                Some(extract_overwrite_callback),
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

impl ArchiveRepository for Bit7zRepository {
    fn set_progress_notifier(&self, notifier: Box<dyn ProgressNotifier>) {
        if let Ok(mut guard) = self.progress_notifier.lock() {
            *guard = Some(notifier);
        }
    }

    fn set_overwrite_mode(&self, mode: OverwriteMode) {
        if let Ok(mut guard) = self.overwrite_mode.lock() {
            *guard = mode;
        }
    }

    fn set_cancel_flag(&self, cancel: Arc<AtomicBool>) {
        if let Ok(mut guard) = self.cancel.lock() {
            *guard = Some(cancel);
        }
    }

    fn set_pause_flag(&self, paused: Arc<AtomicBool>) {
        if let Ok(mut guard) = self.paused.lock() {
            *guard = Some(paused);
        }
    }

    fn open(&self, path: &Path, password: Option<&Password>) -> Result<ArchiveHandle, ArchiveError> {
        let lib = self.lock_lib()?;
        let path_str = path.to_str()
            .ok_or_else(|| ArchiveError::Internal(format!("[open] path is not valid UTF-8: {}", path.display())))?;

        let is_rar = path_str.to_lowercase().ends_with(".rar");

        let (is_header_encrypted, reader) = if is_rar {
            // RAR: skip is_header_encrypted (static check creates a temp reader
            // without a password, which can hang on encrypted RARs).
            let r = match bit7z::ArchiveReader::open(&lib, path_str, password) {
                Ok(r) => r,
                Err(_) if password.is_none() => {
                    return Err(ArchiveError::EncryptedArchiveRequiresPassword);
                }
                Err(e) => return Err(ArchiveError::Internal(
                    format!("[open] failed to open RAR '{}': {}", path_str, e),
                )),
            };
            (false, r)
        } else {
            let enc = lib.is_header_encrypted(path_str);
            if enc && password.is_none() {
                return Err(ArchiveError::EncryptedArchiveRequiresPassword);
            }
            let r = bit7z::ArchiveReader::open(&lib, path_str, password)
                .map_err(|e| ArchiveError::Internal(
                    format!("[open] failed to open archive '{}': {}", path_str, e),
                ))?;
            (enc, r)
        };

        let has_encrypted_items = if is_header_encrypted {
            true
        } else {
            reader.has_encrypted_items()
        };

        let raw = reader.into_raw();
        let mut handle = ArchiveHandle::new_reader()
            .with_path(path.to_path_buf());
        handle.set_encryption_info(is_header_encrypted, has_encrypted_items);
        self.insert_raw(handle.id, raw as *mut std::ffi::c_void);
        Ok(handle)
    }

    fn create(&self, path: &Path, format: ArchiveFormat,
              encryption: Option<&EncryptionConfig>) -> Result<ArchiveHandle, ArchiveError> {
        let lib = self.lock_lib()?;
        let _path_str = path.to_str()
            .ok_or_else(|| ArchiveError::Internal(format!("[create] path is not valid UTF-8: {}", path.display())))?;

        let writer_format = match format {
            ArchiveFormat::SevenZip => bit7z::WriterFormat::SevenZip,
            ArchiveFormat::Zip => bit7z::WriterFormat::Zip,
            ArchiveFormat::Tar => bit7z::WriterFormat::Tar,
            ArchiveFormat::TarGz => bit7z::WriterFormat::GZip,
            ArchiveFormat::TarBz2 => bit7z::WriterFormat::BZip2,
            ArchiveFormat::TarXz => bit7z::WriterFormat::Xz,
            ArchiveFormat::Rar => return Err(ArchiveError::UnsupportedOperation),
        };

        let writer = bit7z::Writer::create(&lib, writer_format)
            .map_err(|e| ArchiveError::Internal(format!("[create] failed to create writer for format {:?}: {}", writer_format, e)))?;

        if let Some(enc) = encryption {
            if !enc.password.is_empty() {
                writer.set_password(enc.password.as_str());
            }
        }

        let raw = writer.into_raw();
        let handle = ArchiveHandle::new_writer()
            .with_path(path.to_path_buf())
            .with_format(format);
        self.insert_raw(handle.id, raw as *mut std::ffi::c_void);
        Ok(handle)
    }

    fn list_page(&self, archive: &ArchiveHandle, offset: usize, limit: usize)
                 -> Result<Page<ArchiveEntry>, ArchiveError> {
        let raw = self.get_raw(archive)?;

        let list = unsafe { crate::ffi::bit7z_reader_items(raw as *mut _) };
        if list.is_null() {
            return Err(ArchiveError::Internal(format!("[list_page] bit7z_reader_items returned null for archive id {}", archive.id)));
        }
        let count = unsafe { crate::ffi::bit7z_item_list_count(list as *mut _) };

        let mut entries = Vec::new();
        let start = (offset as u32).min(count);
        let end = (start + limit as u32).min(count);

        for i in start..end {
            use std::ffi::CStr;
            let p = unsafe { crate::ffi::bit7z_item_list_path(list as *mut _, i) };
            let path_s = if p.is_null() { String::new() }
                         else { unsafe { CStr::from_ptr(p).to_string_lossy().into_owned() } };
            let name_s = path_s.rsplit('/').next().unwrap_or(&path_s).to_string();
            let size = unsafe { crate::ffi::bit7z_item_list_size(list as *mut _, i) };
            let csize = unsafe { crate::ffi::bit7z_item_list_packed_size(list as *mut _, i) };
            let is_dir = unsafe { crate::ffi::bit7z_item_list_is_dir(list as *mut _, i) != 0 };
            let is_enc = unsafe { crate::ffi::bit7z_item_list_is_encrypted(list as *mut _, i) != 0 };

            let mut entry = ArchiveEntry {
                name: name_s, path: path_s,
                size, compressed_size: csize,
                is_directory: is_dir, is_encrypted: is_enc,
                original_index: i,
                ..Default::default()
            };
            populate_item_details(&mut entry, list as *mut std::ffi::c_void, i);
            entries.push(entry);
        }

        unsafe { crate::ffi::bit7z_item_list_free(list as *mut _); }
        Ok(Page::new(entries, offset, Some(count as usize)))
    }

    fn get_properties(&self, archive: &ArchiveHandle) -> Result<ArchiveProperties, ArchiveError> {
        let raw = self.get_raw(archive)?;

        let list = unsafe { crate::ffi::bit7z_reader_items(raw as *mut _) };
        if list.is_null() {
            return Err(ArchiveError::Internal(format!("[get_properties] bit7z_reader_items returned null for archive id {}", archive.id)));
        }
        let count = unsafe { crate::ffi::bit7z_item_list_count(list as *mut _) };

        let mut folders = 0u32;
        let mut files = 0u32;
        let mut total_size = 0u64;
        let mut packed_size = 0u64;
        for i in 0..count {
            let is_dir = unsafe { crate::ffi::bit7z_item_list_is_dir(list as *mut _, i) != 0 };
            if is_dir { folders += 1; } else { files += 1; }
            total_size += unsafe { crate::ffi::bit7z_item_list_size(list as *mut _, i) };
            packed_size += unsafe { crate::ffi::bit7z_item_list_packed_size(list as *mut _, i) };
        }

        unsafe { crate::ffi::bit7z_item_list_free(list as *mut _); }

        let is_solid = unsafe { crate::ffi::bit7z_reader_is_solid(raw as *mut _) != 0 };
        let is_multi_volume = unsafe { crate::ffi::bit7z_reader_is_multi_volume(raw as *mut _) != 0 };
        let volumes_count = unsafe { crate::ffi::bit7z_reader_volumes_count(raw as *mut _) };
        let headers_size = unsafe { crate::ffi::bit7z_reader_headers_size(raw as *mut _) };
        let has_comment = unsafe { crate::ffi::bit7z_reader_has_comment(raw as *mut _) != 0 };
        let dictionary_size = {
            let sz = unsafe { crate::ffi::bit7z_reader_dictionary_size(raw as *mut _) };
            if sz > 0 { Some(sz) } else { None }
        };

        Ok(ArchiveProperties {
            items_count: count,
            folders_count: folders,
            files_count: files,
            total_size, packed_size,
            is_encrypted: archive.is_header_encrypted(),
            has_encrypted_items: archive.has_encrypted_items(),
            is_solid,
            is_multi_volume,
            volumes_count,
            headers_size,
            has_comment,
            dictionary_size,
            encrypted_names: archive.is_header_encrypted(),
            ..Default::default()
        })
    }

    fn extract(&self, archive: &ArchiveHandle, indices: &[u32], dest: &Path)
               -> Result<(), ArchiveError> {
        let raw = self.get_raw(archive)?;
        let dest_str = dest.to_str().ok_or_else(|| {
            ArchiveError::Internal(format!("[extract] destination path is not valid UTF-8: {}", dest.display()))
        })?;
        let c_dest = std::ffi::CString::new(dest_str).map_err(|e| ArchiveError::Internal(format!("[extract] CString conversion failed for '{}': {}", dest.display(), e)))?;
        let ret: i32 = unsafe {
            crate::ffi::bit7z_reader_extract_to(
                raw as *mut _, indices.as_ptr(), indices.len() as u32, c_dest.as_ptr(),
            )
        };
        if ret != 0 { Err(ArchiveError::Internal(format!("[extract] bit7z_reader_extract_to returned {} for archive id {}", ret, archive.id))) }
        else { Ok(()) }
    }

    fn extract_with_progress(
        &self,
        archive: &ArchiveHandle,
        indices: &[u32],
        dest: &Path,
        progress: ProgressSender,
    ) -> Result<(), ArchiveError> {
        self.extract_with_progress(archive, indices, dest, progress)
    }

    fn extract_to_buffer(&self, archive: &ArchiveHandle, index: u32)
                         -> Result<Vec<u8>, ArchiveError> {
        let raw = self.get_raw(archive)?;
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
            return Err(ArchiveError::Internal(format!(
                "[extract_to_buffer] bit7z_reader_extract_to_buffer_c failed for archive id {} index {}: ret={} out_data.null={} out_size={}",
                archive.id, index, ret, out_data.is_null(), out_size
            )));
        }
        let slice = unsafe { std::slice::from_raw_parts(out_data as *const u8, out_size as usize) };
        let result = slice.to_vec();
        unsafe { crate::ffi::bit7z_reader_free_buffer(out_data as *mut autocxx::c_void); }
        Ok(result)
    }

    fn add(&self, archive: &mut ArchiveHandle, files: &[std::path::PathBuf], password: Option<&Password>)
           -> Result<(), ArchiveError> {
        let archive_path = archive.path.clone()
            .ok_or_else(|| ArchiveError::Internal(format!("[add] no path in archive handle {}", archive.id)))?;
        let archive_path_str = archive_path.to_str()
            .ok_or_else(|| ArchiveError::Internal(format!("[add] archive path is not valid UTF-8: {}", archive_path.display())))?
            .to_string();

        // Close the reader to release file locks before opening the writer.
        if !archive.is_writer {
            if let Some(raw) = self.remove_raw(archive.id) {
                unsafe { crate::ffi::bit7z_reader_close(raw as *mut _); }
            }
        }

        let lib = self.lock_lib()?;
        let format = detect_writer_format(&archive_path);
        let pw_str = password.map(|p| p.as_str().to_string());

        let writer = bit7z::Writer::open(&lib, &archive_path_str, format, pw_str.as_deref())
            .map_err(|e| ArchiveError::Internal(format!("[add] failed to open writer for '{}': {}", archive_path_str, e)))?;
        writer.set_update_mode(bit7z::UpdateMode::Append);

        for path in files {
            let path_str = path.to_str()
                .ok_or_else(|| ArchiveError::Internal(format!("[add] file path is not valid UTF-8: {}", path.display())))?;
            if path.is_dir() {
                writer.add_directory(path_str)
                    .map_err(|e| ArchiveError::Internal(format!("[add] failed to add directory '{}': {}", path_str, e)))?;
            } else {
                writer.add_file(path_str)
                    .map_err(|e| ArchiveError::Internal(format!("[add] failed to add file '{}': {}", path_str, e)))?;
            }
        }

        writer.compress_to(&archive_path_str)
            .map_err(|e| ArchiveError::Internal(format!("[add] compress_to failed for '{}': {}", archive_path_str, e)))?;

        drop(writer);

        // Re-open the reader so the handle remains valid.
        let reader = bit7z::ArchiveReader::open(&lib, &archive_path_str, password)
            .map_err(|e| ArchiveError::Internal(format!("[add] failed to re-open reader after adding files to '{}': {}", archive_path_str, e)))?;
        let has_encrypted = reader.has_encrypted_items();
        archive.has_encrypted_items = has_encrypted;
        self.insert_raw(archive.id, reader.into_raw() as *mut std::ffi::c_void);
        drop(lib);

        self.send_progress(ProgressUpdate {
            file_current: files.len() as u64,
            file_total: files.len() as u64,
            current_file: None,
            items_done: files.len() as u64,
            items_total: files.len() as u64,
            bytes_done: 0,
            bytes_total: 0,
            error: None,
        });

        Ok(())
    }

    fn add_file_to_path(&self, archive: &mut ArchiveHandle, file_path: &Path, archive_path: &str, password: Option<&Password>)
                        -> Result<(), ArchiveError> {
        let archive_file_path = archive.path.clone()
            .ok_or_else(|| ArchiveError::Internal(format!("[add_file_to_path] no path in archive handle {}", archive.id)))?;
        let archive_path_str = archive_file_path.to_str()
            .ok_or_else(|| ArchiveError::Internal(format!("[add_file_to_path] archive path is not valid UTF-8: {}", archive_file_path.display())))?
            .to_string();

        if !archive.is_writer {
            if let Some(raw) = self.remove_raw(archive.id) {
                unsafe { crate::ffi::bit7z_reader_close(raw as *mut _); }
            }
        }

        let lib = self.lock_lib()?;
        let format = detect_writer_format(&archive_file_path);
        let pw_str = password.map(|p| p.as_str().to_string());

        let writer = bit7z::Writer::open(&lib, &archive_path_str, format, pw_str.as_deref())
            .map_err(|e| ArchiveError::Internal(format!("[add_file_to_path] failed to open writer for '{}': {}", archive_path_str, e)))?;
        writer.set_update_mode(bit7z::UpdateMode::Append);

        let fs_path = file_path.to_str()
            .ok_or_else(|| ArchiveError::Internal(format!("[add_file_to_path] file path is not valid UTF-8: {}", file_path.display())))?;
        writer.add_items(&[(fs_path, archive_path)])
            .map_err(|e| ArchiveError::Internal(format!("[add_file_to_path] failed to add items to '{}': {}", archive_path_str, e)))?;

        writer.compress_to(&archive_path_str)
            .map_err(|e| ArchiveError::Internal(format!("[add_file_to_path] compress_to failed for '{}': {}", archive_path_str, e)))?;

        drop(writer);

        let reader = bit7z::ArchiveReader::open(&lib, &archive_path_str, password)
            .map_err(|e| ArchiveError::Internal(format!("[add_file_to_path] failed to re-open reader for '{}': {}", archive_path_str, e)))?;
        let has_encrypted = reader.has_encrypted_items();
        archive.has_encrypted_items = has_encrypted;
        self.insert_raw(archive.id, reader.into_raw() as *mut std::ffi::c_void);
        drop(lib);
        Ok(())
    }

    fn delete(&self, archive: &mut ArchiveHandle, indices: &[u32])
               -> Result<(), ArchiveError> {
        let lib = self.lock_lib()?;
        let archive_path = archive.path.as_ref()
            .ok_or_else(|| ArchiveError::Internal(format!("[delete] no path in archive handle {}", archive.id)))?;
        let archive_path_str = archive_path.to_str()
            .ok_or_else(|| ArchiveError::Internal(format!("[delete] archive path is not valid UTF-8: {}", archive_path.display())))?;
        let format = detect_writer_format(archive_path);

        let editor = bit7z::Editor::open(&lib, archive_path_str, format, None)
            .map_err(|e| ArchiveError::Internal(format!("[delete] failed to open editor for '{}': {}", archive_path_str, e)))?;

        let mut sorted: Vec<u32> = indices.to_vec();
        sorted.sort_unstable_by(|a, b| b.cmp(a));

        let total = sorted.len() as u64;
        for (i, &index) in sorted.iter().enumerate() {
            editor.delete(index).map_err(|e| ArchiveError::Internal(format!("[delete] failed to delete index {} from '{}': {}", index, archive_path_str, e)))?;
            self.send_progress(ProgressUpdate {
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

        editor.apply().map_err(|e| ArchiveError::Internal(format!("[delete] editor.apply() failed for '{}': {}", archive_path_str, e)))?;
        Ok(())
    }

    fn rename(&self, archive: &mut ArchiveHandle, index: u32, new_name: &str)
              -> Result<(), ArchiveError> {
        let raw = self.get_raw(archive)?;
        let lib = self.lock_lib()?;
        let archive_path = archive.path.as_ref()
            .ok_or_else(|| ArchiveError::Internal(format!("[rename] no path in archive handle {}", archive.id)))?;
        let archive_path_str = archive_path.to_str()
            .ok_or_else(|| ArchiveError::Internal(format!("[rename] archive path is not valid UTF-8: {}", archive_path.display())))?;
        let format = detect_writer_format(archive_path);

        let p = unsafe { crate::ffi::bit7z_item_path(raw as *mut _, index) };
        let current_path = if p.is_null() { String::new() }
            else { unsafe { std::ffi::CStr::from_ptr(p).to_string_lossy().into_owned() } };

        let new_path = if let Some(slash_pos) = current_path.rfind('/') {
            format!("{}/{}", &current_path[..slash_pos], new_name)
        } else {
            new_name.to_string()
        };

        let editor = bit7z::Editor::open(&lib, archive_path_str, format, None)
            .map_err(|e| ArchiveError::Internal(format!("[rename] failed to open editor for '{}': {}", archive_path_str, e)))?;
        editor.rename(index, &new_path)
            .map_err(|e| ArchiveError::Internal(format!("[rename] failed to rename index {} to '{}' in '{}': {}", index, new_path, archive_path_str, e)))?;
        editor.apply().map_err(|e| ArchiveError::Internal(format!("[rename] editor.apply() failed for '{}': {}", archive_path_str, e)))?;
        Ok(())
    }

    fn test(&self, archive: &ArchiveHandle) -> Result<TestResult, ArchiveError> {
        let raw = self.get_raw(archive)?;
        let count = unsafe { crate::ffi::bit7z_reader_item_count(raw as *mut _) };
        if count == 0 {
            return Ok(TestResult { total: 0, passed: 0, failed: vec![] });
        }

        let result = unsafe { crate::ffi::bit7z_reader_test(raw as *mut _) };
        if result.is_null() {
            return Err(ArchiveError::Internal(format!(
                "[test] bit7z_reader_test returned null for archive id {}",
                archive.id
            )));
        }

        let all_ok = unsafe { crate::ffi::bit7z_test_result_all_ok(result) } != 0;
        let total = unsafe { crate::ffi::bit7z_test_result_total(result) };
        let failed_count = unsafe { crate::ffi::bit7z_test_result_failed_count(result) };

        let mut failed_errors = Vec::new();
        if !all_ok && failed_count > 0 {
            let error_msg = unsafe {
                let ptr = crate::ffi::bit7z_test_result_error(result);
                if ptr.is_null() {
                    "test failed".to_string()
                } else {
                    std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned()
                }
            };
            failed_errors.push(error_msg);
        }

        unsafe { crate::ffi::bit7z_test_result_free(result); }

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
                if failed_count > total {
                    for i in total..failed_count {
                        let error = failed_errors
                            .first()
                            .cloned()
                            .unwrap_or_else(|| "test failed".into());
                        failures.push(TestFailure {
                            entry_path: format!("index {} (extra)", i),
                            error: error.clone(),
                            index: i as usize,
                            path: String::new(),
                            reason: TestFailureReason::ReadError(error),
                        });
                    }
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

    fn list_directory(&self, archive: &ArchiveHandle, path: &str) -> Result<Vec<ArchiveEntry>, ArchiveError> {
        let raw = self.get_raw(archive)?;
        let c_path = std::ffi::CString::new(path)
            .map_err(|_e| ArchiveError::Internal(format!("[list_directory] path contains null byte: '{}'", path)))?;
        let list = unsafe { crate::ffi::bit7z_reader_list_directory(raw as *mut _, c_path.as_ptr()) };
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
            populate_item_details(&mut entry, list as *mut std::ffi::c_void, i);
            entries.push(entry);
        }
        unsafe { crate::ffi::bit7z_item_list_free(list); }
        Ok(entries)
    }

    fn close(&self, archive: ArchiveHandle) {
        if let Some(raw) = self.remove_raw(archive.id) {
            unsafe { crate::ffi::bit7z_reader_close(raw as *mut _); }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
use std::sync::{Arc, Mutex};

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
        assert_eq!(result.len(), 3);
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
        assert_eq!(result.len(), 1);
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
