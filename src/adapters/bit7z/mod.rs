//! Safe Rust wrappers around the C-style FFI functions for bit7z.

pub mod worker;

use crate::domain::archive::Password;
use std::ffi::CStr;
use std::ptr;
use autocxx::c_int;
use autocxx::c_void;

/// Store opaque C++ pointers as usize to avoid autocxx c_void type mismatches.
type Handle = usize;

// ============================================================================
// FfiHandle — RAII wrapper for C++ resource lifecycle
// ============================================================================

/// Identifies the kind of C++ resource held by an [`FfiHandle`], so that
/// `Drop` can call the correct C destructor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandleKind {
    Reader,
    Writer,
    Editor,
}

/// RAII wrapper around a raw C++ pointer stored in `Bit7zRepository.handles`.
///
/// When an `FfiHandle` is dropped it automatically calls the appropriate
/// C destructor (`bit7z_reader_close`, `bit7z_writer_close`, or
/// `bit7z_editor_close`), preventing resource leaks even on panic paths.
pub struct FfiHandle {
    ptr: *mut std::ffi::c_void,
    kind: HandleKind,
}

// SAFETY: FfiHandle wraps a raw FFI pointer. The underlying C++ bit7z
// library is thread-safe for concurrent read operations. Mutable
// operations are serialized through the repository's RwLock.
unsafe impl Send for FfiHandle {}
unsafe impl Sync for FfiHandle {}

impl FfiHandle {
    pub fn reader(ptr: *mut std::ffi::c_void) -> Self {
        Self { ptr, kind: HandleKind::Reader }
    }

    pub fn writer(ptr: *mut std::ffi::c_void) -> Self {
        Self { ptr, kind: HandleKind::Writer }
    }

    pub fn editor(ptr: *mut std::ffi::c_void) -> Self {
        Self { ptr, kind: HandleKind::Editor }
    }

    pub fn ptr(&self) -> *mut std::ffi::c_void {
        self.ptr
    }

    pub fn kind(&self) -> HandleKind {
        self.kind
    }

    pub fn is_null(&self) -> bool {
        self.ptr.is_null()
    }
}

impl Drop for FfiHandle {
    fn drop(&mut self) {
        if self.ptr.is_null() {
            return;
        }
        unsafe {
            match self.kind {
                HandleKind::Reader => crate::ffi::bit7z_reader_close(self.ptr as *mut _),
                HandleKind::Writer => bit7z_writer_close(self.ptr as *mut _),
                HandleKind::Editor => bit7z_editor_close(self.ptr as *mut _),
            }
        }
    }
}

// ============================================================================
// Library
// ============================================================================

pub struct Library {
    raw: Handle,
}

// SAFETY: Library wraps a raw FFI handle (usize) to a C++ bit7z library instance.
// The underlying C++ library is thread-safe for concurrent read operations.
// All FFI calls go through the repository's Mutex-protected methods, ensuring
// serialized access to mutable operations.
unsafe impl Send for Library {}
unsafe impl Sync for Library {}

impl Library {
    pub fn open(path: &str) -> Result<Self, String> {
        let c_path = std::ffi::CString::new(path).map_err(|e| format!("Invalid path: {}", e))?;
        let raw = unsafe { crate::ffi::bit7z_create_library(c_path.as_ptr()) } as Handle;
        if raw == 0 {
            return Err("Failed to load 7-Zip library".into());
        }
        Ok(Self { raw })
    }

    /// Take ownership of the raw handle (prevents Drop from destroying).
    pub fn into_raw(self) -> Handle {
        let h = self.raw;
        std::mem::forget(self);
        h
    }

    /// Create from a raw handle (takes ownership).
    pub unsafe fn from_raw(raw: Handle) -> Self {
        Self { raw }
    }

    /// Borrow the raw FFI handle.
    pub fn raw_handle(&self) -> Handle {
        self.raw
    }

    /// Check if archive at path has encrypted headers (static check without opening).
    pub fn is_header_encrypted(&self, path: &str) -> bool {
        let c_path = match std::ffi::CString::new(path) { Ok(p) => p, Err(_) => return false };
        unsafe { crate::ffi::bit7z_is_header_encrypted(self.raw as *mut _, c_path.as_ptr()) != 0 }
    }

    /// Check if archive at path is encrypted (static check without opening).
    pub fn is_encrypted(&self, path: &str) -> bool {
        let c_path = match std::ffi::CString::new(path) { Ok(p) => p, Err(_) => return false };
        unsafe { crate::ffi::bit7z_is_encrypted(self.raw as *mut _, c_path.as_ptr()) != 0 }
    }
}

