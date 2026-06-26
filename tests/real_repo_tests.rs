mod common;

use bit7z_archiver::adapters::bit7z;
use bit7z_archiver::adapters::platform;
use bit7z_archiver::adapters::repository::Bit7zRepository;
use bit7z_archiver::domain::archive::*;
use bit7z_archiver::domain::repository::*;
use std::path::Path;
use std::sync::Arc;

/// Helper: check if the real 7z DLL is available on this system.
fn has_7z_library() -> bool {
    platform::find_7z_library().is_some()
}

/// Helper: check that all fixture files exist.
fn all_fixtures_exist() -> bool {
    common::ensure_fixtures().is_empty()
}

/// Helper: create a Bit7zRepository backed by the real 7-Zip library.
fn create_repo() -> Option<Arc<dyn ArchiveRepository>> {
    let lib_path = platform::find_7z_library()?;
    let lib = bit7z::Library::open(lib_path.to_str()?).ok()?;
    Some(Arc::new(Bit7zRepository::new(lib)))
}

// ============================================================================
// Basic open & list tests for each archive format
// ============================================================================

mod open_and_list {
    use super::*;

    #[test]
    fn test_open_basic_7z_and_list_entries() {
        if !has_7z_library() || !all_fixtures_exist() {
            return;
        }
        let repo = create_repo().unwrap();
        let handle = repo
            .open(&common::fixture_path("basic.7z"), None)
            .expect("Failed to open basic.7z");

        let page = repo.list_page(&handle, 0, 100).unwrap();
        assert_eq!(page.items.len(), 4);

        let names: Vec<&str> = page.items.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"f1.txt"));
        assert!(names.contains(&"f2.txt"));
        assert!(names.contains(&"sub"));
        assert!(names.contains(&"f3.txt"));

        // Verify the subdirectory entry
        let sub = page.items.iter().find(|e| e.name == "sub").unwrap();
        assert!(sub.is_directory);

        // Verify file sizes
        let f1 = page.items.iter().find(|e| e.name == "f1.txt").unwrap();
        assert_eq!(f1.size, 21);

        let f3 = page.items.iter().find(|e| e.name == "f3.txt").unwrap();
        assert_eq!(f3.size, 32);

        repo.close(handle);
    }

    #[test]
    fn test_open_basic_zip_and_list_entries() {
        if !has_7z_library() || !all_fixtures_exist() {
            return;
        }
        let repo = create_repo().unwrap();
        let handle = repo
            .open(&common::fixture_path("basic.zip"), None)
            .expect("Failed to open basic.zip");

        let page = repo.list_page(&handle, 0, 100).unwrap();
        assert_eq!(page.items.len(), 4);

        let names: Vec<&str> = page.items.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"f1.txt"));
        assert!(names.contains(&"f2.txt"));
        assert!(names.contains(&"sub"));
        assert!(names.contains(&"f3.txt"));

        repo.close(handle);
    }

    #[test]
    fn test_open_basic_tar_and_list_entries() {
        if !has_7z_library() || !all_fixtures_exist() {
            return;
        }
        let repo = create_repo().unwrap();
        let handle = repo
            .open(&common::fixture_path("basic.tar"), None)
            .expect("Failed to open basic.tar");

        let page = repo.list_page(&handle, 0, 100).unwrap();
        assert_eq!(page.items.len(), 4);

        let names: Vec<&str> = page.items.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"f1.txt"));
        assert!(names.contains(&"f2.txt"));
        assert!(names.contains(&"sub"));
        assert!(names.contains(&"f3.txt"));

        repo.close(handle);
    }

    #[test]
    fn test_open_empty_archive() {
        if !has_7z_library() || !all_fixtures_exist() {
            return;
        }
        let repo = create_repo().unwrap();
        let handle = repo
            .open(&common::fixture_path("empty.7z"), None)
            .expect("Failed to open empty.7z");

        let page = repo.list_page(&handle, 0, 10).unwrap();
        assert_eq!(page.items.len(), 0);
        assert_eq!(page.total, Some(0));

        let props = repo.get_properties(&handle).unwrap();
        assert_eq!(props.items_count, 0);
        assert_eq!(props.files_count, 0);

        repo.close(handle);
    }
}

// ============================================================================
// Get properties tests
// ============================================================================

mod properties {
    use super::*;

