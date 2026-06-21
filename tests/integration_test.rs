mod common;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use bit7z_archiver::adapters::view_models::progress_vm::ProgressState;
use bit7z_archiver::application::progress::progress_channel;
use bit7z_archiver::application::test::TestEntriesUseCase;
use bit7z_archiver::domain::archive::*;
use bit7z_archiver::domain::repository::*;

type MockRepo = test_utils::MockArchiveRepository;

// ===== 9.2: Integration tests — Repository add/delete/rename =====

mod repo_operations {
    use super::*;

    #[test]
    fn test_add_files_increases_entry_count() {
        let repo = MockRepo::new(vec![
            ArchiveEntry {
                name: "existing.txt".into(),
                path: "existing.txt".into(),
                size: 100,
                compressed_size: 50,
                original_index: 0,
                ..Default::default()
            },
        ]);
        assert_eq!(repo.entry_count(), 1);

        let mut handle = repo.open(Path::new("test.7z"), None).unwrap();
        let result = repo.add(&mut handle, &[PathBuf::from("new1.txt"), PathBuf::from("new2.txt")], None);
        assert!(result.is_ok());
        assert_eq!(repo.entry_count(), 3);
    }

    #[test]
    fn test_delete_entries_decreases_entry_count() {
        let repo = MockRepo::new(vec![
            ArchiveEntry { name: "a.txt".into(), path: "a.txt".into(), original_index: 0, ..Default::default() },
            ArchiveEntry { name: "b.txt".into(), path: "b.txt".into(), original_index: 1, ..Default::default() },
            ArchiveEntry { name: "c.txt".into(), path: "c.txt".into(), original_index: 2, ..Default::default() },
        ]);
        assert_eq!(repo.entry_count(), 3);

        let mut handle = repo.open(Path::new("test.7z"), None).unwrap();
        let result = repo.delete(&mut handle, &[0, 2]);
        assert!(result.is_ok());
        assert_eq!(repo.entry_count(), 1);

        let page = repo.list_page(&handle, 0, 10).unwrap();
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].name, "b.txt");
        assert_eq!(page.items[0].original_index, 0);
    }

    #[test]
    fn test_rename_entry_changes_path() {
        let repo = MockRepo::new(vec![
            ArchiveEntry { name: "old.txt".into(), path: "old.txt".into(), original_index: 0, ..Default::default() },
        ]);

        let mut handle = repo.open(Path::new("test.7z"), None).unwrap();
        let result = repo.rename(&mut handle, 0, "new.txt");
        assert!(result.is_ok());

        let page = repo.list_page(&handle, 0, 10).unwrap();
        assert_eq!(page.items[0].name, "new.txt");
        assert_eq!(page.items[0].path, "new.txt");
    }

    #[test]
    fn test_rename_entry_in_subdirectory_preserves_path() {
        let repo = MockRepo::new(vec![
            ArchiveEntry { name: "old.txt".into(), path: "subdir/old.txt".into(), original_index: 0, ..Default::default() },
        ]);

        let mut handle = repo.open(Path::new("test.7z"), None).unwrap();
        let result = repo.rename(&mut handle, 0, "new.txt");
        assert!(result.is_ok());

        let page = repo.list_page(&handle, 0, 10).unwrap();
        assert_eq!(page.items[0].name, "new.txt");
        assert_eq!(page.items[0].path, "subdir/new.txt");
    }

    #[test]
    fn test_rename_nonexistent_entry_returns_error() {
        let repo = MockRepo::new(vec![
            ArchiveEntry { name: "f.txt".into(), path: "f.txt".into(), original_index: 0, ..Default::default() },
        ]);

        let mut handle = repo.open(Path::new("test.7z"), None).unwrap();
        let result = repo.rename(&mut handle, 99, "nope.txt");
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_nonexistent_indices_is_noop() {
        let repo = MockRepo::new(vec![
            ArchiveEntry { name: "a.txt".into(), path: "a.txt".into(), original_index: 0, ..Default::default() },
            ArchiveEntry { name: "b.txt".into(), path: "b.txt".into(), original_index: 1, ..Default::default() },
        ]);

        let mut handle = repo.open(Path::new("test.7z"), None).unwrap();
        let result = repo.delete(&mut handle, &[99]);
        assert!(result.is_ok());
        assert_eq!(repo.entry_count(), 2);
    }
}