impl Drop for Library {
    fn drop(&mut self) {
        unsafe { crate::ffi::bit7z_destroy_library(self.raw as *mut _); }
    }
}

// ============================================================================
// ArchiveReader
// ============================================================================

pub struct ArchiveReader {
    raw: Handle,
}

// SAFETY: ArchiveReader wraps a raw FFI handle (usize) to a C++ archive reader.
// The handle is only used for read operations which are thread-safe in bit7z.
// The handle is stored in Bit7zRepository's HashMap protected by Mutex, and
// all access goes through methods that lock the mutex first.
unsafe impl Send for ArchiveReader {}
unsafe impl Sync for ArchiveReader {}

impl ArchiveReader {
    pub fn open(lib: &Library, path: &str, password: Option<&Password>) -> Result<Self, String> {
        let c_path = std::ffi::CString::new(path).map_err(|e| format!("Invalid path: {}", e))?;
        let c_pw = password.map(|p| std::ffi::CString::new(p.as_str())).transpose()
            .map_err(|e| format!("Invalid password: {}", e))?;
        let raw = unsafe {
            crate::ffi::bit7z_reader_open(
                lib.raw as *mut _,
                c_path.as_ptr(),
                c_pw.as_ref().map_or(ptr::null(), |p| p.as_ptr()),
            )
        } as Handle;
        if raw == 0 {
            return Err("Failed to open archive".into());
        }
        Ok(Self { raw })
    }

    pub fn item_count(&self) -> u32 {
        unsafe { crate::ffi::bit7z_reader_item_count(self.raw as *mut _) }
    }

    pub fn item(&self, index: u32) -> Item<'_> {
        Item { reader: self, index }
    }

    pub fn extract_to(&self, indices: &[u32], dest: &str) -> Result<(), String> {
        let c_dest = std::ffi::CString::new(dest).map_err(|e| format!("Invalid path: {}", e))?;
        let ret: i32 = unsafe {
            crate::ffi::bit7z_reader_extract_to(
                self.raw as *mut _,
                indices.as_ptr(),
                indices.len() as u32,
                c_dest.as_ptr(),
            )
        };
        if ret != 0 { Err("Extraction failed".into()) } else { Ok(()) }
    }

    pub fn extract_to_buffer(&self, index: u32) -> Result<Vec<u8>, String> {
        let mut out_data: *mut std::ffi::c_void = std::ptr::null_mut();
        let mut out_size: i64 = 0;
        let ret = unsafe {
            bit7z_reader_extract_to_buffer_c(
                self.raw as *mut _,
                index,
                &mut out_data,
                &mut out_size,
            )
        };
        if ret != 0 || out_data.is_null() || out_size <= 0 {
            return Err("Extraction to buffer failed".into());
        }
        let slice = unsafe { std::slice::from_raw_parts(out_data as *const u8, out_size as usize) };
        let result = slice.to_vec();
        unsafe { crate::ffi::bit7z_reader_free_buffer(out_data as *mut autocxx::c_void); }
        Ok(result)
    }

    /// Borrow the raw FFI handle.
    pub fn raw_handle(&self) -> Handle {
        self.raw
    }

    /// Take ownership of the raw handle (prevents Drop from closing).
    pub fn into_raw(self) -> Handle {
        let h = self.raw;
        std::mem::forget(self);
        h
    }

    /// Create from a raw handle (takes ownership).
    pub unsafe fn from_raw(raw: Handle) -> Self {
        Self { raw }
    }

    /// Test archive integrity.
    pub fn test(&self) -> Result<(bool, u32, u32, Vec<String>, Vec<String>), String> {
        let result = unsafe { crate::ffi::bit7z_reader_test(self.raw as *mut _) };
        if result.is_null() {
            return Err("test call failed".into());
        }
        let all_ok = unsafe { crate::ffi::bit7z_test_result_all_ok(result) } != 0;
        let total = unsafe { crate::ffi::bit7z_test_result_total(result) };
        let failed_count = unsafe { crate::ffi::bit7z_test_result_failed_count(result) };
        let failed_paths = Vec::new();
        let mut failed_errors = Vec::new();
        // Note: The C++ implementation only stores one error path/error for exception case
        // For per-item failures, we'd need extended C++ API
        if !all_ok && failed_count > 0 {
            let error_msg = unsafe {
                let ptr = crate::ffi::bit7z_test_result_error(result);
                if ptr.is_null() { "test failed".to_string() }
                else { std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned() }
            };
            failed_errors.push(error_msg);
        }
        unsafe { crate::ffi::bit7z_test_result_free(result); }
        Ok((all_ok, total, failed_count, failed_paths, failed_errors))
    }

    /// Check if opened archive has any encrypted items.
    pub fn has_encrypted_items(&self) -> bool {
        unsafe { bit7z_reader_has_encrypted_items(self.raw as *mut _) != 0 }
    }
}

