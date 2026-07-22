use bit7z_app_archive::runtime_service::ArchiveService;
use bit7z_domain::archive::ArchiveHandle;
use bit7z_domain::repository::ArchiveError;
use md5::{Digest, Md5};
use serde::Deserialize;
use sha1::Sha1;
use sha2::Sha256;
use std::sync::Arc;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Deserialize)]
pub enum ChecksumAlgorithm {
    Crc32,
    Md5,
    Sha1,
    Sha256,
}

pub struct ChecksumResult {
    pub path: String,
    pub crc32: Option<String>,
    pub md5: Option<String>,
    pub sha1: Option<String>,
    pub sha256: Option<String>,
}

pub struct CalculateChecksumUseCase {
    service: Arc<ArchiveService>,
}

impl CalculateChecksumUseCase {
    pub fn new(service: Arc<ArchiveService>) -> Self {
        Self { service }
    }

    pub fn execute(
        &self,
        archive: &ArchiveHandle,
        indices: &[u32],
        algorithms: &[ChecksumAlgorithm],
    ) -> Result<Vec<ChecksumResult>, ArchiveError> {
        let mut results = Vec::with_capacity(indices.len());

        for &index in indices {
            let page = match self.service.list_page(archive, index as usize, 1) {
                Ok(p) => p,
                Err(e) => return Err(e),
            };
            if page.items.is_empty() {
                return Err(ArchiveError::NotFound(format!("entry at index {}", index)));
            }
            let entry = &page.items[0];

            let mut result = ChecksumResult {
                path: entry.path.clone(),
                crc32: None,
                md5: None,
                sha1: None,
                sha256: None,
            };

            if entry.is_directory {
                results.push(result);
                continue;
            }

            let data = self.service.extract_to_buffer(archive, index)?;

            for algo in algorithms {
                match algo {
                    ChecksumAlgorithm::Crc32 => {
                        let hash = crc32fast::hash(&data);
                        result.crc32 = Some(format!("{:08x}", hash));
                    }
                    ChecksumAlgorithm::Md5 => {
                        let mut hasher = Md5::new();
                        hasher.update(&data);
                        result.md5 = Some(format!("{:x}", hasher.finalize()));
                    }
                    ChecksumAlgorithm::Sha1 => {
                        let mut hasher = Sha1::new();
                        hasher.update(&data);
                        result.sha1 = Some(format!("{:x}", hasher.finalize()));
                    }
                    ChecksumAlgorithm::Sha256 => {
                        let mut hasher = Sha256::new();
                        hasher.update(&data);
                        result.sha256 = Some(format!("{:x}", hasher.finalize()));
                    }
                }
            }

            results.push(result);
        }

        Ok(results)
    }
}
