//! Safe Rust wrappers around the C-style FFI functions for bit7z.

use std::ffi::CStr;
use std::ptr;
use crate::domain::archive::*;

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
    pub fn open(lib: &Library, path: &str, password: Option<&str>) -> Result<Self, String> {
        let c_path = std::ffi::CString::new(path).map_err(|e| format!("Invalid path: {}", e))?;
        let c_pw = password.map(|p| std::ffi::CString::new(p)).transpose()
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
}

impl Drop for ArchiveReader {
    fn drop(&mut self) {
        unsafe { crate::ffi::bit7z_reader_close(self.raw as *mut _); }
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