impl Drop for ArchiveReader {
    fn drop(&mut self) {
        unsafe { crate::ffi::bit7z_reader_close(self.raw as *mut _); }
    }
}

// C-linkage callback-based extraction (declared in demo.h)
extern "C" {
    fn bit7z_reader_extract_to_cb_c(
        reader: *mut std::ffi::c_void,
        indices: *const u32,
        count: u32,
        dest: *const std::ffi::c_char,
        ctx: *mut std::ffi::c_void,
        on_overwrite: Option<unsafe extern "C" fn(*const std::ffi::c_char, *const std::ffi::c_char, u64, u64, i64, i64, *mut std::ffi::c_void) -> i32>,
        on_progress: Option<unsafe extern "C" fn(u64, u64, *mut std::ffi::c_void) -> i32>,
        on_file: Option<unsafe extern "C" fn(*const std::ffi::c_char, u64, *mut std::ffi::c_void)>,
    ) -> i32;
}

impl ArchiveReader {
    /// Extracts items with per-file overwrite/progress/file callbacks.
    /// `ctx` is passed to every callback as opaque user data.
    #[allow(clippy::too_many_arguments)]
    pub unsafe fn extract_to_cb(
        &self,
        indices: &[u32],
        dest: &str,
        ctx: *mut std::ffi::c_void,
        on_overwrite: Option<unsafe extern "C" fn(*const std::ffi::c_char, *const std::ffi::c_char, u64, u64, i64, i64, *mut std::ffi::c_void) -> i32>,
        on_progress: Option<unsafe extern "C" fn(u64, u64, *mut std::ffi::c_void) -> i32>,
        on_file: Option<unsafe extern "C" fn(*const std::ffi::c_char, u64, *mut std::ffi::c_void)>,
    ) -> Result<(), String> {
        let c_dest = std::ffi::CString::new(dest).map_err(|e| format!("{}", e))?;
        let ret = bit7z_reader_extract_to_cb_c(
            self.raw as *mut std::ffi::c_void,
            indices.as_ptr(),
            indices.len() as u32,
            c_dest.as_ptr(),
            ctx,
            on_overwrite,
            on_progress,
            on_file,
        );
        if ret == 0 { Ok(()) } else { Err("extraction failed or cancelled".into()) }
    }

    /// Extract all items with per-file rename/skip/overwrite via RenameCallback.
    #[allow(clippy::too_many_arguments)]
    pub unsafe fn extract_with_rename(
        &self,
        dest: &str,
        ctx: *mut std::ffi::c_void,
        on_rename: Option<unsafe extern "C" fn(*const std::ffi::c_char, u64, i32, *mut std::ffi::c_char, u32, *mut std::ffi::c_void) -> i32>,
        on_progress: Option<unsafe extern "C" fn(u64, u64, *mut std::ffi::c_void) -> i32>,
        on_file: Option<unsafe extern "C" fn(*const std::ffi::c_char, *mut std::ffi::c_void)>,
    ) -> Result<(), String> {
        let c_dest = std::ffi::CString::new(dest).map_err(|e| format!("{}", e))?;
        let ret = bit7z_reader_extract_with_rename_c(
            self.raw as *mut std::ffi::c_void,
            c_dest.as_ptr(),
            ctx,
            on_rename,
            on_progress,
            on_file,
        );
        if ret == 0 { Ok(()) } else { Err("extraction failed or cancelled".into()) }
    }
}

extern "C" {
    fn bit7z_reader_extract_with_rename_c(
        reader: *mut std::ffi::c_void,
        dest: *const std::ffi::c_char,
        ctx: *mut std::ffi::c_void,
        on_rename: Option<unsafe extern "C" fn(*const std::ffi::c_char, u64, i32, *mut std::ffi::c_char, u32, *mut std::ffi::c_void) -> i32>,
        on_progress: Option<unsafe extern "C" fn(u64, u64, *mut std::ffi::c_void) -> i32>,
        on_file: Option<unsafe extern "C" fn(*const std::ffi::c_char, *mut std::ffi::c_void)>,
    ) -> i32;
}

