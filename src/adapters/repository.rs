use crate::adapters::bit7z;
use crate::domain::archive::*;
use crate::domain::repository::*;
use std::path::Path;
use std::sync::Mutex;

/// FFI-backed implementation of ArchiveRepository using the C wrapper layer.
pub struct Bit7zRepository {
    lib: Mutex<bit7z::Library>,
}

unsafe impl Send for Bit7zRepository {}
unsafe impl Sync for Bit7zRepository {}

impl Bit7zRepository {
    pub fn new(lib: bit7z::Library) -> Self {
        Self { lib: Mutex::new(lib) }
    }
}

impl ArchiveRepository for Bit7zRepository {
    fn open(&self, path: &Path, password: Option<&str>) -> Result<ArchiveHandle, ArchiveError> {
        let lib = self.lib.lock().map_err(|e| ArchiveError::Internal(e.to_string()))?;
        let path_str = path.to_str()
            .ok_or_else(|| ArchiveError::Internal("Non-UTF-8 path".into()))?;
        let reader = bit7z::ArchiveReader::open(&lib, path_str, password)
            .map_err(|e| ArchiveError::Internal(e))?;
        let raw_handle = reader.into_raw();
        Ok(ArchiveHandle::new_reader(raw_handle as *mut std::ffi::c_void))
    }

    fn create(&self, _path: &Path, _format: ArchiveFormat,
              _encryption: Option<&EncryptionConfig>) -> Result<ArchiveHandle, ArchiveError> {
        Err(ArchiveError::UnsupportedOperation)
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

            entries.push(ArchiveEntry {
                name: name_s, path: path_s,
                size, compressed_size: csize,
                is_directory: is_dir, is_encrypted: is_enc,
                is_symlink: false, modified: None, crc: None,
            });
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
            is_encrypted: false,
            has_encrypted_items: false,
            is_multi_volume: false,
            is_solid: false,
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
        let size: i64 = unsafe {
            crate::ffi::bit7z_reader_extract_item_size(raw as *mut _, index)
        };
        if size <= 0 { return Err(ArchiveError::Internal("Extract buffer failed".into())); }
        let data = unsafe {
            crate::ffi::bit7z_reader_extract_item_data(raw as *mut _, index)
        };
        if data.is_null() {
            return Err(ArchiveError::Internal("Extract data null".into()));
        }
        let slice = unsafe { std::slice::from_raw_parts(data as *const u8, size as usize) };
        let result = slice.to_vec();
        unsafe { crate::ffi::bit7z_reader_free_buffer(data as *mut _); }
        Ok(result)
    }

    fn add(&self, _archive: &mut ArchiveHandle, _files: &[std::path::PathBuf])
           -> Result<(), ArchiveError> {
        Err(ArchiveError::UnsupportedOperation)
    }

    fn delete(&self, _archive: &mut ArchiveHandle, _indices: &[u32])
              -> Result<(), ArchiveError> {
        Err(ArchiveError::UnsupportedOperation)
    }

    fn rename(&self, _archive: &mut ArchiveHandle, _index: u32, _new_name: &str)
              -> Result<(), ArchiveError> {
        Err(ArchiveError::UnsupportedOperation)
    }

    fn test(&self, _archive: &ArchiveHandle) -> Result<TestResult, ArchiveError> {
        Err(ArchiveError::UnsupportedOperation)
    }

    fn close(&self, archive: ArchiveHandle) {
        let raw = archive.raw as usize;
        unsafe { crate::ffi::bit7z_reader_close(raw as *mut _); }
    }
}
