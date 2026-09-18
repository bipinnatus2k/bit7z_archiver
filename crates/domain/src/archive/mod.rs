
pub(crate) mod archive_entry;
pub(crate) mod archive_format;
pub(crate) mod change;
pub(crate) mod create;
pub mod encryption;
pub mod handle;
pub mod overwrite_mode;
pub mod session;
pub mod test;
pub mod archive_properties;
pub mod error;
pub mod progress;

pub use archive_properties::ArchiveProperties;
pub use archive_entry::ArchiveEntry;
pub use archive_format::{ArchiveFormat,ALL_FORMATS};
pub use overwrite_mode::OverwriteMode;
pub use handle::ArchiveHandle;
pub use session::ArchiveSession;
pub use change::{ArchiveChange, ChangeSet};


#[cfg(test)]
mod tests {
    use crate::archive::archive_format::ArchiveFormat;
    use super::*;

    #[test]
    fn test_compression_ratio_zero_size() {
        let entry = ArchiveEntry {
            name: String::new(),
            path: String::new(),
            size: 0,
            compressed_size: 0,
            is_directory: false,
            is_encrypted: false,
            is_symlink: false,
            modified: None,
            created: None,
            accessed: None,
            crc: None,
            attributes: None,
            posix_attrib: None,
            host_os: None,
            compression_method: None,
            comment: None,
            user: None,
            group: None,
            extension: None,
            hardlink: None,
            original_index: 0,
        };
        assert_eq!(entry.compression_ratio(), 0.0);
    }

    #[test]
    fn test_compression_ratio_no_compression() {
        let entry = ArchiveEntry {
            name: "test.bin".into(),
            path: "test.bin".into(),
            size: 1000,
            compressed_size: 1000,
            ..Default::default()
        };
        assert_eq!(entry.compression_ratio(), 0.0);
    }

    #[test]
    fn test_compression_ratio_positive() {
        let entry = ArchiveEntry {
            name: "test.txt".into(),
            path: "test.txt".into(),
            size: 1000,
            compressed_size: 300,
            ..Default::default()
        };
        let ratio = entry.compression_ratio();
        assert!((ratio - 0.7).abs() < 0.001, "Expected ~0.7, got {}", ratio);
    }

    #[test]
    fn test_archive_format_extension() {
        assert_eq!(ArchiveFormat::SevenZip.extension(), "7z");
        assert_eq!(ArchiveFormat::Zip.extension(), "zip");
        assert_eq!(ArchiveFormat::Tar.extension(), "tar");
    }

    #[test]
    fn test_archive_format_supports_encryption() {
        assert!(ArchiveFormat::SevenZip.supports_encryption());
        assert!(ArchiveFormat::Zip.supports_encryption());
        assert!(!ArchiveFormat::Tar.supports_encryption());
        assert!(!ArchiveFormat::Rar.supports_encryption());
    }

    #[test]
    fn test_archive_format_is_writable() {
        assert!(ArchiveFormat::SevenZip.is_writable());
        assert!(!ArchiveFormat::Rar.is_writable());
    }

    #[test]
    fn test_archive_format_display_name() {
        assert_eq!(ArchiveFormat::SevenZip.display_name(), "7z");
        assert_eq!(ArchiveFormat::TarGz.display_name(), "Tar.gz");
    }

    mod test_result_logic {
        use crate::archive::test::{TestFailure, TestFailureReason, TestResult};


        #[test]
        fn test_all_passed() {
            let r = TestResult {
                total: 10,
                passed: 10,
                failed: vec![],
            };
            assert_eq!(r.total, 10);
            assert_eq!(r.passed, 10);
            assert!(r.failed.is_empty());
        }

        #[test]
        fn test_all_failed() {
            let failures: Vec<TestFailure> = (0..3)
                .map(|i| TestFailure {
                    entry_path: format!("f{}.txt", i),
                    error: "CRC mismatch".into(),
                    index: i,
                    path: format!("f{}.txt", i),
                    reason: TestFailureReason::CrcMismatch {
                        expected: i as u32,
                        actual: 99,
                    },
                })
                .collect();
            let r = TestResult {
                total: 3,
                passed: 0,
                failed: failures,
            };
            assert_eq!(r.total, 3);
            assert_eq!(r.passed, 0);
            assert_eq!(r.failed.len(), 3);
        }

        #[test]
        fn test_mixed() {
            let r = TestResult {
                total: 5,
                passed: 4,
                failed: vec![TestFailure {
                    entry_path: "bad.txt".into(),
                    error: "err".into(),
                    index: 1,
                    path: "bad.txt".into(),
                    reason: TestFailureReason::ReadError("err".into()),
                }],
            };
            assert_eq!(r.passed, r.total - r.failed.len());
        }

        #[test]
        fn test_empty() {
            let r = TestResult {
                total: 0,
                passed: 0,
                failed: vec![],
            };
            assert_eq!(r.total, 0);
            assert_eq!(r.passed, 0);
        }
    }
}