// ============================================================================
// Writer / Editor FFI declarations (extern "C" linkage in demo.h)
// ============================================================================

extern "C" {
    fn bit7z_writer_create(lib: *mut std::ffi::c_void, format: i32) -> *mut std::ffi::c_void;
    fn bit7z_writer_open(lib: *mut std::ffi::c_void, path: *const std::ffi::c_char, format: i32, password: *const std::ffi::c_char) -> *mut std::ffi::c_void;
    fn bit7z_writer_close(w: *mut std::ffi::c_void);
    fn bit7z_writer_set_threads(w: *mut std::ffi::c_void, n: u32);
    fn bit7z_writer_set_compression_level(w: *mut std::ffi::c_void, level: i32);
    fn bit7z_writer_set_password(w: *mut std::ffi::c_void, password: *const std::ffi::c_char);
    fn bit7z_writer_set_update_mode(w: *mut std::ffi::c_void, mode: i32);
    fn bit7z_writer_add_file(w: *mut std::ffi::c_void, path: *const std::ffi::c_char) -> i32;
    fn bit7z_writer_add_files(w: *mut std::ffi::c_void, paths: *const *const std::ffi::c_char, count: u32) -> i32;
    // bit7z_writer_add_items uses `const char**` which autocxx cannot bind,
    // so it is declared manually here instead of via generate!() in ffi.rs.
    fn bit7z_writer_add_items(w: *mut std::ffi::c_void, paths: *const *const std::ffi::c_char, archive_paths: *const *const std::ffi::c_char, count: u32) -> i32;
    fn bit7z_writer_add_dir(w: *mut std::ffi::c_void, dir: *const std::ffi::c_char) -> i32;
    fn bit7z_writer_compress_to(w: *mut std::ffi::c_void, out_path: *const std::ffi::c_char) -> i32;
    fn bit7z_writer_compress_to_cb(
        w: *mut std::ffi::c_void,
        out_path: *const std::ffi::c_char,
        ctx: *mut std::ffi::c_void,
        on_progress: Option<unsafe extern "C" fn(u64, u64, *mut std::ffi::c_void) -> i32>,
        on_file: Option<unsafe extern "C" fn(*const std::ffi::c_char, *mut std::ffi::c_void)>,
    ) -> i32;
    fn bit7z_editor_open(lib: *mut std::ffi::c_void, path: *const std::ffi::c_char, format: i32, password: *const std::ffi::c_char) -> *mut std::ffi::c_void;
    fn bit7z_editor_close(e: *mut std::ffi::c_void);
    fn bit7z_editor_rename(e: *mut std::ffi::c_void, index: u32, new_path: *const std::ffi::c_char) -> i32;
    fn bit7z_editor_delete(e: *mut std::ffi::c_void, index: u32) -> i32;
    fn bit7z_editor_apply(e: *mut std::ffi::c_void) -> i32;

    // Test archive integrity — not yet used (inline in demo.h, missing linker symbols)
    // fn bit7z_reader_test(reader: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
    // fn bit7z_test_result_total(result: *mut std::ffi::c_void) -> u32;
    // fn bit7z_test_result_failed_count(result: *mut std::ffi::c_void) -> u32;
    // fn bit7z_test_result_all_ok(result: *mut std::ffi::c_void) -> i32;
    // fn bit7z_test_result_error(result: *mut std::ffi::c_void) -> *const std::ffi::c_char;
    // fn bit7z_test_result_free(result: *mut std::ffi::c_void);

    // Encryption detection
    fn bit7z_reader_has_encrypted_items(reader: *mut std::ffi::c_void) -> i32;
    // Single-call extract to buffer (avoids double-extraction).
    pub fn bit7z_reader_extract_to_buffer_c(
        reader: *mut std::ffi::c_void,
        index: u32,
        out_data: *mut *mut std::ffi::c_void,
        out_size: *mut i64,
    ) -> i32;
}

// ============================================================================
// Writer
// ============================================================================

/// Formats supported for writing (compressing).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum WriterFormat {
    SevenZip = 0,
    Zip = 1,
    Tar = 2,
    GZip = 3,
    BZip2 = 4,
    Xz = 5,
    Wim = 6,
}

