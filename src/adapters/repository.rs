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
    fn open(&self, path: &Path, password: Option<&Password>) -> Result<ArchiveHandle, ArchiveError> {
        let lib = self.lib.lock().map_err(|e| ArchiveError::Internal(e.to_string()))?;
        let path_str = path.to_str()
            .ok_or_else(|| ArchiveError::Internal("Non-UTF-8 path".into()))?;
        let reader = bit7z::ArchiveReader::open(&lib, path_str, password)
            .map_err(|e| ArchiveError::Internal(e))?;
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

        Ok(ArchiveHandle::new_reader(raw_handle as *mut std::ffi::c_void)
            .with_path(path.to_path_buf())
            .with_format_opt(format))
    }

    fn create(&self, path: &Path, format: ArchiveFormat,
              encryption: Option<&EncryptionConfig>) -> Result<ArchiveHandle, ArchiveError> {
        let lib = self.lib.lock().map_err(|e| ArchiveError::Internal(e.to_string()))?;
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
                writer.set_password(&enc.password);
            }
        }

        let raw_handle = writer.into_raw();
        Ok(ArchiveHandle::new_writer(raw_handle as *mut std::ffi::c_void))
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
                original_index: i,
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
            entries.push(ArchiveEntry {
                name,
                path,
                size: unsafe { crate::ffi::bit7z_item_list_size(list, i) },
                compressed_size: unsafe { crate::ffi::bit7z_item_list_packed_size(list, i) },
                is_directory: unsafe { crate::ffi::bit7z_item_list_is_dir(list, i) != 0 },
                is_encrypted: unsafe { crate::ffi::bit7z_item_list_is_encrypted(list, i) != 0 },
                is_symlink: false,
                modified: None,
                crc: None,
                original_index: orig_idx,
            });
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
        fn add(&self, archive: &mut ArchiveHandle, files: &[PathBuf]) -> Result<(), ArchiveError> {
            self.inner.add(archive, files)
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
        ArchiveEntry {
            name: String::new(), path: String::new(), size: 0, compressed_size: 0,
            is_directory: false, is_encrypted: false, is_symlink: false,
            modified: None, crc: None, original_index: 0,
        }
    }
}
