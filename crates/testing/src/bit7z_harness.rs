use std::path::Path;
use std::sync::Arc;

use bit7z_app_archive::runtime_service::ArchiveService;
use bit7z_domain::archive::{Password, ArchiveHandle};
use bit7z_domain::vfs::SessionState;
use bit7z_domain::archive::progress::ArchiveError;

use crate::harness::{TestHarness, TestIntegrity};

pub struct Bit7zHarness {
    pub service: Arc<ArchiveService>,
}

impl Bit7zHarness {
    pub fn new(service: Arc<ArchiveService>) -> Self {
        Self { service }
    }

    fn to_string(e: ArchiveError) -> String {
        e.to_string()
    }

    fn open_inner(
        &self,
        path: &Path,
        password: Option<&str>,
    ) -> Result<(ArchiveHandle, SessionState), String> {
        let pw = password.map(|p| Password::new(p.to_string()));
        let session = self.service.open(path, pw.as_ref()).map_err(Self::to_string)?;
        let state = self
            .service
            .get_session_state(&session)
            .map_err(Self::to_string)?;
        Ok((session, state))
    }
}

impl TestHarness for Bit7zHarness {
    fn open_archive(
        &self,
        path: &Path,
        password: Option<&str>,
    ) -> Result<SessionState, String> {
        Ok(self.open_inner(path, password)?.1)
    }

    fn extract_all(
        &self,
        path: &Path,
        dest: &Path,
        password: Option<&str>,
    ) -> Result<(), String> {
        let (handle, state) = self.open_inner(path, password)?;

        let indices: Vec<u32> = state
            .vfs
            .base_tree()
            .all_ids()
            .iter()
            .filter_map(|&id| {
                let node = state.vfs.base_tree().node(id)?;
                if !node.is_directory {
                    node.original_index
                } else {
                    None
                }
            })
            .collect();

        use bit7z_app_archive::use_case::extract::ExtractEntriesUseCase;
        use bit7z_domain::archive::OverwriteMode;
        use bit7z_domain::archive::progress::{ExtractOptions, NoopNotifier};
        use std::sync::atomic::AtomicBool;

        let usecase = ExtractEntriesUseCase::new(self.service.clone());
        usecase
            .execute(
                &handle,
                &indices,
                dest,
                &ExtractOptions {
                    overwrite_mode: OverwriteMode::Overwrite,
                    cancel: Arc::new(AtomicBool::new(false)),
                    paused: Arc::new(AtomicBool::new(false)),
                    notifier: Arc::new(NoopNotifier),
                },
            )
            .map_err(Self::to_string)
    }

    fn test_archive(
        &self,
        path: &Path,
        password: Option<&str>,
    ) -> Result<TestIntegrity, String> {
        let (handle, _state) = self.open_inner(path, password)?;
        let result = self.service.test(&handle).map_err(Self::to_string)?;
        let failures: Vec<String> = result
            .failed
            .iter()
            .map(|f| format!("[{}] {}: {}", f.index, f.entry_path, f.error))
            .collect();
        Ok(TestIntegrity {
            passed: result.failed.is_empty(),
            failures,
        })
    }
}