/// Compression levels mapped to bit7z BitCompressionLevel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum WriterCompressionLevel {
    None = 0,
    Fastest = 1,
    Fast = 2,
    Normal = 3,
    Max = 4,
    Ultra = 5,
}

/// Update mode for modifying existing archives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateMode {
    None = 0,
    Append = 1,
    Update = 2,
}

/// Compression method for writing archives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum WriterCompressionMethod {
    Copy = 0,
    Deflate = 1,
    Deflate64 = 2,
    BZip2 = 3,
    Lzma = 4,
    Lzma2 = 5,
    Ppmd = 6,
}

/// Encryption scope for archive passwords.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncryptionScope {
    DataOnly = 0,
    DataAndHeaders = 1,
}

/// Filter policy for directory enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum FilterPolicy {
    Include = 0,
    Exclude = 1,
}

pub struct Writer {
    raw: Handle,
}

// SAFETY: Writer wraps a raw FFI handle (usize) to a C++ archive writer.
// Writer instances are short-lived and used within a single operation.
// All access is serialized through Bit7zRepository's Mutex-protected library lock.
unsafe impl Send for Writer {}
unsafe impl Sync for Writer {}

impl Writer {
    pub fn create(lib: &Library, format: WriterFormat) -> Result<Self, String> {
        let raw = unsafe { bit7z_writer_create(lib.raw_handle() as *mut _, format as i32) };
        if raw.is_null() { Err("failed to create writer".into()) }
        else { Ok(Self { raw: raw as Handle }) }
    }

    pub unsafe fn from_raw(raw: Handle) -> Self {
        Self { raw }
    }

    pub fn open(lib: &Library, path: &str, format: WriterFormat, password: Option<&Password>) -> Result<Self, String> {
        let c_path = std::ffi::CString::new(path).map_err(|e| format!("{}", e))?;
        let c_pw = password.and_then(|p| std::ffi::CString::new(p.as_str()).ok());
        let raw = unsafe {
            bit7z_writer_open(
                lib.raw_handle() as *mut _,
                c_path.as_ptr(),
                format as i32,
                c_pw.as_ref().map_or(std::ptr::null(), |s| s.as_ptr()),
            )
        };
        if raw.is_null() { Err("failed to open writer".into()) }
        else { Ok(Self { raw: raw as Handle }) }
    }

    pub fn set_threads(&self, n: u32) {
        unsafe { bit7z_writer_set_threads(self.raw as *mut _, n); }
    }

    pub fn set_compression_level(&self, level: WriterCompressionLevel) {
        unsafe { bit7z_writer_set_compression_level(self.raw as *mut _, level as i32); }
    }

    pub fn set_password(&self, password: &str) {
        let c_pw = std::ffi::CString::new(password).unwrap();
        unsafe { bit7z_writer_set_password(self.raw as *mut _, c_pw.as_ptr()); }
    }

    pub fn set_update_mode(&self, mode: UpdateMode) {
        unsafe { bit7z_writer_set_update_mode(self.raw as *mut _, mode as i32); }
    }

    pub fn add_file(&self, path: &str) -> Result<(), String> {
        let c_path = std::ffi::CString::new(path).map_err(|e| format!("{}", e))?;
        let ret = unsafe { bit7z_writer_add_file(self.raw as *mut _, c_path.as_ptr()) };
        if ret == 0 { Ok(()) } else { Err("add_file failed".into()) }
    }

    pub fn add_files(&self, paths: &[&str]) -> Result<(), String> {
        let c_paths: Vec<std::ffi::CString> = paths.iter()
            .filter_map(|p| std::ffi::CString::new(*p).ok())
            .collect();
        let ptrs: Vec<*const std::ffi::c_char> = c_paths.iter().map(|s| s.as_ptr()).collect();
        let ret = unsafe { bit7z_writer_add_files(self.raw as *mut _, ptrs.as_ptr(), ptrs.len() as u32) };
        if ret == 0 { Ok(()) } else { Err("add_files failed".into()) }
    }

    pub fn add_directory(&self, dir: &str) -> Result<(), String> {
        let c_dir = std::ffi::CString::new(dir).map_err(|e| format!("{}", e))?;
        let ret = unsafe { bit7z_writer_add_dir(self.raw as *mut _, c_dir.as_ptr()) };
        if ret == 0 { Ok(()) } else { Err("add_directory failed".into()) }
    }

