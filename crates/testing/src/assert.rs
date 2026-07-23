use std::path::Path;
use bit7z_domain::vfs::SessionState;
use crate::harness::TestHarness;

/// Assert two SessionStates have identical tree structure + metadata.
pub fn assert_session_state_eq(expected: &SessionState, actual: &SessionState) {
    expected.vfs.assert_structural_eq(&actual.vfs);
}

/// Assert extracted files match ground truth by CRC.
pub fn assert_extraction_matches(ground_truth: &SessionState, extracted_root: &Path) {
    let tree = ground_truth.vfs.base_tree();
    for id in tree.all_ids() {
        let node = tree.node(id).unwrap();
        if node.is_directory {
            continue;
        }
        let path = tree.path_of(id).unwrap();
        let normalized = path.replace('/', std::path::MAIN_SEPARATOR_STR);
        let file_path = extracted_root.join(&normalized);
        assert!(
            file_path.exists(),
            "missing extracted file: {path} (expected at {})",
            file_path.display()
        );

        let meta = ground_truth
            .metadata_cache
            .get(&id)
            .expect("missing metadata");

        if let Some(expected_crc) = meta.crc {
            let actual = std::fs::read(&file_path)
                .unwrap_or_else(|e| panic!("cannot read {path}: {e}"));
            let actual_crc = crc32fast::hash(&actual);
            assert_eq!(
                expected_crc, actual_crc,
                "CRC mismatch for '{path}': expected 0x{expected_crc:08X}, got 0x{actual_crc:08X}",
            );
        }
    }
}

/// Run both harnesses on the same archive and assert they agree.
pub fn assert_harnesses_agree(
    harness_a: &dyn TestHarness,
    harness_b: &dyn TestHarness,
    archive: &Path,
    password: Option<&str>,
) {
    let state_a = harness_a
        .open_archive(archive, password)
        .expect("harness_a open_archive failed");
    let state_b = harness_b
        .open_archive(archive, password)
        .expect("harness_b open_archive failed");
    assert_session_state_eq(&state_a, &state_b);
}