    #[test]
    fn test_properties_basic_7z() {
        if !has_7z_library() || !all_fixtures_exist() {
            return;
        }
        let repo = create_repo().unwrap();
        let handle = repo
            .open(&common::fixture_path("basic.7z"), None)
            .expect("Failed to open basic.7z");

        let props = repo.get_properties(&handle).unwrap();
        assert_eq!(props.items_count, 4);
        assert_eq!(props.files_count, 3);
        assert_eq!(props.folders_count, 1);
        assert!(!props.is_encrypted);
        assert!(!props.has_encrypted_items);

        repo.close(handle);
    }

    #[test]
    fn test_properties_multi_file() {
        if !has_7z_library() || !all_fixtures_exist() {
            return;
        }
        let repo = create_repo().unwrap();
        let handle = repo
            .open(&common::fixture_path("multi_file.7z"), None)
            .expect("Failed to open multi_file.7z");

        let props = repo.get_properties(&handle).unwrap();
        assert_eq!(props.items_count, 4);
        assert_eq!(props.files_count, 4);
        assert_eq!(props.folders_count, 0);

        // large.txt is 10002 bytes
        let total_expected = 7u64 + 6 + 19 + 10002;
        assert_eq!(props.total_size, total_expected);
        assert!(props.packed_size > 0);
        assert!(props.packed_size <= props.total_size);

        repo.close(handle);
    }

    #[test]
    fn test_properties_encrypted_archive() {
        if !has_7z_library() || !all_fixtures_exist() {
            return;
        }
        let pw = Password::new("secret123");
        let repo = create_repo().unwrap();
        let handle = repo
            .open(&common::fixture_path("encrypted.7z"), Some(&pw))
            .expect("Failed to open encrypted.7z with password");

        let props = repo.get_properties(&handle).unwrap();
        assert!(props.is_encrypted);
        assert!(props.has_encrypted_items);

        repo.close(handle);
    }
}

// ============================================================================
// Extraction tests
// ============================================================================

mod extract {
    use super::*;

    #[test]
    fn test_extract_single_file_to_disk() {
        if !has_7z_library() || !all_fixtures_exist() {
            return;
        }
        let repo = create_repo().unwrap();
        let handle = repo
            .open(&common::fixture_path("basic.7z"), None)
            .expect("Failed to open basic.7z");

        // Find f1.txt index
        let page = repo.list_page(&handle, 0, 100).unwrap();
        let f1 = page.items.iter().find(|e| e.name == "f1.txt").unwrap();
        let f1_index = f1.original_index;

        let dest = common::temp_dir();
        repo.extract(&handle, &[f1_index], &dest)
            .expect("Failed to extract f1.txt");

        let extracted_path = dest.join("f1.txt");
        assert!(extracted_path.exists(), "Extracted file should exist");

        let content = std::fs::read_to_string(&extracted_path).unwrap();
        assert_eq!(content.trim_end(), "hello world from f1");

        repo.close(handle);
    }

    #[test]
    fn test_extract_all_files_to_disk() {
        if !has_7z_library() || !all_fixtures_exist() {
            return;
        }
        let repo = create_repo().unwrap();
        let handle = repo
            .open(&common::fixture_path("basic.7z"), None)
            .expect("Failed to open basic.7z");

        let page = repo.list_page(&handle, 0, 100).unwrap();
        let file_indices: Vec<u32> = page
            .items
            .iter()
            .filter(|e| !e.is_directory)
            .map(|e| e.original_index)
            .collect();

        let dest = common::temp_dir();
        repo.extract(&handle, &file_indices, &dest)
            .expect("Failed to extract all files");

        assert!(dest.join("f1.txt").exists());
        assert!(dest.join("f2.txt").exists());
        assert!(dest.join("sub").join("f3.txt").exists());

        repo.close(handle);
    }

    #[test]
    fn test_extract_to_buffer() {
        if !has_7z_library() || !all_fixtures_exist() {
            return;
        }
        let repo = create_repo().unwrap();
        let handle = repo
            .open(&common::fixture_path("basic.7z"), None)
            .expect("Failed to open basic.7z");

        let page = repo.list_page(&handle, 0, 100).unwrap();
        let f1 = page.items.iter().find(|e| e.name == "f1.txt").unwrap();

        let buffer = repo
            .extract_to_buffer(&handle, f1.original_index)
            .expect("Failed to extract to buffer");

        let content = String::from_utf8(buffer).unwrap();
        assert_eq!(content.trim_end(), "hello world from f1");

        repo.close(handle);
    }