    pub fn compress_to(&self, out_path: &str) -> Result<(), String> {
        let c_out = std::ffi::CString::new(out_path).map_err(|e| format!("{}", e))?;
        let ret = unsafe { bit7z_writer_compress_to(self.raw as *mut _, c_out.as_ptr()) };
        if ret == 0 { Ok(()) } else { Err("compress_to failed".into()) }
    }

    #[allow(clippy::too_many_arguments)]
    pub unsafe fn compress_to_cb(
        &self,
        out_path: &str,
        ctx: *mut std::ffi::c_void,
        on_progress: Option<unsafe extern "C" fn(u64, u64, *mut std::ffi::c_void) -> i32>,
        on_file: Option<unsafe extern "C" fn(*const std::ffi::c_char, *mut std::ffi::c_void)>,
    ) -> Result<(), String> {
        let c_out = std::ffi::CString::new(out_path).map_err(|e| format!("{}", e))?;
        let ret = bit7z_writer_compress_to_cb(
            self.raw as *mut std::ffi::c_void,
            c_out.as_ptr(),
            ctx,
            on_progress,
            on_file,
        );
        if ret == 0 { Ok(()) } else { Err("compress_to failed or cancelled".into()) }
    }

    pub fn set_compression_method(&self, method: WriterCompressionMethod) {
        unsafe { crate::ffi::bit7z_writer_set_compression_method(self.raw as *mut _, c_int(method as i32)); }
    }

    pub fn set_dictionary_size(&self, bytes: u32) {
        unsafe { crate::ffi::bit7z_writer_set_dictionary_size(self.raw as *mut _, bytes); }
    }

    pub fn set_word_size(&self, bytes: u32) {
        unsafe { crate::ffi::bit7z_writer_set_word_size(self.raw as *mut _, bytes); }
    }

    pub fn set_solid_mode(&self, solid: bool) {
        unsafe { crate::ffi::bit7z_writer_set_solid_mode(self.raw as *mut _, c_int(solid as i32)); }
    }

    pub fn set_volume_size(&self, bytes: u64) {
        unsafe { crate::ffi::bit7z_writer_set_volume_size(self.raw as *mut _, bytes); }
    }

    pub fn set_password_ex(&self, password: &str, encrypt_header: bool) {
        let c_pw = std::ffi::CString::new(password).unwrap();
        unsafe { crate::ffi::bit7z_writer_set_password_ex(self.raw as *mut _, c_pw.as_ptr(), c_int(encrypt_header as i32)); }
    }

    pub fn set_store_timestamps(&self, modified: bool, created: bool, accessed: bool) {
        unsafe {
            crate::ffi::bit7z_writer_set_store_timestamps(
                self.raw as *mut _,
                c_int(modified as i32),
                c_int(created as i32),
                c_int(accessed as i32),
            );
        }
    }

    pub fn add_dir_filtered(&self, dir: &str, filter: &str, policy: FilterPolicy, recursive: bool) -> Result<(), String> {
        let c_dir = std::ffi::CString::new(dir).map_err(|e| format!("{}", e))?;
        let c_filter = std::ffi::CString::new(filter).map_err(|e| format!("{}", e))?;
        let ret = unsafe {
            crate::ffi::bit7z_writer_add_dir_filtered(
                self.raw as *mut _,
                c_dir.as_ptr(),
                c_filter.as_ptr(),
                c_int(policy as i32),
                c_int(recursive as i32),
            )
        };
        if ret == 0 { Ok(()) } else { Err("add_dir_filtered failed".into()) }
    }

    pub fn add_items(&self, paths_and_names: &[(&str, &str)]) -> Result<(), String> {
        let c_paths: Vec<std::ffi::CString> = paths_and_names
            .iter()
            .filter_map(|(p, _)| std::ffi::CString::new(*p).ok())
            .collect();
        let c_names: Vec<std::ffi::CString> = paths_and_names
            .iter()
            .filter_map(|(_, n)| std::ffi::CString::new(*n).ok())
            .collect();
        let path_ptrs: Vec<*const std::ffi::c_char> = c_paths.iter().map(|s| s.as_ptr()).collect();
        let name_ptrs: Vec<*const std::ffi::c_char> = c_names.iter().map(|s| s.as_ptr()).collect();
        let ret = unsafe {
            bit7z_writer_add_items(
                self.raw as *mut _,
                path_ptrs.as_ptr(),
                name_ptrs.as_ptr(),
                path_ptrs.len() as u32,
            )
        };
        if ret == 0 { Ok(()) } else { Err("add_items failed".into()) }
    }