// ===== 9.3: Integration tests — TestEntriesUseCase =====

mod test_entries {
    use super::*;

    fn make_repo_with_entries(entries: Vec<(u32, u32)>) -> Arc<dyn ArchiveRepository> {
        let entries: Vec<ArchiveEntry> = entries.into_iter().enumerate().map(|(i, (size, crc))| {
            ArchiveEntry {
                name: format!("file_{}.txt", i),
                path: format!("file_{}.txt", i),
                size: size as u64,
                compressed_size: (size as u64 / 2).max(1),
                crc: Some(crc),
                original_index: i as u32,
                ..Default::default()
            }
        }).collect();
        Arc::new(MockRepo::new(entries).with_extract_buffer(true))
    }

    /// Build a repo where stored CRCs match the deterministic mock data.
    fn make_repo_matching_crc(sizes: &[u32]) -> Arc<dyn ArchiveRepository> {
        let entries: Vec<(u32, u32)> = sizes.iter().enumerate().map(|(i, &size)| {
            let fill = (i as u8).wrapping_mul(17);
            let data = vec![fill; size.max(1) as usize];
            let crc = crc32fast::hash(&data);
            (size, crc)
        }).collect();
        make_repo_with_entries(entries)
    }

    #[test]
    fn test_all_entries_pass_with_matching_crc() {
        let repo = make_repo_matching_crc(&[4]);
        let handle = repo.open(Path::new("test.7z"), None).unwrap();
        let uc = TestEntriesUseCase::new(repo);
        let result = uc.execute(&handle, None, None).unwrap();
        assert_eq!(result.total, 1);
        assert_eq!(result.passed, 1);
        assert!(result.failed.is_empty());
    }

    #[test]
    fn test_entry_fails_with_mismatched_crc() {
        // Use a CRC that doesn't match the mock's deterministic data
        let bad_crc: u32 = 0xDEADBEEF;
        let repo = make_repo_with_entries(vec![(100, bad_crc)]);
        let handle = repo.open(Path::new("test.7z"), None).unwrap();
        let uc = TestEntriesUseCase::new(repo);
        let result = uc.execute(&handle, None, None).unwrap();
        assert_eq!(result.total, 1);
        assert_eq!(result.passed, 0);
        assert_eq!(result.failed.len(), 1);
        assert!(matches!(result.failed[0].reason, TestFailureReason::CrcMismatch { .. }));
    }

    #[test]
    fn test_directory_with_files_tests_all_children() {
        let fill1 = (1u8).wrapping_mul(17);
        let crc1 = crc32fast::hash(&vec![fill1; 4]);
        let fill2 = (2u8).wrapping_mul(17);
        let crc2 = crc32fast::hash(&vec![fill2; 4]);
        let repo = Arc::new(MockRepo::new(vec![
            ArchiveEntry {
                name: "dir".into(),
                path: "dir".into(),
                is_directory: true,
                size: 0,
                compressed_size: 0,
                original_index: 0,
                ..Default::default()
            },
            ArchiveEntry {
                name: "a.txt".into(),
                path: "dir/a.txt".into(),
                size: 4,
                compressed_size: 2,
                crc: Some(crc1),
                original_index: 1,
                ..Default::default()
            },
            ArchiveEntry {
                name: "b.txt".into(),
                path: "dir/b.txt".into(),
                size: 4,
                compressed_size: 2,
                crc: Some(crc2),
                original_index: 2,
                ..Default::default()
            },
        ]).with_extract_buffer(true));
        let handle = repo.open(Path::new("test.7z"), None).unwrap();
        let uc = TestEntriesUseCase::new(repo);
        let result = uc.execute(&handle, None, None).unwrap();
        assert_eq!(result.total, 2);
        assert_eq!(result.passed, 2);
        assert!(result.failed.is_empty());
    }

