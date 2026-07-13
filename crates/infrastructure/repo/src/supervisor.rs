use crate::actor::{spawn_actor, ActorCmd, OpenReply};
use bit7z_domain::archive::*;
use bit7z_domain::plan::ExecutionPlan;
use bit7z_domain::repository::*;
use bit7z_infra_bit7z::WriterFormat;
use crossbeam_channel::{bounded, unbounded, Sender};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

static NEXT_ARCHIVE_ID: AtomicU64 = AtomicU64::new(1);

/// Handle to an actor thread, holding the command sender and join handle.
pub struct ArchiveActorHandle {
    cmd_tx: Sender<ActorCmd>,
    join: Option<JoinHandle<()>>,
    running_cancel: parking_lot::Mutex<Option<CancellationToken>>,
    running_pause: parking_lot::Mutex<Option<PauseToken>>,
}

/// Supervisor that manages per-archive actor threads.
///
/// Each `open` call spawns a new actor thread. Commands are sent via
/// crossbeam channel and processed serially on the actor thread.
pub struct RepoSupervisor {
    lib: Arc<bit7z_infra_bit7z::Library>,
    actors: Mutex<HashMap<u64, ArchiveActorHandle>>,
}

impl RepoSupervisor {
    pub fn new(lib_path: &str) -> Result<Self, ArchiveError> {
        let lib = bit7z_infra_bit7z::Library::open(lib_path)
            .map_err(|e| ArchiveError::Internal(format!("failed to load 7-Zip library: {}", e)))?;
        Ok(Self {
            lib: Arc::new(lib),
            actors: Mutex::new(HashMap::new()),
        })
    }

    pub fn with_library(lib: Arc<bit7z_infra_bit7z::Library>) -> Self {
        Self {
            lib,
            actors: Mutex::new(HashMap::new()),
        }
    }

    fn next_id() -> u64 {
        NEXT_ARCHIVE_ID.fetch_add(1, Ordering::Relaxed)
    }

    fn spawn(&self) -> Result<(u64, ArchiveActorHandle), ArchiveError> {
        let id = Self::next_id();
        let (cmd_tx, cmd_rx) = unbounded();
        let lib = self.lib.clone();
        let name = format!("bit7z-actor-{}", id);
        let join = spawn_actor(lib, cmd_rx, name)
            .map_err(|e| ArchiveError::Internal(format!("failed to spawn actor: {}", e)))?;
        Ok((id, ArchiveActorHandle {
            cmd_tx,
            join: Some(join),
            running_cancel: parking_lot::Mutex::new(None),
            running_pause: parking_lot::Mutex::new(None),
        }))
    }

    fn actor_cmd_tx(&self, handle: &ArchiveHandle) -> Result<Sender<ActorCmd>, ArchiveError> {
        let id = handle.raw_id();
        let actors = self.actors.lock();
        actors.get(&id)
            .map(|a| a.cmd_tx.clone())
            .ok_or(ArchiveError::NotOpen)
    }

    fn send_recv<T: Send + 'static>(
        &self,
        handle: &ArchiveHandle,
        f: impl FnOnce(Sender<Result<T, ArchiveError>>) -> ActorCmd,
    ) -> Result<T, ArchiveError> {
        let cmd_tx = self.actor_cmd_tx(handle)?;
        let (reply_tx, reply_rx) = bounded(1);
        let cmd = f(reply_tx);
        cmd_tx.send(cmd).map_err(|_| ArchiveError::Internal("actor channel closed".into()))?;
        reply_rx.recv()
            .map_err(|_| ArchiveError::Internal("actor reply channel closed".into()))?
    }

    fn set_running_tokens(&self, id: u64, cancel: &CancellationToken, pause: &PauseToken) {
        let actors = self.actors.lock();
        if let Some(actor) = actors.get(&id) {
            *actor.running_cancel.lock() = Some(cancel.clone());
            *actor.running_pause.lock() = Some(pause.clone());
        }
    }

    fn clear_running_tokens(&self, id: u64) {
        let actors = self.actors.lock();
        if let Some(actor) = actors.get(&id) {
            *actor.running_cancel.lock() = None;
            *actor.running_pause.lock() = None;
        }
    }
}

impl ArchiveRepository for RepoSupervisor {
    fn open(&self, path: &Path, password: Option<&Password>) -> Result<ArchiveHandle, ArchiveError> {
        let path_str = path.to_str().ok_or_else(|| {
            ArchiveError::Internal("path is not valid UTF-8".into())
        })?;

        let (id, actor) = self.spawn()?;

        let is_header_encrypted = self.lib.is_header_encrypted(path_str);
        let needs_password = is_header_encrypted && password.is_none();

        if needs_password {
            return Err(ArchiveError::EncryptedArchiveRequiresPassword);
        }

        let (reply_tx, reply_rx) = bounded(1);
        actor.cmd_tx.send(ActorCmd::Open {
            path: path.to_path_buf(),
            password: password.cloned(),
            reply: reply_tx,
        }).map_err(|_| ArchiveError::Internal("actor channel closed".into()))?;

        let open_reply: OpenReply = reply_rx.recv()
            .map_err(|_| ArchiveError::Internal("actor reply channel closed".into()))??;

        let has_encrypted_items = open_reply.has_encrypted_items;

        let format = archive_format_from_extension(path);

        let mut handle = ArchiveHandle::new(id)
            .with_path(path.to_path_buf());
        if let Some(fmt) = format {
            handle = handle.with_format(fmt);
        }
        handle.set_encryption_info(is_header_encrypted, has_encrypted_items);

        self.actors.lock().insert(handle.raw_id(), actor);

        Ok(handle)
    }