    /// Borrow the raw FFI handle.
    pub fn raw_handle(&self) -> Handle {
        self.raw
    }

    /// Take ownership of the raw handle (prevents Drop from closing).
    pub fn into_raw(self) -> Handle {
        let h = self.raw;
        std::mem::forget(self);
        h
    }
}

impl Drop for Writer {
    fn drop(&mut self) {
        unsafe { bit7z_writer_close(self.raw as *mut _); }
    }
}

// ============================================================================
// Editor
// ============================================================================

pub struct Editor {
    raw: Handle,
}

// SAFETY: Editor wraps a raw FFI handle (usize) to a C++ archive editor.
// Editor instances are short-lived and used within a single operation.
// All access is serialized through Bit7zRepository's Mutex-protected library lock.
unsafe impl Send for Editor {}
unsafe impl Sync for Editor {}

impl Editor {
    pub fn open(lib: &Library, path: &str, format: WriterFormat, password: Option<&str>) -> Result<Self, String> {
        let c_path = std::ffi::CString::new(path).map_err(|e| format!("{}", e))?;
        let c_pw = password.and_then(|p| std::ffi::CString::new(p).ok());
        let raw = unsafe {
            bit7z_editor_open(
                lib.raw_handle() as *mut _,
                c_path.as_ptr(),
                format as i32,
                c_pw.as_ref().map_or(std::ptr::null(), |s| s.as_ptr()),
            )
        };
        if raw.is_null() { Err("failed to open editor".into()) }
        else { Ok(Self { raw: raw as Handle }) }
    }

    pub fn rename(&self, index: u32, new_path: &str) -> Result<(), String> {
        let c_path = std::ffi::CString::new(new_path).map_err(|e| format!("{}", e))?;
        let ret = unsafe { bit7z_editor_rename(self.raw as *mut _, index, c_path.as_ptr()) };
        if ret == 0 { Ok(()) } else { Err("rename failed".into()) }
    }

    pub fn delete(&self, index: u32) -> Result<(), String> {
        let ret = unsafe { bit7z_editor_delete(self.raw as *mut _, index) };
        if ret == 0 { Ok(()) } else { Err("delete failed".into()) }
    }

    pub fn apply(&self) -> Result<(), String> {
        let ret = unsafe { bit7z_editor_apply(self.raw as *mut _) };
        if ret == 0 { Ok(()) } else { Err("apply changes failed".into()) }
    }
}

impl Drop for Editor {
    fn drop(&mut self) {
        unsafe { bit7z_editor_close(self.raw as *mut _); }
    }
}

// ============================================================================
// Item
// ============================================================================

pub struct Item<'a> {
    reader: &'a ArchiveReader,
    pub index: u32,
}

impl<'a> Item<'a> {
    pub fn path(&self) -> String {
        let p = unsafe { crate::ffi::bit7z_item_path(self.reader.raw as *mut _, self.index) };
        if p.is_null() { String::new() }
        else { unsafe { CStr::from_ptr(p).to_string_lossy().into_owned() } }
    }
    pub fn name(&self) -> String {
        let n = unsafe { crate::ffi::bit7z_item_name(self.reader.raw as *mut _, self.index) };
        if n.is_null() { String::new() }
        else { unsafe { CStr::from_ptr(n).to_string_lossy().into_owned() } }
    }
    pub fn size(&self) -> u64 {
        unsafe { crate::ffi::bit7z_item_size(self.reader.raw as *mut _, self.index) }
    }
    pub fn packed_size(&self) -> u64 {
        unsafe { crate::ffi::bit7z_item_packed_size(self.reader.raw as *mut _, self.index) }
    }
    pub fn is_directory(&self) -> bool {
        unsafe { crate::ffi::bit7z_item_is_dir(self.reader.raw as *mut _, self.index) != 0 }
    }
    pub fn is_encrypted(&self) -> bool {
        unsafe { crate::ffi::bit7z_item_is_encrypted(self.reader.raw as *mut _, self.index) != 0 }
    }

    fn raw_ptr(&self) -> *mut c_void {
        unsafe { crate::ffi::bit7z_item_from_reader(self.reader.raw as *mut _, self.index) }
    }

    pub fn mtime(&self) -> Result<u64, String> {
        let result = unsafe { crate::ffi::bit7z_item_mtime(self.raw_ptr()) };
        Ok(result)
    }