    #[test]
    fn test_empty_directory_passes() {
        let repo = Arc::new(MockRepo::new(vec![
            ArchiveEntry {
                name: "emptydir".into(),
                path: "emptydir".into(),
                is_directory: true,
                size: 0,
                compressed_size: 0,
                original_index: 0,
                ..Default::default()
            },
        ]).with_extract_buffer(true));
        let handle = repo.open(Path::new("test.7z"), None).unwrap();
        let uc = TestEntriesUseCase::new(repo);
        let result = uc.execute(&handle, None, None).unwrap();
        assert_eq!(result.total, 1);
        assert_eq!(result.passed, 1);
        assert!(result.failed.is_empty());
    }

    #[test]
    fn test_with_specific_indices() {
        let repo = make_repo_matching_crc(&[4, 4, 4]);
        let handle = repo.open(Path::new("test.7z"), None).unwrap();
        let uc = TestEntriesUseCase::new(repo);
        let result = uc.execute(&handle, Some(&[0, 2]), None).unwrap();
        assert_eq!(result.total, 2);
        assert_eq!(result.passed, 2);
    }

    #[test]
    fn test_multiple_entries_mixed_results() {
        let fill0 = (0u8).wrapping_mul(17);
        let good_crc0 = crc32fast::hash(&vec![fill0; 4]);
        let bad_crc: u32 = 0xBADBAD;
        let fill2 = (2u8).wrapping_mul(17);
        let good_crc2 = crc32fast::hash(&vec![fill2; 4]);

        let entries = vec![
            (4u32, good_crc0),
            (100u32, bad_crc),
            (4u32, good_crc2),
        ];
        let repo = make_repo_with_entries(entries);
        let handle = repo.open(Path::new("test.7z"), None).unwrap();
        let uc = TestEntriesUseCase::new(repo);
        let result = uc.execute(&handle, None, None).unwrap();
        assert_eq!(result.total, 3);
        assert_eq!(result.passed, 2);
        assert_eq!(result.failed.len(), 1);
        assert_eq!(result.failed[0].index, 1);
    }
}

// ===== 9.4: Integration tests — Compress CLI round-trip =====

mod compress_cli {
    use super::*;

    #[test]
    fn test_create_and_list_round_trip() {
        let repo = Arc::new(MockRepo::new(vec![]));

        let dest = PathBuf::from("test_out.7z");
        let mut handle = repo.create(&dest, ArchiveFormat::SevenZip, None).unwrap();

        let files = vec![
            PathBuf::from("a.txt"),
            PathBuf::from("b.txt"),
            PathBuf::from("sub/c.txt"),
        ];
        let result = repo.add(&mut handle, &files, None);
        assert!(result.is_ok());

        let page = repo.list_page(&handle, 0, 10).unwrap();
        assert_eq!(page.items.len(), 3);
        assert_eq!(page.total, Some(3));
    }

    #[test]
    fn test_create_then_get_properties() {
        let repo2 = Arc::new(MockRepo::new(vec![
            ArchiveEntry { name: "f1".into(), path: "f1".into(), size: 100, compressed_size: 50, original_index: 0, ..Default::default() },
            ArchiveEntry { name: "f2".into(), path: "f2".into(), size: 200, compressed_size: 100, original_index: 1, ..Default::default() },
        ]));
        let h = repo2.open(Path::new("p.7z"), None).unwrap();
        let props = repo2.get_properties(&h).unwrap();
        assert_eq!(props.items_count, 2);
        assert_eq!(props.files_count, 2);
        assert_eq!(props.folders_count, 0);
        assert_eq!(props.total_size, 300);
        assert_eq!(props.packed_size, 150);
    }

