use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use bit7z_domain::archive::{
    ArchiveEntry, ArchiveFormat, ArchiveSession, ChangeSet, EncryptionConfig, SessionId,
};
use bit7z_domain::plan::{ExecutionPlan, plan_changes};
use bit7z_domain::repository::ArchiveError;
use bit7z_ports::ArchiveWriter;

use bit7z_infra_bit7z as bit7z;

use crate::adapters::ffi_util::detect_writer_format;

/// FFI-backed writer adapter.
pub struct Bit7zWriterAdapter {
    lib: bit7z::Library,
    handles: Mutex<HashMap<SessionId, bit7z::FfiHandle>>,
}

impl Bit7zWriterAdapter {
    pub fn new(lib: bit7z::Library) -> Self {
        Self {
            lib,
            handles: Mutex::new(HashMap::new()),
        }
    }

    pub fn close(&self, session_id: SessionId) {
        if let Ok(mut guard) = self.handles.lock() {
            guard.remove(&session_id);
        }
    }

    fn raw_ptr(&self, session_id: SessionId) -> Result<*mut std::ffi::c_void, ArchiveError> {
        let guard = self.handles.lock().map_err(|_| {
            ArchiveError::Internal("[Bit7zWriterAdapter] lock poisoned".into())
        })?;
        guard
            .get(&session_id)
            .map(|h| h.ptr())
            .ok_or_else(|| {
                ArchiveError::Internal(format!(
                    "[Bit7zWriterAdapter] session {} not found",
                    session_id
                ))
            })
    }

    fn archive_format_to_writer_format(format: ArchiveFormat) -> Result<bit7z::WriterFormat, ArchiveError> {
        match format {
            ArchiveFormat::SevenZip => Ok(bit7z::WriterFormat::SevenZip),
            ArchiveFormat::Zip => Ok(bit7z::WriterFormat::Zip),
            ArchiveFormat::Tar => Ok(bit7z::WriterFormat::Tar),
            ArchiveFormat::TarGz => Ok(bit7z::WriterFormat::GZip),
            ArchiveFormat::TarBz2 => Ok(bit7z::WriterFormat::BZip2),
            ArchiveFormat::TarXz => Ok(bit7z::WriterFormat::Xz),
            ArchiveFormat::Rar => Err(ArchiveError::UnsupportedOperation),
        }
    }
}

impl ArchiveWriter for Bit7zWriterAdapter {
    fn create(
        &self,
        path: &Path,
        format: ArchiveFormat,
        encryption: Option<&EncryptionConfig>,
    ) -> Result<ArchiveSession, ArchiveError> {
        let _path_str = path.to_str().ok_or_else(|| {
            ArchiveError::Internal(format!(
                "[Bit7zWriterAdapter::create] path is not valid UTF-8: {}",
                path.display()
            ))
        })?;

        let writer_format = Self::archive_format_to_writer_format(format)?;
        let writer = bit7z::Writer::create(&self.lib, writer_format)
            .map_err(|e| ArchiveError::Internal(format!(
                "[create] failed to create writer for format {:?}: {}",
                writer_format, e
            )))?;

        if let Some(enc) = encryption {
            if !enc.password.is_empty() {
                writer.set_password(enc.password.as_str());
            }
        }

        let raw = writer.into_raw();
        let session = ArchiveSession {
            id: bit7z_domain::archive::next_archive_id(),
            path: path.to_path_buf(),
            format,
            password: encryption.map(|e| e.password.clone()),
        };

        let mut guard = self.handles.lock().map_err(|_| {
            ArchiveError::Internal("[Bit7zWriterAdapter::create] lock poisoned".into())
        })?;
        guard.insert(session.id, bit7z::FfiHandle::writer(raw.as_ptr()));

        Ok(session)
    }