    fn create(&self, path: &Path, format: ArchiveFormat, encryption: Option<&EncryptionConfig>) -> Result<ArchiveHandle, ArchiveError> {
        let wf = archive_format_to_writer(format);
        let password = encryption.map(|e| e.password.clone());

        let (id, actor) = self.spawn()?;

        let (reply_tx, reply_rx) = bounded(1);
        actor.cmd_tx.send(ActorCmd::CreateWriter {
            path: path.to_path_buf(),
            format: wf,
            password,
            reply: reply_tx,
        }).map_err(|_| ArchiveError::Internal("actor channel closed".into()))?;

        reply_rx.recv()
            .map_err(|_| ArchiveError::Internal("actor reply channel closed".into()))??;

        let handle = ArchiveHandle::new(id)
            .with_path(path.to_path_buf());

        self.actors.lock().insert(handle.raw_id(), actor);

        Ok(handle)
    }

    fn list(&self, handle: &ArchiveHandle, range: Range<usize>) -> Result<Page<ArchiveEntry>, ArchiveError> {
        self.send_recv(handle, |reply| ActorCmd::List { range, reply })
    }

    fn list_dir(&self, handle: &ArchiveHandle, dir: &str, range: Range<usize>) -> Result<Page<ArchiveEntry>, ArchiveError> {
        self.send_recv(handle, |reply| ActorCmd::ListDir {
            dir: dir.to_string(),
            range,
            reply,
        })
    }

    fn properties(&self, handle: &ArchiveHandle) -> Result<ArchiveProperties, ArchiveError> {
        self.send_recv(handle, |reply| ActorCmd::Properties { reply })
    }

    fn extract(&self, handle: &ArchiveHandle, req: &ExtractRequest, ctx: &OpCtx) -> Result<ExtractReport, ArchiveError> {
        let ctx_owned = OpCtx {
            cancel: ctx.cancel.clone(),
            pause: ctx.pause.clone(),
            progress: ctx.progress.clone(),
        };

        self.set_running_tokens(handle.raw_id(), &ctx.cancel, &ctx.pause);
        let result = self.send_recv(handle, |reply| ActorCmd::Extract {
            req: ExtractRequest {
                indices: req.indices.clone(),
                dest: req.dest.clone(),
                overwrite: req.overwrite,
            },
            ctx: ctx_owned,
            reply,
        });
        self.clear_running_tokens(handle.raw_id());

        result
    }

    fn extract_to_buffer(&self, handle: &ArchiveHandle, index: u32) -> Result<Vec<u8>, ArchiveError> {
        self.send_recv(handle, |reply| ActorCmd::ExtractToBuffer { index, reply })
    }

    fn test(&self, handle: &ArchiveHandle, indices: &[u32], ctx: &OpCtx) -> Result<TestReport, ArchiveError> {
        let ctx_owned = OpCtx {
            cancel: ctx.cancel.clone(),
            pause: ctx.pause.clone(),
            progress: ctx.progress.clone(),
        };

        self.set_running_tokens(handle.raw_id(), &ctx.cancel, &ctx.pause);
        let result = self.send_recv(handle, |reply| ActorCmd::Test {
            indices: indices.to_vec(),
            ctx: ctx_owned,
            reply,
        });
        self.clear_running_tokens(handle.raw_id());

        result
    }

    fn plan(&self, handle: &ArchiveHandle, changes: &ChangeSet) -> Result<ExecutionPlan, ArchiveError> {
        self.send_recv(handle, |reply| ActorCmd::Plan {
            changes: changes.clone(),
            reply,
        })
    }