    #[test]
    fn test_extract_is_ok_with_mock() {
        let repo = Arc::new(MockRepo::new(vec![
            ArchiveEntry { name: "f.txt".into(), path: "f.txt".into(), original_index: 0, ..Default::default() },
        ]));
        let handle = repo.open(Path::new("test.7z"), None).unwrap();

        let dest = common::temp_dir();
        let result = repo.extract(&handle, &[0], &dest);
        assert!(result.is_ok());
    }
}

// ===== 9.5: Unit tests — ProgressState =====

mod progress_state {
    use super::*;

    #[test]
    fn test_default_progress_state_is_not_active() {
        let state = ProgressState::default();
        assert!(!state.is_active);
        assert!(!state.is_complete);
        assert!(!state.is_paused);
        assert!(state.error.is_none());
        assert_eq!(state.current, 0);
        assert_eq!(state.total, 0);
    }

    #[test]
    fn test_poll_updates_fields_from_channel() {
        let mut state = ProgressState::default();
        let (tx, rx) = progress_channel();

        state.is_active = true;
        state.receiver = Some(std::sync::Arc::new(std::sync::Mutex::new(rx)));

        let update = ProgressUpdate {
            file_current: 3,
            file_total: 10,
            current_file: Some("test.txt".into()),
            items_done: 5,
            items_total: 20,
            bytes_done: 1024,
            bytes_total: 4096,
            error: None,
        };
        tx.send(update).unwrap();
        drop(tx);

        let updated = state.poll();
        assert!(updated);
        assert_eq!(state.file_current, 3);
        assert_eq!(state.file_total, 10);
        assert_eq!(state.current_file, Some("test.txt".into()));
        assert_eq!(state.items_done, 5);
        assert_eq!(state.items_total, 20);
        assert_eq!(state.bytes_done, 1024);
        assert_eq!(state.bytes_total, 4096);
        assert_eq!(state.current, 1024);
        assert_eq!(state.total, 4096);
    }

    #[test]
    fn test_is_complete_after_sender_drops() {
        let mut state = ProgressState::default();
        let (tx, rx) = progress_channel();

        state.is_active = true;
        state.receiver = Some(std::sync::Arc::new(std::sync::Mutex::new(rx)));
        drop(tx);

        let updated = state.poll();
        assert!(updated);
        assert!(!state.is_active);
        assert!(state.is_complete);
    }

    #[test]
    fn test_pause_prevents_poll_from_updating() {
        let mut state = ProgressState::default();
        let (tx, rx) = progress_channel();

        state.is_active = true;
        state.is_paused = true;
        state.receiver = Some(std::sync::Arc::new(std::sync::Mutex::new(rx)));

        let update = ProgressUpdate {
            file_current: 1,
            file_total: 5,
            current_file: None,
            items_done: 1,
            items_total: 5,
            bytes_done: 100,
            bytes_total: 500,
            error: None,
        };
        tx.send(update).unwrap();

        let updated = state.poll();
        assert!(!updated);
        assert_eq!(state.file_current, 0);
    }

    #[test]
    fn test_error_capture_via_poll() {
        let mut state = ProgressState::default();
        let (tx, rx) = progress_channel();

        state.is_active = true;
        state.receiver = Some(std::sync::Arc::new(std::sync::Mutex::new(rx)));

        let update = ProgressUpdate {
            file_current: 0,
            file_total: 1,
            current_file: None,
            items_done: 0,
            items_total: 1,
            bytes_done: 0,
            bytes_total: 0,
            error: Some("CRC mismatch".into()),
        };
        tx.send(update).unwrap();
        drop(tx);

        state.poll();
        assert_eq!(state.error, Some("CRC mismatch".into()));
    }