    #[test]
    fn test_extract_from_encrypted_archive() {
        if !has_7z_library() || !all_fixtures_exist() {
            return;
        }
        let pw = Password::new("secret123");
        let repo = create_repo().unwrap();
        let handle = repo
            .open(&common::fixture_path("encrypted.7z"), Some(&pw))
            .expect("Failed to open encrypted.7z");

        let page = repo.list_page(&handle, 0, 100).unwrap();
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].name, "f1.txt");
        assert!(page.items[0].is_encrypted);

        let buffer = repo
            .extract_to_buffer(&handle, page.items[0].original_index)
            .expect("Failed to extract from encrypted archive");

        let content = String::from_utf8(buffer).unwrap();
        assert_eq!(content.trim_end(), "hello world from f1");

        repo.close(handle);
    }
}

// ============================================================================
// Integrity test (test operation)
// ============================================================================

mod test_integrity {
    use super::*;

    #[test]
    fn test_integrity_of_valid_archive() {
        if !has_7z_library() || !all_fixtures_exist() {
            return;
        }
        let repo = create_repo().unwrap();
        let handle = repo
            .open(&common::fixture_path("basic.7z"), None)
            .expect("Failed to open basic.7z");

        let result = repo.test(&handle).expect("Test should succeed");
        assert_eq!(result.total, 4);
        assert_eq!(result.passed, 4);
        assert!(result.failed.is_empty());

        repo.close(handle);
    }

    #[test]
    fn test_integrity_of_empty_archive() {
        if !has_7z_library() || !all_fixtures_exist() {
            return;
        }
        let repo = create_repo().unwrap();
        let handle = repo
            .open(&common::fixture_path("empty.7z"), None)
            .expect("Failed to open empty.7z");

        let result = repo.test(&handle).expect("Test should succeed");
        assert_eq!(result.total, 0);
        assert_eq!(result.passed, 0);
        assert!(result.failed.is_empty());

        repo.close(handle);
    }

    #[test]
    fn test_integrity_of_encrypted_archive() {
        if !has_7z_library() || !all_fixtures_exist() {
            return;
        }
        let pw = Password::new("secret123");
        let repo = create_repo().unwrap();
        let handle = repo
            .open(&common::fixture_path("encrypted.7z"), Some(&pw))
            .expect("Failed to open encrypted.7z");

        let result = repo.test(&handle).expect("Test should succeed");
        assert_eq!(result.total, 1);
        assert_eq!(result.passed, 1);
        assert!(result.failed.is_empty());

        repo.close(handle);
    }

    #[test]
    fn test_integrity_of_corrupted_archive() {
        if !has_7z_library() || !all_fixtures_exist() {
            return;
        }
        let repo = create_repo().unwrap();
        let result = repo.open(&common::fixture_path("corrupted.7z"), None);

        match result {
            Ok(handle) => {
                let test_result = repo.test(&handle).expect("Test should succeed");
                assert_eq!(test_result.total, 2);
                assert!(
                    test_result.passed < 2 || !test_result.failed.is_empty(),
                    "Corrupted archive should have at least one failed entry"
                );
                repo.close(handle);
            }
            Err(e) => {
                // Opening a corrupted archive may fail — that itself proves integrity detection
                eprintln!("Corrupted archive open failed as expected: {:?}", e);
            }
        }
    }
}

// ============================================================================
// List directory tests
// ============================================================================

mod list_directory {
    use super::*;

    #[test]
    fn test_list_directory_root() {
        if !has_7z_library() || !all_fixtures_exist() {
            return;
        }
        let repo = create_repo().unwrap();
        let handle = repo
            .open(&common::fixture_path("basic.7z"), None)
            .expect("Failed to open basic.7z");

        let root = repo.list_directory(&handle, "").unwrap();
        // Root should show: f1.txt, f2.txt, sub (3 items)
        assert_eq!(root.len(), 3);
        assert!(root.iter().any(|e| e.name == "sub" && e.is_directory));
        assert!(root.iter().any(|e| e.name == "f1.txt" && !e.is_directory));
        assert!(root.iter().any(|e| e.name == "f2.txt" && !e.is_directory));

        repo.close(handle);
    }

    #[test]
    fn test_list_directory_subdir() {
        if !has_7z_library() || !all_fixtures_exist() {
            return;
        }
        let repo = create_repo().unwrap();
        let handle = repo
            .open(&common::fixture_path("basic.7z"), None)
            .expect("Failed to open basic.7z");

        // NOTE: C++ implementation requires trailing slash for subdirectory paths
        let children = repo.list_directory(&handle, "sub/").unwrap();
        assert_eq!(children.len(), 1, "Expected 1 child in 'sub/' directory");
        assert_eq!(children[0].name, "f3.txt");

        repo.close(handle);
    }