    fn apply(&self, handle: &ArchiveHandle, plan: &ExecutionPlan, _opts: &WriteOptions, ctx: &OpCtx) -> Result<(), ArchiveError> {
        let ctx_owned = OpCtx {
            cancel: ctx.cancel.clone(),
            pause: ctx.pause.clone(),
            progress: ctx.progress.clone(),
        };

        let format = handle.format()
            .map(archive_format_to_writer)
            .or_else(|| {
                handle.path()
                    .and_then(archive_format_from_extension)
                    .map(archive_format_to_writer)
            })
            .unwrap_or(WriterFormat::SevenZip);

        self.set_running_tokens(handle.raw_id(), &ctx.cancel, &ctx.pause);
        let result = self.send_recv(handle, |reply| ActorCmd::Apply {
            plan: ExecutionPlan {
                deletes: plan.deletes.clone(),
                renames: plan.renames.clone(),
                adds: plan.adds.clone(),
                updates: plan.updates.clone(),
                conflicts: plan.conflicts.clone(),
            },
            format,
            opts: WriteOptions {
                cancel: _opts.cancel.clone(),
                paused: _opts.paused.clone(),
                notifier: _opts.notifier.clone(),
            },
            ctx: ctx_owned,
            reply,
        });
        self.clear_running_tokens(handle.raw_id());

        result
    }

    fn build_archive(&self, handle: &ArchiveHandle, files: &[(PathBuf, String)], out_path: &Path, ctx: &OpCtx) -> Result<(), ArchiveError> {
        let ctx_owned = OpCtx {
            cancel: ctx.cancel.clone(),
            pause: ctx.pause.clone(),
            progress: ctx.progress.clone(),
        };

        let cmd_tx = self.actor_cmd_tx(handle)?;

        self.set_running_tokens(handle.raw_id(), &ctx.cancel, &ctx.pause);

        let (reply_tx, reply_rx) = bounded(1);
        let fs_paths: Vec<PathBuf> = files.iter().map(|(p, _)| p.clone()).collect();
        cmd_tx.send(ActorCmd::AddFiles { files: fs_paths, reply: reply_tx })
            .map_err(|_| ArchiveError::Internal("actor channel closed".into()))?;
        reply_rx.recv()
            .map_err(|_| ArchiveError::Internal("actor reply channel closed".into()))??;

        let (reply_tx2, reply_rx2) = bounded(1);
        cmd_tx.send(ActorCmd::CompressTo { out_path: out_path.to_path_buf(), ctx: ctx_owned, reply: reply_tx2 })
            .map_err(|_| ArchiveError::Internal("actor channel closed".into()))?;
        reply_rx2.recv()
            .map_err(|_| ArchiveError::Internal("actor reply channel closed".into()))??;

        self.clear_running_tokens(handle.raw_id());

        Ok(())
    }

    fn close(&self, handle: &ArchiveHandle) {
        let id = handle.raw_id();
        let mut actors = self.actors.lock();
        if let Some(mut actor) = actors.remove(&id) {
            drop(actors);

            {
                let cancel = actor.running_cancel.lock();
                if let Some(ref token) = *cancel {
                    token.cancel();
                }
                drop(cancel);
                let pause = actor.running_pause.lock();
                if let Some(ref token) = *pause {
                    token.resume();
                }
            }

            let (reply_tx, reply_rx) = bounded(1);
            let _ = actor.cmd_tx.send(ActorCmd::Close { reply: reply_tx });
            let _ = reply_rx.recv();
            if let Some(join) = actor.join.take() {
                let _ = join.join();
            }
        }
    }
}

impl Drop for RepoSupervisor {
    fn drop(&mut self) {
        let actors = std::mem::take(&mut *self.actors.lock());
        for (_id, mut actor) in actors {
            {
                let cancel = actor.running_cancel.lock();
                if let Some(ref token) = *cancel {
                    token.cancel();
                }
                drop(cancel);
                let pause = actor.running_pause.lock();
                if let Some(ref token) = *pause {
                    token.resume();
                }
            }
            let (reply_tx, reply_rx) = bounded(1);
            let _ = actor.cmd_tx.send(ActorCmd::Close { reply: reply_tx });
            let _ = reply_rx.recv();
            if let Some(join) = actor.join.take() {
                let _ = join.join();
            }
        }
    }
}

fn archive_format_to_writer(fmt: ArchiveFormat) -> WriterFormat {
    match fmt {
        ArchiveFormat::SevenZip => WriterFormat::SevenZip,
        ArchiveFormat::Zip => WriterFormat::Zip,
        ArchiveFormat::Tar => WriterFormat::Tar,
        ArchiveFormat::TarGz => WriterFormat::GZip,
        ArchiveFormat::TarBz2 => WriterFormat::BZip2,
        ArchiveFormat::TarXz => WriterFormat::Xz,
        ArchiveFormat::Rar => WriterFormat::SevenZip,
    }
}

fn archive_format_from_extension(path: &Path) -> Option<ArchiveFormat> {
    let ext = path.extension()?.to_str()?;
    match ext {
        "7z" => Some(ArchiveFormat::SevenZip),
        "zip" => Some(ArchiveFormat::Zip),
        "tar" => Some(ArchiveFormat::Tar),
        "gz" | "tgz" => Some(ArchiveFormat::TarGz),
        "bz2" | "tbz2" => Some(ArchiveFormat::TarBz2),
        "xz" | "txz" => Some(ArchiveFormat::TarXz),
        "rar" => Some(ArchiveFormat::Rar),
        _ => None,
    }
}
