use crate::domain::archive::*;
use crate::domain::repository::*;
use std::sync::Arc;

pub enum PreviewData {
    Text(String),
    Hex(Vec<u8>),
    Image(Vec<u8>),
    Unsupported(String),
}

pub struct PreviewEntryUseCase {
    repo: Arc<dyn ArchiveRepository>,
}

impl PreviewEntryUseCase {
    pub fn new(repo: Arc<dyn ArchiveRepository>) -> Self {
        Self { repo }
    }

    pub fn execute(
        &self,
        archive: &ArchiveHandle,
        index: u32,
        max_bytes: usize,
    ) -> Result<PreviewData, ArchiveError> {
        let bytes = self.repo.extract_to_buffer(archive, index)?;
        let truncated: Vec<u8> = bytes.into_iter().take(max_bytes).collect();

        // Detect image by magic bytes
        if truncated.len() >= 4 {
            if &truncated[..4] == b"\x89PNG" || &truncated[..3] == b"\xFF\xD8\xFF" {
                return Ok(PreviewData::Image(truncated));
            }
        }

        // Try UTF-8 text
        if let Ok(text) = String::from_utf8(truncated.clone()) {
            if text.chars().all(|c| c.is_ascii_graphic() || c.is_ascii_whitespace()) || truncated.len() < 1024 {
                return Ok(PreviewData::Text(text));
            }
        }

        // Fall back to hex
        Ok(PreviewData::Hex(truncated))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::archive::{ArchiveEntry, ArchiveFormat, EncryptionConfig, Page, Password, TestResult};
    use crate::domain::repository::{ArchiveError, ArchiveProperties, ArchiveRepository};
    use std::sync::Arc;

    fn repo_with_buffer(data: Vec<u8>) -> Arc<dyn ArchiveRepository> {
        struct MockBuffer {
            data: Vec<u8>,
        }
        impl ArchiveRepository for MockBuffer {
            fn open(&self, _: &std::path::Path, _: Option<&Password>) -> Result<ArchiveHandle, ArchiveError> { Ok(ArchiveHandle::new_reader()) }
            fn create(&self, _: &std::path::Path, _: ArchiveFormat, _: Option<&EncryptionConfig>) -> Result<ArchiveHandle, ArchiveError> { Err(ArchiveError::UnsupportedOperation) }
            fn list_page(&self, _: &ArchiveHandle, _: usize, _: usize) -> Result<Page<ArchiveEntry>, ArchiveError> { Ok(Page::new(vec![], 0, Some(0))) }
            fn get_properties(&self, _: &ArchiveHandle) -> Result<ArchiveProperties, ArchiveError> { Ok(ArchiveProperties::default()) }
            fn extract(&self, _: &ArchiveHandle, _: &[u32], _: &std::path::Path) -> Result<(), ArchiveError> { Ok(()) }
            fn extract_to_buffer(&self, _: &ArchiveHandle, _: u32) -> Result<Vec<u8>, ArchiveError> { Ok(self.data.clone()) }
            fn add(&self, _: &mut ArchiveHandle, _: &[std::path::PathBuf], _: Option<&Password>) -> Result<(), ArchiveError> { Err(ArchiveError::UnsupportedOperation) }
            fn delete(&self, _: &mut ArchiveHandle, _: &[u32]) -> Result<(), ArchiveError> { Err(ArchiveError::UnsupportedOperation) }
            fn rename(&self, _: &mut ArchiveHandle, _: u32, _: &str) -> Result<(), ArchiveError> { Err(ArchiveError::UnsupportedOperation) }
            fn test(&self, _: &ArchiveHandle) -> Result<TestResult, ArchiveError> { Ok(TestResult { total: 0, passed: 0, failed: vec![] }) }
            fn list_directory(&self, _: &ArchiveHandle, _: &str) -> Result<Vec<ArchiveEntry>, ArchiveError> { Ok(vec![]) }
            fn close(&self, _: &ArchiveHandle) {}
        }
        Arc::new(MockBuffer { data })
    }

    #[test]
    fn test_preview_text() {
        let repo = repo_with_buffer(b"hello world".to_vec());
        let uc = PreviewEntryUseCase::new(repo);
        let handle = ArchiveHandle::new_reader();
        let result = uc.execute(&handle, 0, 4096).unwrap();
        assert!(matches!(result, PreviewData::Text(ref t) if t == "hello world"));
    }

    #[test]
    fn test_preview_hex_fallback() {
        let repo = repo_with_buffer(b"\x00\x01\x02\xFF\xFE".to_vec());
        let uc = PreviewEntryUseCase::new(repo);
        let handle = ArchiveHandle::new_reader();
        let result = uc.execute(&handle, 0, 4096).unwrap();
        assert!(matches!(result, PreviewData::Hex(_)));
    }

    #[test]
    fn test_preview_image_png() {
        let png_header = b"\x89PNG\x0D\x0A\x1A\x0Amore data";
        let repo = repo_with_buffer(png_header.to_vec());
        let uc = PreviewEntryUseCase::new(repo);
        let handle = ArchiveHandle::new_reader();
        let result = uc.execute(&handle, 0, 4096).unwrap();
        assert!(matches!(result, PreviewData::Image(_)));
    }

    #[test]
    fn test_preview_image_jpeg() {
        let jpeg_header = b"\xFF\xD8\xFF\xE0more data";
        let repo = repo_with_buffer(jpeg_header.to_vec());
        let uc = PreviewEntryUseCase::new(repo);
        let handle = ArchiveHandle::new_reader();
        let result = uc.execute(&handle, 0, 4096).unwrap();
        assert!(matches!(result, PreviewData::Image(_)));
    }

    #[test]
    fn test_preview_truncation() {
        let data = b"short data";
        let repo = repo_with_buffer(data.to_vec());
        let uc = PreviewEntryUseCase::new(repo);
        let handle = ArchiveHandle::new_reader();
        let result = uc.execute(&handle, 0, 5).unwrap();
        match result {
            PreviewData::Text(t) => assert_eq!(t.len(), 5),
            _ => panic!("expected Text"),
        }
    }

    #[test]
    fn test_preview_extract_error_propagated() {
        struct FailBuffer;
        impl ArchiveRepository for FailBuffer {
            fn open(&self, _: &std::path::Path, _: Option<&Password>) -> Result<ArchiveHandle, ArchiveError> { Err(ArchiveError::UnsupportedOperation) }
            fn create(&self, _: &std::path::Path, _: ArchiveFormat, _: Option<&EncryptionConfig>) -> Result<ArchiveHandle, ArchiveError> { Err(ArchiveError::UnsupportedOperation) }
            fn list_page(&self, _: &ArchiveHandle, _: usize, _: usize) -> Result<Page<ArchiveEntry>, ArchiveError> { Ok(Page::new(vec![], 0, Some(0))) }
            fn get_properties(&self, _: &ArchiveHandle) -> Result<ArchiveProperties, ArchiveError> { Ok(ArchiveProperties::default()) }
            fn extract(&self, _: &ArchiveHandle, _: &[u32], _: &std::path::Path) -> Result<(), ArchiveError> { Err(ArchiveError::Internal("extract error".into())) }
            fn extract_to_buffer(&self, _: &ArchiveHandle, _: u32) -> Result<Vec<u8>, ArchiveError> { Err(ArchiveError::Internal("buffer error".into())) }
            fn add(&self, _: &mut ArchiveHandle, _: &[std::path::PathBuf], _: Option<&Password>) -> Result<(), ArchiveError> { Err(ArchiveError::UnsupportedOperation) }
            fn delete(&self, _: &mut ArchiveHandle, _: &[u32]) -> Result<(), ArchiveError> { Err(ArchiveError::UnsupportedOperation) }
            fn rename(&self, _: &mut ArchiveHandle, _: u32, _: &str) -> Result<(), ArchiveError> { Err(ArchiveError::UnsupportedOperation) }
            fn test(&self, _: &ArchiveHandle) -> Result<TestResult, ArchiveError> { Ok(TestResult { total: 0, passed: 0, failed: vec![] }) }
            fn list_directory(&self, _: &ArchiveHandle, _: &str) -> Result<Vec<ArchiveEntry>, ArchiveError> { Ok(vec![]) }
            fn close(&self, _: &ArchiveHandle) {}
        }
        let uc = PreviewEntryUseCase::new(Arc::new(FailBuffer));
        let handle = ArchiveHandle::new_reader();
        let result = uc.execute(&handle, 0, 4096);
        assert!(result.is_err());
    }
}
