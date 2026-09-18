
use std::sync::Arc;
use bit7z_domain::archive::error::ArchiveError;
use bit7z_domain::archive::handle::ArchiveHandle;
use crate::runtime_service::ArchiveService;

pub enum PreviewData {
    Text(String),
    // Hex(Vec<u8>),
    Image(Vec<u8>),
    Unsupported(String),
}

pub struct PreviewEntryUseCase {
    service: Arc<ArchiveService>,
}

impl PreviewEntryUseCase {
    pub fn new(service: Arc<ArchiveService>) -> Self {
        Self { service }
    }

    pub fn execute(
        &self,
        archive: &ArchiveHandle,
        index: u32,
        max_bytes: usize,
    ) -> Result<PreviewData, ArchiveError> {
        let bytes = self.service.extract_to_buffer(archive, index)?;
        let truncated: Vec<u8> = bytes.into_iter().take(max_bytes).collect();

        // Detect image by magic bytes
        if truncated.len() >= 4 {
            if &truncated[..4] == b"\x89PNG" || &truncated[..3] == b"\xFF\xD8\xFF" {
                return Ok(PreviewData::Image(truncated));
            }
        }

        // Try UTF-8 text
        if let Ok(text) = String::from_utf8(truncated.clone()) {
            if text
                .chars()
                .all(|c| c.is_ascii_graphic() || c.is_ascii_whitespace())
                || truncated.len() < 1024
            {
                return Ok(PreviewData::Text(text));
            }
        }

        // Fall back to hex
        Ok(PreviewData::Unsupported("".into()))
    }
}
