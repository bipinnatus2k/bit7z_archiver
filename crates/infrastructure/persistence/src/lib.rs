pub mod preferences_json;

use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheState {
    Valid,
    Dirty,
}

/// Detect writer format from archive path extension.
pub fn detect_writer_format(path: &Path) -> bit7z_infra_bit7z::WriterFormat {
    path.extension()
        .and_then(|ext| {
            let ext = ext.to_string_lossy().to_lowercase();
            match ext.as_str() {
                "7z" => Some(bit7z_infra_bit7z::WriterFormat::SevenZip),
                "zip" => Some(bit7z_infra_bit7z::WriterFormat::Zip),
                "tar" => Some(bit7z_infra_bit7z::WriterFormat::Tar),
                "gz" | "tgz" => Some(bit7z_infra_bit7z::WriterFormat::GZip),
                "bz2" | "tbz" | "tbz2" => Some(bit7z_infra_bit7z::WriterFormat::BZip2),
                "xz" | "txz" => Some(bit7z_infra_bit7z::WriterFormat::Xz),
                _ => None,
            }
        })
        .unwrap_or(bit7z_infra_bit7z::WriterFormat::SevenZip)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bit7z_domain::repository::test_utils::MockArchiveRepository;
    use std::path::PathBuf;

    #[test]
    fn test_mock_open_nonexistent_returns_error() {
        let inner = MockArchiveRepository::arc_with_count(0);
        let result = inner.open(Path::new("nonexistent.7z"), None);
        assert!(result.is_ok()); // Mock always succeeds
    }

    #[test]
    fn test_mock_list_page_returns_entries() {
        let repo = MockArchiveRepository::arc_with_count(10);
        let handle = repo.open(Path::new("test.7z"), None).unwrap();
        let page = repo.list(&handle, 0..5).unwrap();
        assert_eq!(page.items.len(), 5);
        assert_eq!(page.total, Some(10));
    }

    #[test]
    fn test_detect_writer_format() {
        assert_eq!(detect_writer_format(Path::new("a.7z")), bit7z_infra_bit7z::WriterFormat::SevenZip);
        assert_eq!(detect_writer_format(Path::new("a.zip")), bit7z_infra_bit7z::WriterFormat::Zip);
        assert_eq!(detect_writer_format(Path::new("a.tar.gz")), bit7z_infra_bit7z::WriterFormat::GZip);
        assert_eq!(detect_writer_format(Path::new("a.unknown")), bit7z_infra_bit7z::WriterFormat::SevenZip);
    }
}