    fn commit(
        &self,
        session: &ArchiveSession,
        snapshot: &[ArchiveEntry],
        changeset: &ChangeSet,
    ) -> Result<(), ArchiveError> {
        let plan = plan_changes(snapshot, changeset);
        let path = session.path.to_str().ok_or_else(|| {
            ArchiveError::Internal(format!(
                "[Bit7zWriterAdapter::commit] path is not valid UTF-8: {}",
                session.path.display()
            ))
        })?;

        let format = detect_writer_format(&session.path);

        let _cancel = Arc::new(AtomicBool::new(false));
        let _paused = Arc::new(AtomicBool::new(false));

        if plan.deletes.is_empty() && plan.renames.is_empty() && plan.adds.is_empty() && plan.updates.is_empty() {
            return Ok(());
        }

        // Decide whether to use the editor path or the writer-only path.
        let use_editor = !plan.deletes.is_empty() || !plan.renames.is_empty();
        if use_editor {
            execute_with_editor(&self.lib, path, format, &plan)
        } else {
            execute_with_writer(&self.lib, path, format, &plan)
        }
    }
}

fn execute_with_editor(
    lib: &bit7z::Library,
    path: &str,
    format: bit7z::WriterFormat,
    plan: &ExecutionPlan,
) -> Result<(), ArchiveError> {
    let editor = bit7z::Editor::open(lib, path, format, None)
        .map_err(|e| ArchiveError::Internal(format!("editor open: {}", e)))?;

    let mut sorted_deletes = plan.deletes.clone();
    sorted_deletes.sort_unstable_by(|a, b| b.cmp(a));
    for &idx in &sorted_deletes {
        editor
            .delete(idx)
            .map_err(|e| ArchiveError::Internal(format!("delete: {}", e)))?;
    }

    for &(idx, ref new_path) in &plan.renames {
        editor
            .rename(idx, new_path)
            .map_err(|e| ArchiveError::Internal(format!("rename: {}", e)))?;
    }

    editor
        .apply()
        .map_err(|e| ArchiveError::Internal(format!("apply: {}", e)))?;

    if !plan.adds.is_empty() || !plan.updates.is_empty() {
        let writer = bit7z::Writer::open(lib, path, format, None)
            .map_err(|e| ArchiveError::Internal(format!("writer open after editor: {}", e)))?;
        writer.set_update_mode(bit7z::UpdateMode::Append);

        for (fs_path, _arc_path) in &plan.adds {
            let path_str = fs_path
                .to_str()
                .ok_or_else(|| ArchiveError::Internal("invalid path".into()))?;
            writer
                .add_file(path_str)
                .map_err(|e| ArchiveError::Internal(format!("add: {}", e)))?;
        }

        for (fs_path, _arc_path) in &plan.updates {
            let path_str = fs_path
                .to_str()
                .ok_or_else(|| ArchiveError::Internal("invalid path".into()))?;
            writer
                .add_file(path_str)
                .map_err(|e| ArchiveError::Internal(format!("update: {}", e)))?;
        }

        writer
            .compress_to(path)
            .map_err(|e| ArchiveError::Internal(format!("compress after editor: {}", e)))?;
    }

    Ok(())
}

fn execute_with_writer(
    lib: &bit7z::Library,
    path: &str,
    format: bit7z::WriterFormat,
    plan: &ExecutionPlan,
) -> Result<(), ArchiveError> {
    let writer = bit7z::Writer::open(lib, path, format, None)
        .map_err(|e| ArchiveError::Internal(format!("writer open: {}", e)))?;

    let has_updates = !plan.updates.is_empty();
    writer.set_update_mode(if has_updates {
        bit7z::UpdateMode::Update
    } else {
        bit7z::UpdateMode::Append
    });

    for (fs_path, _arc_path) in &plan.adds {
        let path_str = fs_path
            .to_str()
            .ok_or_else(|| ArchiveError::Internal("invalid path".into()))?;
        writer
            .add_file(path_str)
            .map_err(|e| ArchiveError::Internal(format!("add: {}", e)))?;
    }

    for (fs_path, _arc_path) in &plan.updates {
        let path_str = fs_path
            .to_str()
            .ok_or_else(|| ArchiveError::Internal("invalid path".into()))?;
        writer
            .add_file(path_str)
            .map_err(|e| ArchiveError::Internal(format!("update: {}", e)))?;
    }

    writer
        .compress_to(path)
        .map_err(|e| ArchiveError::Internal(format!("compress: {}", e)))?;

    Ok(())
}

