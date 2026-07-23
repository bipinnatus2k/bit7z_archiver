use std::path::Path;
use bit7z_domain::vfs::SessionState;

/// Result of an integrity test.
#[derive(Debug, Clone, PartialEq)]
pub struct TestIntegrity {
    pub passed: bool,
    pub failures: Vec<String>,
}

/// Programmatic interface to archive operations.
///
/// Two implementations: `Bit7zHarness` (via usecases) and
/// `CliReferee` (via 7z/rar CLI).
pub trait TestHarness: Send + Sync {
    /// Open an archive and return the full session state.
    fn open_archive(
        &self,
        path: &Path,
        password: Option<&str>,
    ) -> Result<SessionState, String>;

    /// Extract all entries to destination directory.
    fn extract_all(
        &self,
        path: &Path,
        dest: &Path,
        password: Option<&str>,
    ) -> Result<(), String>;

    /// Run integrity test on archive.
    fn test_archive(
        &self,
        path: &Path,
        password: Option<&str>,
    ) -> Result<TestIntegrity, String>;
}
