use crate::adapters::bit7z;
use crate::application::plan::{ExecutionPlan, plan_changes};
use crate::application::progress::{ProgressNotifier, ProgressSender, ProgressUpdate};
use crate::domain::archive::*;
use crate::domain::repository::*;
use chrono::DateTime;
use humansize::{format_size, BINARY};
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

#[cfg(target_os = "windows")]
unsafe extern "system" {
    fn MessageBoxW(hWnd: *mut std::ffi::c_void, lpText: *const u16, lpCaption: *const u16, uType: u32) -> i32;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheState {
    Valid,
    Dirty,
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
        src_path, format_size(src_size, BINARY), format_time(src_mtime),
        dest_path, format_size(existing_size, BINARY), format_time(dest_mtime),
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

fn read_all_entries(raw: *mut std::ffi::c_void) -> Vec<ArchiveEntry> {
    let list = unsafe { crate::ffi::bit7z_reader_items(raw as *mut _) };
    if list.is_null() {
        return Vec::new();
    }
    let count = unsafe { crate::ffi::bit7z_item_list_count(list as *mut _) };
    let mut entries = Vec::with_capacity(count as usize);

    for i in 0..count {
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
    entries
}

fn execute_with_editor(
    lib: &bit7z::Library,
    path: &str,
    format: bit7z::WriterFormat,
    plan: &ExecutionPlan,
    _cancel: &Arc<AtomicBool>,
    _paused: &Arc<AtomicBool>,
) -> Result<(), ArchiveError> {
    let editor = bit7z::Editor::open(lib, path, format, None)
        .map_err(|e| ArchiveError::Internal(format!("editor open: {}", e)))?;

    let mut sorted_deletes = plan.deletes.clone();
    sorted_deletes.sort_unstable_by(|a, b| b.cmp(a));
    for &idx in &sorted_deletes {
        editor.delete(idx)
            .map_err(|e| ArchiveError::Internal(format!("delete: {}", e)))?;
    }

    for &(idx, ref new_path) in &plan.renames {
        editor.rename(idx, new_path)
            .map_err(|e| ArchiveError::Internal(format!("rename: {}", e)))?;
    }

    editor.apply()
        .map_err(|e| ArchiveError::Internal(format!("apply: {}", e)))?;

    if !plan.adds.is_empty() || !plan.updates.is_empty() {
        let writer = bit7z::Writer::open(lib, path, format, None)
            .map_err(|e| ArchiveError::Internal(format!("writer open after editor: {}", e)))?;
        writer.set_update_mode(bit7z::UpdateMode::Append);

        for (fs_path, _arc_path) in &plan.adds {
            let path_str = fs_path.to_str()
                .ok_or_else(|| ArchiveError::Internal("invalid path".into()))?;
            writer.add_file(path_str)
                .map_err(|e| ArchiveError::Internal(format!("add: {}", e)))?;
        }

        for (fs_path, _arc_path) in &plan.updates {
            let path_str = fs_path.to_str()
                .ok_or_else(|| ArchiveError::Internal("invalid path".into()))?;
            writer.add_file(path_str)
                .map_err(|e| ArchiveError::Internal(format!("update: {}", e)))?;
        }

        writer.compress_to(path)
            .map_err(|e| ArchiveError::Internal(format!("compress after editor: {}", e)))?;
    }

    Ok(())
}

fn execute_with_writer(
    lib: &bit7z::Library,
    path: &str,
    format: bit7z::WriterFormat,
    plan: &ExecutionPlan,
    _cancel: &Arc<AtomicBool>,
    _paused: &Arc<AtomicBool>,
) -> Result<(), ArchiveError> {
    let writer = bit7z::Writer::open(lib, path, format, None)
        .map_err(|e| ArchiveError::Internal(format!("writer open: {}", e)))?;

    let has_updates = !plan.updates.is_empty();
    writer.set_update_mode(if has_updates {
        bit7z::UpdateMode::Update
    } else {
        bit7z::UpdateMode::Append
    });

    for (fs_path, _arc_path) in &plan.adds {
        let path_str = fs_path.to_str()
            .ok_or_else(|| ArchiveError::Internal("invalid path".into()))?;
        writer.add_file(path_str)
            .map_err(|e| ArchiveError::Internal(format!("add: {}", e)))?;
    }

    for (fs_path, _arc_path) in &plan.updates {
        let path_str = fs_path.to_str()
            .ok_or_else(|| ArchiveError::Internal("invalid path".into()))?;
        writer.add_file(path_str)
            .map_err(|e| ArchiveError::Internal(format!("update: {}", e)))?;
    }

    writer.compress_to(path)
        .map_err(|e| ArchiveError::Internal(format!("compress: {}", e)))?;

    Ok(())
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

    // Use a single stack-allocated buffer for all C string reads
    let mut buf = [0u8; 256];

    macro_rules! read_field {
        ($ffi_fn:ident, $field:ident) => {
            let ret = unsafe { crate::ffi::$ffi_fn(item_ptr, buf.as_mut_ptr() as *mut _, 256) };
            if ret >= 0 {
                let s = unsafe { std::ffi::CStr::from_ptr(buf.as_ptr() as *const _) };
                let s = s.to_string_lossy().into_owned();
                if !s.is_empty() { entry.$field = Some(s); }
            }
        };
    }

    read_field!(bit7z_item_compression_method, compression_method);
    read_field!(bit7z_item_comment, comment);
    read_field!(bit7z_item_user, user);
    read_field!(bit7z_item_group, group);
    read_field!(bit7z_item_extension, extension);
    read_field!(bit7z_item_hardlink, hardlink);
}

struct RepositoryInner {
    lib: bit7z::Library,
    handles: HashMap<u64, bit7z::FfiHandle>,
    overwrite_mode: OverwriteMode,
    cancel: Option<Arc<AtomicBool>>,
    paused: Option<Arc<AtomicBool>>,
    progress_notifier: Option<Box<dyn ProgressNotifier>>,
    cache_state: HashMap<u64, CacheState>,
}

/// FFI-backed implementation of ArchiveRepository.
///
/// All mutable state is consolidated into a single `Mutex<RepositoryInner>`.
/// The Mutex serializes all access, ensuring thread safety since the C++ bit7z
/// library is not safe for concurrent read operations on the same handle.
pub struct Bit7zRepository {
    inner: Mutex<RepositoryInner>,
}

impl Bit7zRepository {
    pub fn new(lib: bit7z::Library) -> Self {
        Self {
            inner: Mutex::new(RepositoryInner {
                lib,
                handles: HashMap::new(),
                overwrite_mode: OverwriteMode::Ask,
                cancel: None,
                paused: None,
                progress_notifier: None,
                cache_state: HashMap::new(),
            }),
        }
    }

    pub fn extract_with_progress(
        &self,
        archive: &ArchiveHandle,
        indices: &[u32],
        dest: &Path,
        progress: ProgressSender,
    ) -> Result<(), ArchiveError> {
        let (raw_ptr, mode, cancel, paused) = {
            let guard = self.inner.lock().map_err(|e| {
                ArchiveError::Internal(format!("[extract_with_progress] lock poisoned: {}", e))
            })?;
            let raw = guard.handles.get(&archive.id)
                .map(|h| h.ptr())
                .ok_or_else(|| {
                    ArchiveError::Internal(format!(
                        "[extract_with_progress] archive handle {} not found",
                        archive.id
                    ))
                })?;
            let mode = match guard.overwrite_mode {
                OverwriteMode::Overwrite => 0i32,
                OverwriteMode::Skip => 1i32,
                OverwriteMode::RenameExtracted => 2i32,
                OverwriteMode::Ask => -1i32,
            };
            (raw, mode, guard.cancel.clone(), guard.paused.clone())
        };

        let dest_str = dest.to_str().ok_or_else(|| {
            ArchiveError::Internal(format!(
                "[extract_with_progress] destination path is not valid UTF-8: {}",
                dest.display()
            ))
        })?;

        let ctx = Box::into_raw(Box::new(ExtractCtx {
            progress,
            global_mode: AtomicI32::new(mode),
            current_file: Mutex::new(String::new()),
            current_file_size: AtomicU64::new(0),
            cancel,
            paused,
        })) as *mut std::ffi::c_void;

        let ret = unsafe {
            // SAFETY: We hold the Mutex lock on self.inner, so the FfiHandle at
            // raw_ptr is guaranteed to remain valid for the duration of this
            // call. ManuallyDrop prevents the FfiHandle's Drop from running
            // (the handle must stay alive in the HashMap).
            let reader = std::mem::ManuallyDrop::new(bit7z::ArchiveReader::from_raw(bit7z::Handle::from_raw(raw_ptr)));
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
    fn open(&self, path: &Path, password: Option<&Password>) -> Result<ArchiveHandle, ArchiveError> {
        let mut guard = self.inner.lock().map_err(|_| {
            ArchiveError::Internal("[open] lock poisoned".into())
        })?;
        let path_str = path.to_str()
            .ok_or_else(|| ArchiveError::Internal(format!("[open] path is not valid UTF-8: {}", path.display())))?;

        let is_rar = path_str.to_lowercase().ends_with(".rar");

        let (is_header_encrypted, reader) = if is_rar {
            let r = match bit7z::ArchiveReader::open(&guard.lib, path_str, password) {
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
            let enc = guard.lib.is_header_encrypted(path_str);
            if enc && password.is_none() {
                return Err(ArchiveError::EncryptedArchiveRequiresPassword);
            }
            let r = bit7z::ArchiveReader::open(&guard.lib, path_str, password)
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
        guard.handles.insert(handle.id, bit7z::FfiHandle::reader(raw.as_ptr()));
        guard.cache_state.insert(handle.id, CacheState::Dirty);
        Ok(handle)
    }

    fn create(&self, path: &Path, format: ArchiveFormat,
              encryption: Option<&EncryptionConfig>) -> Result<ArchiveHandle, ArchiveError> {
        let mut guard = self.inner.lock().map_err(|_| {
            ArchiveError::Internal("[create] lock poisoned".into())
        })?;
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

        let writer = bit7z::Writer::create(&guard.lib, writer_format)
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
        guard.handles.insert(handle.id, bit7z::FfiHandle::writer(raw.as_ptr()));
        Ok(handle)
    }

    fn list_page(&self, archive: &ArchiveHandle, offset: usize, limit: usize)
                 -> Result<Page<ArchiveEntry>, ArchiveError> {
        let guard = self.inner.lock().map_err(|_| {
            ArchiveError::Internal("[list_page] lock poisoned".into())
        })?;
        let raw = guard.handles.get(&archive.id)
            .map(|h| h.ptr())
            .ok_or_else(|| ArchiveError::Internal(format!("[list_page] handle {} not found", archive.id)))?;

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
        let guard = self.inner.lock().map_err(|_| {
            ArchiveError::Internal("[get_properties] lock poisoned".into())
        })?;
        let raw = guard.handles.get(&archive.id)
            .map(|h| h.ptr())
            .ok_or_else(|| ArchiveError::Internal(format!("[get_properties] handle {} not found", archive.id)))?;

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

    fn extract(&self, archive: &ArchiveHandle, indices: &[u32], dest: &Path, options: &ExtractOptions)
               -> Result<(), ArchiveError> {
        let mut guard = self.inner.lock().map_err(|_| {
            ArchiveError::Internal("[extract] lock poisoned".into())
        })?;
        guard.overwrite_mode = options.overwrite_mode;
        guard.cancel = Some(options.cancel.clone());
        guard.paused = Some(options.paused.clone());
        guard.progress_notifier = Some(Box::new(crate::application::progress::CrossbeamNotifier(
            crossbeam::channel::unbounded().0
        )));
        drop(guard);

        let (tx, rx) = crossbeam::channel::unbounded();
        let result = self.extract_with_progress(archive, indices, dest, tx);
        while let Ok(update) = rx.try_recv() {
            options.notifier.notify(&update);
        }
        result
    }

    fn extract_to_buffer(&self, archive: &ArchiveHandle, index: u32)
                         -> Result<Vec<u8>, ArchiveError> {
        let guard = self.inner.lock().map_err(|_| {
            ArchiveError::Internal("[extract_to_buffer] lock poisoned".into())
        })?;
        let raw = guard.handles.get(&archive.id)
            .map(|h| h.ptr())
            .ok_or_else(|| ArchiveError::Internal(format!("[extract_to_buffer] handle {} not found", archive.id)))?;
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

    fn plan_changes(&self, archive: &ArchiveHandle, change_set: &ChangeSet) -> Result<ExecutionPlan, ArchiveError> {
        let guard = self.inner.lock().map_err(|_| {
            ArchiveError::Internal("[plan_changes] lock poisoned".into())
        })?;
        let raw = guard.handles.get(&archive.id)
            .map(|h| h.ptr())
            .ok_or_else(|| ArchiveError::Internal(format!("[plan_changes] handle {} not found", archive.id)))?;

        let snapshot = read_all_entries(raw);
        Ok(plan_changes(&snapshot, change_set))
    }

    fn apply_changes(&self, archive: &ArchiveHandle, plan: &ExecutionPlan, options: &WriteOptions) -> Result<(), ArchiveError> {
        if !plan.has_writes() {
            return Ok(());
        }

        let mut guard = self.inner.lock().map_err(|_| {
            ArchiveError::Internal("[apply_changes] lock poisoned".into())
        })?;

        guard.cache_state.insert(archive.id, CacheState::Dirty);
        guard.cancel = Some(options.cancel.clone());
        guard.paused = Some(options.paused.clone());

        let archive_path = archive.path.clone()
            .ok_or_else(|| ArchiveError::Internal(format!("[apply_changes] no path in archive handle {}", archive.id)))?;
        let archive_path_str = archive_path.to_str()
            .ok_or_else(|| ArchiveError::Internal(format!("[apply_changes] path is not valid UTF-8: {}", archive_path.display())))?
            .to_string();

        guard.handles.remove(&archive.id);

        let format = detect_writer_format(&archive_path);
        let needs_editor = !plan.deletes.is_empty() || !plan.renames.is_empty();

        let result = if needs_editor {
            execute_with_editor(&guard.lib, &archive_path_str, format, plan, &options.cancel, &options.paused)
        } else {
            execute_with_writer(&guard.lib, &archive_path_str, format, plan, &options.cancel, &options.paused)
        };

        match bit7z::ArchiveReader::open(&guard.lib, &archive_path_str, None) {
            Ok(reader) => {
                guard.handles.insert(archive.id, bit7z::FfiHandle::reader(reader.into_raw().as_ptr()));
            }
            Err(_) => {
                guard.handles.remove(&archive.id);
            }
        }

        result
    }

    fn test(&self, archive: &ArchiveHandle) -> Result<TestResult, ArchiveError> {
        let guard = self.inner.lock().map_err(|_| {
            ArchiveError::Internal("[test] lock poisoned".into())
        })?;
        let raw = guard.handles.get(&archive.id)
            .map(|h| h.ptr())
            .ok_or_else(|| ArchiveError::Internal(format!("[test] handle {} not found", archive.id)))?;
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
        let guard = self.inner.lock().map_err(|_| {
            ArchiveError::Internal("[list_directory] lock poisoned".into())
        })?;
        let raw = guard.handles.get(&archive.id)
            .map(|h| h.ptr())
            .ok_or_else(|| ArchiveError::Internal(format!("[list_directory] handle {} not found", archive.id)))?;
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

    fn close(&self, archive: &ArchiveHandle) {
        if let Ok(mut guard) = self.inner.lock() {
            guard.handles.remove(&archive.id);
            guard.cache_state.remove(&archive.id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

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
        fn extract(&self, archive: &ArchiveHandle, indices: &[u32], dest: &Path, options: &ExtractOptions) -> Result<(), ArchiveError> {
            self.inner.extract(archive, indices, dest, options)
        }
        fn extract_to_buffer(&self, archive: &ArchiveHandle, index: u32) -> Result<Vec<u8>, ArchiveError> {
            self.inner.extract_to_buffer(archive, index)
        }
        fn plan_changes(&self, archive: &ArchiveHandle, change_set: &ChangeSet) -> Result<ExecutionPlan, ArchiveError> {
            self.inner.plan_changes(archive, change_set)
        }
        fn apply_changes(&self, archive: &ArchiveHandle, plan: &ExecutionPlan, options: &WriteOptions) -> Result<(), ArchiveError> {
            self.inner.apply_changes(archive, plan, options)
        }
        fn test(&self, archive: &ArchiveHandle) -> Result<TestResult, ArchiveError> {
            self.inner.test(archive)
        }
        fn close(&self, archive: &ArchiveHandle) {
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
