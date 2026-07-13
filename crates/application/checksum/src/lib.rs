use bit7z_domain::archive::*;
use bit7z_domain::repository::*;
use md5::{Digest, Md5};
use sha1::Sha1;
use sha2::Sha256;
use std::sync::Arc;
use serde::Deserialize;

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
    repo: Arc<dyn ArchiveRepository>,
}

impl CalculateChecksumUseCase {
    pub fn new(repo: Arc<dyn ArchiveRepository>) -> Self {
        Self { repo }
    }

    pub fn execute(
        &self,
        archive: &ArchiveHandle,
        indices: &[u32],
        algorithms: &[ChecksumAlgorithm],
    ) -> Result<Vec<ChecksumResult>, ArchiveError> {
        let mut results = Vec::with_capacity(indices.len());

        for &index in indices {
            let page = match self.repo.list_page(archive, index as usize, 1) {
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

            let data = self.repo.extract_to_buffer(archive, index)?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use bit7z_domain::repository::test_utils::MockArchiveRepository;
    use std::sync::Arc;

    #[test]
    fn test_checksum_crc32() {
        let mock = MockArchiveRepository::new(vec![
            ArchiveEntry { name: "a.txt".into(), path: "a.txt".into(), size: 100, original_index: 0, ..Default::default() },
        ]).with_extract_buffer(true);
        let repo: Arc<dyn ArchiveRepository> = Arc::new(mock);
        let uc = CalculateChecksumUseCase::new(repo);
        let handle = ArchiveHandle::new_reader();
        let results = uc.execute(&handle, &[0], &[ChecksumAlgorithm::Crc32]).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].path, "a.txt");
        assert!(results[0].crc32.is_some());
        assert!(results[0].md5.is_none());
    }

    #[test]
    fn test_checksum_md5() {
        let mock = MockArchiveRepository::new(vec![
            ArchiveEntry { name: "b.bin".into(), path: "b.bin".into(), size: 50, original_index: 0, ..Default::default() },
        ]).with_extract_buffer(true);
        let repo: Arc<dyn ArchiveRepository> = Arc::new(mock);
        let uc = CalculateChecksumUseCase::new(repo);
        let handle = ArchiveHandle::new_reader();
        let results = uc.execute(&handle, &[0], &[ChecksumAlgorithm::Md5]).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].md5.is_some());
    }

    #[test]
    fn test_checksum_multiple_algorithms() {
        let mock = MockArchiveRepository::new(vec![
            ArchiveEntry { name: "c.txt".into(), path: "c.txt".into(), size: 30, original_index: 0, ..Default::default() },
        ]).with_extract_buffer(true);
        let repo: Arc<dyn ArchiveRepository> = Arc::new(mock);
        let uc = CalculateChecksumUseCase::new(repo);
        let handle = ArchiveHandle::new_reader();
        let results = uc.execute(&handle, &[0], &[ChecksumAlgorithm::Crc32, ChecksumAlgorithm::Sha256]).unwrap();
        assert!(results[0].crc32.is_some());
        assert!(results[0].sha256.is_some());
        assert!(results[0].md5.is_none());
    }

    #[test]
    fn test_checksum_directory_skipped() {
        let mock = MockArchiveRepository::new(vec![
            ArchiveEntry { name: "dir".into(), path: "dir".into(), is_directory: true, original_index: 0, ..Default::default() },
        ]).with_extract_buffer(true);
        let repo: Arc<dyn ArchiveRepository> = Arc::new(mock);
        let uc = CalculateChecksumUseCase::new(repo);
        let handle = ArchiveHandle::new_reader();
        let results = uc.execute(&handle, &[0], &[ChecksumAlgorithm::Crc32]).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].crc32.is_none());
        assert!(results[0].md5.is_none());
    }

    #[test]
    fn test_checksum_multiple_indices() {
        let mock = MockArchiveRepository::new(vec![
            ArchiveEntry { name: "f1.txt".into(), path: "f1.txt".into(), size: 10, original_index: 0, ..Default::default() },
            ArchiveEntry { name: "f2.txt".into(), path: "f2.txt".into(), size: 20, original_index: 1, ..Default::default() },
        ]).with_extract_buffer(true);
        let repo: Arc<dyn ArchiveRepository> = Arc::new(mock);
        let uc = CalculateChecksumUseCase::new(repo);
        let handle = ArchiveHandle::new_reader();
        let results = uc.execute(&handle, &[0, 1], &[ChecksumAlgorithm::Crc32]).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].path, "f1.txt");
        assert_eq!(results[1].path, "f2.txt");
    }

    #[test]
    fn test_checksum_not_found() {
        let repo = MockArchiveRepository::arc_with_count(3);
        let uc = CalculateChecksumUseCase::new(repo);
        let handle = ArchiveHandle::new_reader();
        let result = uc.execute(&handle, &[99], &[ChecksumAlgorithm::Crc32]);
        assert!(matches!(result, Err(ArchiveError::NotFound(_))));
    }
}