    #[test]
    fn test_percent_calculation() {
        let state = ProgressState {
            current: 50,
            total: 100,
            ..Default::default()
        };
        assert!((state.percent() - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_percent_zero_total() {
        let state = ProgressState::default();
        assert_eq!(state.percent(), 0.0);
    }
}

// ===== 9.6: Unit tests — TestResult =====

mod test_result_logic {
    use super::*;

    #[test]
    fn test_all_passed() {
        let result = TestResult { total: 10, passed: 10, failed: vec![] };
        assert_eq!(result.total, 10);
        assert_eq!(result.passed, 10);
        assert!(result.failed.is_empty());
    }

    #[test]
    fn test_all_failed() {
        let failures: Vec<TestFailure> = (0..5).map(|i| TestFailure {
            entry_path: format!("file_{}.txt", i),
            error: "CRC mismatch".into(),
            index: i,
            path: format!("file_{}.txt", i),
            reason: TestFailureReason::CrcMismatch { expected: i as u32, actual: (i + 1) as u32 },
        }).collect();
        let result = TestResult { total: 5, passed: 0, failed: failures };
        assert_eq!(result.total, 5);
        assert_eq!(result.passed, 0);
        assert_eq!(result.failed.len(), 5);
        assert_eq!(result.failed[0].index, 0);
    }

    #[test]
    fn test_mixed_results_passed_equals_total_minus_failed() {
        let failures = vec![
            TestFailure {
                entry_path: "bad1.txt".into(),
                error: "read error".into(),
                index: 1,
                path: "bad1.txt".into(),
                reason: TestFailureReason::ReadError("read error".into()),
            },
        ];
        let result = TestResult { total: 5, passed: 4, failed: failures };
        assert_eq!(result.passed, result.total - result.failed.len());
    }

    #[test]
    fn test_empty_result() {
        let result = TestResult { total: 0, passed: 0, failed: vec![] };
        assert_eq!(result.total, 0);
        assert_eq!(result.passed, 0);
        assert!(result.failed.is_empty());
    }
}

// ===== 9.7: Unit tests — ArchiveProperties =====

mod archive_properties {
    use super::*;

    #[test]
    fn test_default_properties_are_zero() {
        let props = ArchiveProperties::default();
        assert_eq!(props.items_count, 0);
        assert_eq!(props.folders_count, 0);
        assert_eq!(props.files_count, 0);
        assert_eq!(props.total_size, 0);
        assert_eq!(props.packed_size, 0);
        assert!(!props.is_encrypted);
        assert!(!props.has_encrypted_items);
        assert!(!props.is_multi_volume);
        assert!(!props.is_solid);
        assert!(!props.encrypted_names);
        assert!(!props.has_comment);
        assert_eq!(props.comment_size, None);
        assert!(!props.has_recovery_record);
        assert!(!props.locked);
        assert_eq!(props.dictionary_size, None);
    }

    #[test]
    fn test_custom_properties() {
        let props = ArchiveProperties {
            items_count: 42,
            folders_count: 5,
            files_count: 37,
            total_size: 10240,
            packed_size: 5120,
            is_encrypted: true,
            has_encrypted_items: true,
            is_multi_volume: false,
            is_solid: true,
            encrypted_names: true,
            has_comment: true,
            comment_size: Some(128),
            has_recovery_record: false,
            locked: true,
            dictionary_size: Some(65536),
        };
        assert_eq!(props.items_count, 42);
        assert_eq!(props.folders_count, 5);
        assert_eq!(props.files_count, 37);
        assert!(props.is_encrypted);
        assert!(props.is_solid);
        assert!(props.locked);
        assert_eq!(props.dictionary_size, Some(65536));
    }

    #[test]
    fn test_properties_files_plus_folders_equals_items() {
        let props = ArchiveProperties {
            items_count: 100,
            folders_count: 10,
            files_count: 90,
            ..Default::default()
        };
        assert_eq!(props.files_count + props.folders_count, props.items_count);
    }
}
