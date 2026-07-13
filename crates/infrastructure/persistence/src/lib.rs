pub mod preferences_json;

#[cfg(test)]
mod tests {
    use super::*;
    use bit7z_domain::repository::test_utils::MockArchiveRepository;
    use std::path::Path;

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
}
