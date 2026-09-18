use bit7z_domain::archive::{ArchiveHandle, ArchiveProperties};
use std::path::Path;
use std::sync::Arc;
use bit7z_domain::archive::error::ArchiveError;
use bit7z_domain::password::Password;
use crate::runtime_service::ArchiveService;

pub struct OpenArchiveUseCase {
    service: Arc<ArchiveService>,
}

pub struct OpenArchiveOutput {
    pub handle: ArchiveHandle,
    pub properties: ArchiveProperties,
}

impl OpenArchiveUseCase {
    pub fn new(service: Arc<ArchiveService>) -> Self {
        Self { service }
    }

    pub fn execute(
        &self,
        path: &Path,
        password: Option<&Password>,
    ) -> Result<OpenArchiveOutput, ArchiveError> {
        let handle = self.service.open(path, password)?;
        let properties = self.service.get_properties(&handle)?;
        Ok(OpenArchiveOutput { handle, properties })
    }
}