    pub fn ctime(&self) -> Result<u64, String> {
        let result = unsafe { crate::ffi::bit7z_item_ctime(self.raw_ptr()) };
        Ok(result)
    }

    pub fn atime(&self) -> Result<u64, String> {
        let result = unsafe { crate::ffi::bit7z_item_atime(self.raw_ptr()) };
        Ok(result)
    }

    pub fn attributes(&self) -> Result<u32, String> {
        let result = unsafe { crate::ffi::bit7z_item_attributes(self.raw_ptr()) };
        Ok(result)
    }

    pub fn host_os(&self) -> Result<u8, String> {
        let result = unsafe { crate::ffi::bit7z_item_host_os(self.raw_ptr()) };
        Ok(result)
    }

    pub fn compression_method(&self) -> Result<String, String> {
        let buf_size: u32 = 256;
        let mut buf: Vec<u8> = vec![0u8; buf_size as usize];
        let ret = unsafe {
            crate::ffi::bit7z_item_compression_method(self.raw_ptr(), buf.as_mut_ptr() as *mut _, buf_size)
        };
        if ret < 0 { return Err("failed to get compression method".into()); }
        let c_str = unsafe { CStr::from_ptr(buf.as_ptr() as *const _) };
        Ok(c_str.to_string_lossy().into_owned())
    }

    pub fn comment(&self) -> Result<String, String> {
        let buf_size: u32 = 256;
        let mut buf: Vec<u8> = vec![0u8; buf_size as usize];
        let ret = unsafe {
            crate::ffi::bit7z_item_comment(self.raw_ptr(), buf.as_mut_ptr() as *mut _, buf_size)
        };
        if ret < 0 { return Err("failed to get comment".into()); }
        let c_str = unsafe { CStr::from_ptr(buf.as_ptr() as *const _) };
        Ok(c_str.to_string_lossy().into_owned())
    }

    pub fn user(&self) -> Result<String, String> {
        let buf_size: u32 = 256;
        let mut buf: Vec<u8> = vec![0u8; buf_size as usize];
        let ret = unsafe {
            crate::ffi::bit7z_item_user(self.raw_ptr(), buf.as_mut_ptr() as *mut _, buf_size)
        };
        if ret < 0 { return Err("failed to get user".into()); }
        let c_str = unsafe { CStr::from_ptr(buf.as_ptr() as *const _) };
        Ok(c_str.to_string_lossy().into_owned())
    }

    pub fn group(&self) -> Result<String, String> {
        let buf_size: u32 = 256;
        let mut buf: Vec<u8> = vec![0u8; buf_size as usize];
        let ret = unsafe {
            crate::ffi::bit7z_item_group(self.raw_ptr(), buf.as_mut_ptr() as *mut _, buf_size)
        };
        if ret < 0 { return Err("failed to get group".into()); }
        let c_str = unsafe { CStr::from_ptr(buf.as_ptr() as *const _) };
        Ok(c_str.to_string_lossy().into_owned())
    }

    pub fn is_symlink(&self) -> Result<bool, String> {
        let result = unsafe { crate::ffi::bit7z_item_is_symlink(self.raw_ptr()) };
        Ok(result != 0)
    }

    pub fn posix_attrib(&self) -> Result<u32, String> {
        let result = unsafe { crate::ffi::bit7z_item_posix_attrib(self.raw_ptr()) };
        Ok(result)
    }

    pub fn extension(&self) -> Result<String, String> {
        let buf_size: u32 = 256;
        let mut buf: Vec<u8> = vec![0u8; buf_size as usize];
        let ret = unsafe {
            crate::ffi::bit7z_item_extension(self.raw_ptr(), buf.as_mut_ptr() as *mut _, buf_size)
        };
        if ret < 0 { return Err("failed to get extension".into()); }
        let c_str = unsafe { CStr::from_ptr(buf.as_ptr() as *const _) };
        Ok(c_str.to_string_lossy().into_owned())
    }

    pub fn hardlink(&self) -> Result<String, String> {
        let buf_size: u32 = 256;
        let mut buf: Vec<u8> = vec![0u8; buf_size as usize];
        let ret = unsafe {
            crate::ffi::bit7z_item_hardlink(self.raw_ptr(), buf.as_mut_ptr() as *mut _, buf_size)
        };
        if ret < 0 { return Err("failed to get hardlink".into()); }
        let c_str = unsafe { CStr::from_ptr(buf.as_ptr() as *const _) };
        Ok(c_str.to_string_lossy().into_owned())
    }
}



