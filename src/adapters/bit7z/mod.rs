//! Safe Rust wrappers around the C-style FFI functions for bit7z.

pub mod worker;

use crate::domain::archive::Password;
use std::ffi::CStr;
use std::ptr;

/// Store opaque C++ pointers as usize to avoid autocxx c_void type mismatches.
type Handle = usize;

// ============================================================================
// Library
// ============================================================================

pub struct Library {
    raw: Handle,
}

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

    pub fn item(&self, index: u32) -> Item {
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
    let size: i64 = unsafe {
        crate::ffi::bit7z_reader_extract_item_size(self.raw as *mut _, index)
    };
    if size <= 0 { return Err("Extraction to buffer failed".into()); }
    let data = unsafe {
        crate::ffi::bit7z_reader_extract_item_data(self.raw as *mut _, index)
    };
    if data.is_null() {
        return Err("Extraction to buffer failed (null)".into());
    }
    let slice = unsafe { std::slice::from_raw_parts(data as *const u8, size as usize) };
    let result = slice.to_vec();
    unsafe { crate::ffi::bit7z_reader_free_buffer(data as *mut _); }
    Ok(result)
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

    /// Test archive integrity. Returns (all_ok, total, failed_count, error_message).
    pub fn test(&self) -> Result<(bool, u32, u32, String), String> {
        let result = unsafe { bit7z_reader_test(self.raw as *mut _) };
        if result.is_null() {
            return Err("Test failed".into());
        }
        let all_ok = unsafe { bit7z_test_result_all_ok(result) } != 0;
        let total = unsafe { bit7z_test_result_total(result) };
        let failed_count = unsafe { bit7z_test_result_failed_count(result) };
        let error = unsafe {
            let ptr = bit7z_test_result_error(result);
            if ptr.is_null() { String::new() }
            else { CStr::from_ptr(ptr).to_string_lossy().into_owned() }
        };
        unsafe { bit7z_test_result_free(result); }
        Ok((all_ok, total, failed_count, error))
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
        on_overwrite: Option<unsafe extern "C" fn(*const std::ffi::c_char, *const std::ffi::c_char, u64, *mut std::ffi::c_void) -> i32>,
        on_progress: Option<unsafe extern "C" fn(u64, u64, *mut std::ffi::c_void) -> i32>,
        on_file: Option<unsafe extern "C" fn(*const std::ffi::c_char, *mut std::ffi::c_void)>,
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
        on_overwrite: Option<unsafe extern "C" fn(*const std::ffi::c_char, *const std::ffi::c_char, u64, *mut std::ffi::c_void) -> i32>,
        on_progress: Option<unsafe extern "C" fn(u64, u64, *mut std::ffi::c_void) -> i32>,
        on_file: Option<unsafe extern "C" fn(*const std::ffi::c_char, *mut std::ffi::c_void)>,
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

    // Test archive integrity
    fn bit7z_reader_test(reader: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
    fn bit7z_test_result_total(result: *mut std::ffi::c_void) -> u32;
    fn bit7z_test_result_failed_count(result: *mut std::ffi::c_void) -> u32;
    fn bit7z_test_result_all_ok(result: *mut std::ffi::c_void) -> i32;
    fn bit7z_test_result_error(result: *mut std::ffi::c_void) -> *const std::ffi::c_char;
    fn bit7z_test_result_free(result: *mut std::ffi::c_void);
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

pub struct Writer {
    raw: Handle,
}

unsafe impl Send for Writer {}
unsafe impl Sync for Writer {}

impl Writer {
    pub fn create(lib: &Library, format: WriterFormat) -> Result<Self, String> {
        let raw = unsafe { bit7z_writer_create(lib.raw_handle() as *mut _, format as i32) };
        if raw.is_null() { Err("failed to create writer".into()) }
        else { Ok(Self { raw: raw as Handle }) }
    }

    pub fn open(lib: &Library, path: &str, format: WriterFormat, password: Option<&str>) -> Result<Self, String> {
        let c_path = std::ffi::CString::new(path).map_err(|e| format!("{}", e))?;
        let c_pw = password.and_then(|p| std::ffi::CString::new(p).ok());
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
}



