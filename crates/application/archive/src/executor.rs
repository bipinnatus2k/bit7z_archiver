use std::sync::Arc;

use bit7z_domain::repository::ArchiveError;
use bit7z_runtime::{OperationKind, OperationRequest, Runtime};
use bit7z_runtime::job::Priority;

use crate::capability::ArchiveOpKind;
use crate::commands::{AddFilesToArchive, CreateArchive, ExtractArchive};
use crate::runtime_service::ArchiveService;
use crate::token::OperationToken;
use crate::vfs_util::expand_directory_entries;

pub struct CommandExecutor {
    service: Arc<ArchiveService>,
    runtime: Arc<Runtime>,
}

impl CommandExecutor {
    pub fn new(
        service: Arc<ArchiveService>,
        runtime: Arc<Runtime>,
    ) -> Self {
        Self { service, runtime }
    }

    /// Submit an extraction — returns a token for progress/cancel.
    pub fn execute_extract(
        &self,
        cmd: ExtractArchive,
    ) -> Result<OperationToken, ArchiveError> {
        let session = self.service.lookup_session(&cmd.handle)?;
        let indices = expand_directory_entries(
            self.runtime.session_manager.as_ref(),
            session.id,
            &cmd.entry_indices,
        )?;

        let desc = self.service.resolve_capability(
            ArchiveOpKind::Extract,
            session.format,
            Some(session.id),
        )?;

        let handle = self.runtime.submit(OperationRequest {
            kind: OperationKind::Extract {
                session_id: session.id,
                indices,
                destination: cmd.destination,
                overwrite_mode: cmd.overwrite_mode,
            },
            descriptor: desc,
            priority: Priority::User,
        });
        Ok(OperationToken { handle, runtime: self.runtime.clone() })
    }

    /// Create a new archive (synchronous).
    pub fn execute_create(
        &self,
        cmd: CreateArchive,
    ) -> Result<bit7z_domain::archive::ArchiveHandle, ArchiveError> {
        self.service.create(&cmd.path, cmd.format, cmd.encryption.as_ref())
    }

    /// Add files to an existing archive (synchronous).
    pub fn execute_add_files(
        &self,
        cmd: AddFilesToArchive,
    ) -> Result<(), ArchiveError> {
        let use_case = crate::modify::ModifyArchiveUseCase::new(self.service.clone());
        use_case.add_files(&cmd.handle, &cmd.files, None)
    }
}