    #[test]
    fn test_list_directory_nonexistent_returns_not_found_or_empty() {
        if !has_7z_library() || !all_fixtures_exist() {
            return;
        }
        let repo = create_repo().unwrap();
        let handle = repo
            .open(&common::fixture_path("basic.7z"), None)
            .expect("Failed to open basic.7z");

        // The C++ implementation may either return NotFound or an empty vec
        let result = repo.list_directory(&handle, "nonexistent");
        match result {
            Ok(entries) => assert_eq!(entries.len(), 0),
            Err(ArchiveError::NotFound(_)) => {},
            Err(e) => panic!("Unexpected error: {:?}", e),
        }

        repo.close(handle);
    }
}

// ============================================================================
// Error handling tests
// ============================================================================

mod error_handling {
    use super::*;

    #[test]
    fn test_open_nonexistent_file_returns_error() {
        if !has_7z_library() {
            return;
        }
        let repo = create_repo().unwrap();
        let result = repo.open(Path::new("Z:\\nonexistent_archive.xyz"), None);
        assert!(result.is_err());
    }

    #[test]
    fn test_open_encrypted_without_password_returns_appropriate_error() {
        if !has_7z_library() || !all_fixtures_exist() {
            return;
        }
        let repo = create_repo().unwrap();

        // The encrypted.7z has header encryption (mhe=on),
        // so opening without password should fail.
        let result = repo.open(&common::fixture_path("encrypted.7z"), None);
        assert!(
            matches!(result, Err(ArchiveError::EncryptedArchiveRequiresPassword)),
            "Expected EncryptedArchiveRequiresPassword, got {:?}",
            result
        );
    }

    #[test]
    fn test_open_encrypted_with_wrong_password_returns_error() {
        if !has_7z_library() || !all_fixtures_exist() {
            return;
        }
        let wrong_pw = Password::new("wrong_password");
        let repo = create_repo().unwrap();

        // With wrong password, opening an encrypted archive should fail.
        let result = repo.open(&common::fixture_path("encrypted.7z"), Some(&wrong_pw));
        assert!(
            result.is_err(),
            "Opening encrypted archive with wrong password should fail"
        );
    }

    #[test]
    fn test_extract_nonexistent_index_returns_error() {
        if !has_7z_library() || !all_fixtures_exist() {
            return;
        }
        let repo = create_repo().unwrap();
        let handle = repo
            .open(&common::fixture_path("basic.7z"), None)
            .expect("Failed to open basic.7z");

        let dest = common::temp_dir();
        let result = repo.extract(&handle, &[999], &dest);
        assert!(result.is_err());

        repo.close(handle);
    }
}

// ============================================================================
// Pagination tests
// ============================================================================

mod pagination {
    use super::*;

    #[test]
    fn test_list_page_offset_and_limit() {
        if !has_7z_library() || !all_fixtures_exist() {
            return;
        }
        let repo = create_repo().unwrap();
        let handle = repo
            .open(&common::fixture_path("multi_file.7z"), None)
            .expect("Failed to open multi_file.7z");

        // First page: 2 items
        let page1 = repo.list_page(&handle, 0, 2).unwrap();
        assert_eq!(page1.items.len(), 2);
        assert_eq!(page1.total, Some(4));

        // Second page: 2 items
        let page2 = repo.list_page(&handle, 2, 2).unwrap();
        assert_eq!(page2.items.len(), 2);
        assert_eq!(page2.total, Some(4));

        // Offset beyond total
        let page3 = repo.list_page(&handle, 10, 5).unwrap();
        assert_eq!(page3.items.len(), 0);

        repo.close(handle);
    }
}

// ============================================================================
// Compression ratio tests
// ============================================================================

mod compression_ratio {
    use super::*;

    #[test]
    fn test_entry_compression_ratio_from_real_archive() {
        if !has_7z_library() || !all_fixtures_exist() {
            return;
        }
        let repo = create_repo().unwrap();
        let handle = repo
            .open(&common::fixture_path("multi_file.7z"), None)
            .expect("Failed to open multi_file.7z");

        let page = repo.list_page(&handle, 0, 100).unwrap();

        // Each entry should have a valid compression ratio
        for entry in &page.items {
            let ratio = entry.compression_ratio();
            assert!(ratio >= 0.0, "Ratio should not be negative");
            assert!(ratio <= 1.0, "Ratio should not exceed 1.0");
        }

        repo.close(handle);
    }
}
