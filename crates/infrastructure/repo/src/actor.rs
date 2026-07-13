use bit7z_domain::archive::*;
use bit7z_domain::plan::ExecutionPlan;
use bit7z_domain::repository::*;
use bit7z_infra_bit7z::ArchiveReader;
use crossbeam_channel::{Receiver, Sender};
use std::ops::Range;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread::JoinHandle;

/// Commands sent to an archive actor thread.
pub enum ActorCmd {
    Open {
        path: PathBuf,
        password: Option<Password>,
        reply: Sender<Result<(), ArchiveError>>,
    },
    List {
        range: Range<usize>,
        reply: Sender<Result<Page<ArchiveEntry>, ArchiveError>>,
    },
    ListDir {
        dir: String,
        range: Range<usize>,
        reply: Sender<Result<Page<ArchiveEntry>, ArchiveError>>,
    },
    Properties {
        reply: Sender<Result<ArchiveProperties, ArchiveError>>,
    },
    Extract {
        req: ExtractRequest,
        ctx: OpCtx,
        reply: Sender<Result<ExtractReport, ArchiveError>>,
    },
    Test {
        indices: Vec<u32>,
        ctx: OpCtx,
        reply: Sender<Result<TestReport, ArchiveError>>,
    },
    Plan {
        changes: ChangeSet,
        reply: Sender<Result<ExecutionPlan, ArchiveError>>,
    },
    Apply {
        plan: ExecutionPlan,
        opts: WriteOptions,
        ctx: OpCtx,
        reply: Sender<Result<(), ArchiveError>>,
    },
    Close {
        reply: Sender<()>,
    },
}

/// Spawn an actor thread that exclusively owns an ArchiveReader.
///
/// The actor initializes COM (on Windows) with STA apartment, processes
/// commands sequentially, and cleans up COM on exit.
pub fn spawn_actor(
    lib: Arc<bit7z_infra_bit7z::Library>,
    cmd_rx: Receiver<ActorCmd>,
    name: String,
) -> Result<JoinHandle<()>, std::io::Error> {
    std::thread::Builder::new()
        .name(name)
        .spawn(move || {
            #[cfg(windows)]
            let com_owner = co_init_sta();

            let mut reader: Option<ArchiveReader> = None;

            while let Ok(cmd) = cmd_rx.recv() {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    handle_command(&lib, &mut reader, cmd)
                }));
                if let Err(panic_err) = result {
                    let msg = if let Some(s) = panic_err.downcast_ref::<&str>() {
                        s.to_string()
                    } else if let Some(s) = panic_err.downcast_ref::<String>() {
                        s.clone()
                    } else {
                        "actor panic".to_string()
                    };
                    log::error!("Actor thread panic: {}", msg);
                }
            }

            // Drop reader explicitly before COM uninit
            drop(reader);

            #[cfg(windows)]
            co_uninit(com_owner);
        })
}

fn handle_command(
    lib: &bit7z_infra_bit7z::Library,
    reader: &mut Option<ArchiveReader>,
    cmd: ActorCmd,
) {
    match cmd {
        ActorCmd::Open { path, password, reply } => {
            let path_str = match path.to_str() {
                Some(s) => s.to_string(),
                None => {
                    let _ = reply.send(Err(ArchiveError::Internal(
                        "invalid UTF-8 path".into(),
                    )));
                    return;
                }
            };
            let pw_ref = password.as_ref();
            let result = ArchiveReader::open(lib, &path_str, pw_ref);
            match result {
                Ok(rd) => {
                    *reader = Some(rd);
                    let _ = reply.send(Ok(()));
                }
                Err(e) => {
                    let _ = reply.send(Err(ArchiveError::Internal(e)));
                }
            }
        }
        ActorCmd::List { range, reply } => {
            let result = match reader.as_ref() {
                Some(rd) => handle_list(rd, range),
                None => Err(ArchiveError::NotOpen),
            };
            let _ = reply.send(result);
        }
        ActorCmd::ListDir { dir, range, reply } => {
            let result = match reader.as_ref() {
                Some(rd) => handle_list_dir(rd, &dir, range),
                None => Err(ArchiveError::NotOpen),
            };
            let _ = reply.send(result);
        }
        ActorCmd::Properties { reply } => {
            let result = match reader.as_ref() {
                Some(rd) => handle_properties(rd),
                None => Err(ArchiveError::NotOpen),
            };
            let _ = reply.send(result);
        }
        ActorCmd::Extract { req, ctx, reply } => {
            let result = match reader.as_ref() {
                Some(rd) => handle_extract(rd, req, ctx),
                None => Err(ArchiveError::NotOpen),
            };
            let _ = reply.send(result);
        }
        ActorCmd::Test { indices, ctx, reply } => {
            let result = match reader.as_ref() {
                Some(rd) => handle_test(rd, &indices, ctx),
                None => Err(ArchiveError::NotOpen),
            };
            let _ = reply.send(result);
        }
        ActorCmd::Plan { changes, reply } => {
            let result = match reader.as_ref() {
                Some(rd) => handle_plan(rd, changes),
                None => Err(ArchiveError::NotOpen),
            };
            let _ = reply.send(result);
        }
        ActorCmd::Apply { plan, opts, ctx, reply } => {
            let result = match reader.as_ref() {
                Some(rd) => handle_apply(rd, plan, opts, ctx),
                None => Err(ArchiveError::NotOpen),
            };
            let _ = reply.send(result);
        }
        ActorCmd::Close { reply } => {
            let _ = reader.take();
            let _ = reply.send(());
        }
    }
}

fn handle_list(rd: &ArchiveReader, range: Range<usize>) -> Result<Page<ArchiveEntry>, ArchiveError> {
    let count = rd.item_count();
    let start = range.start.min(count as usize);
    let end = range.end.min(count as usize);
    let mut entries = Vec::new();
    for i in start..end {
        let item = rd.item(i as u32);
        entries.push(
            ArchiveEntry::builder()
                .name(item.name())
                .path(item.path())
                .size(item.size())
                .compressed_size(item.packed_size())
                .is_directory(item.is_directory())
                .is_encrypted(item.is_encrypted())
                .original_index(item.index)
                .build()
        );
    }
    Ok(Page::new(entries, range.start, Some(count as usize)))
}

fn handle_list_dir(
    rd: &ArchiveReader,
    dir: &str,
    range: Range<usize>,
) -> Result<Page<ArchiveEntry>, ArchiveError> {
    let count = rd.item_count();
    let prefix = if dir.is_empty() { String::new() } else { dir.to_string() };
    let plen = prefix.len();
    let mut matching = Vec::new();
    for i in 0..count {
        let item = rd.item(i);
        let path = item.path();
        let matches = if plen == 0 {
            !path.contains('/')
        } else {
            path.starts_with(&prefix) && path[plen..].find('/').is_none()
        };
        if matches {
            matching.push(i);
        }
    }
    let total = matching.len();
    let entries: Vec<ArchiveEntry> = matching
        .into_iter()
        .skip(range.start)
        .take(range.end.saturating_sub(range.start))
        .map(|idx| {
            let item = rd.item(idx);
            ArchiveEntry::builder()
                .name(item.name())
                .path(item.path())
                .size(item.size())
                .compressed_size(item.packed_size())
                .is_directory(item.is_directory())
                .is_encrypted(item.is_encrypted())
                .original_index(idx)
                .build()
        })
        .collect();
    Ok(Page::new(entries, range.start, Some(total)))
}

fn handle_properties(rd: &ArchiveReader) -> Result<ArchiveProperties, ArchiveError> {
    let count = rd.item_count();
    let mut folders = 0u32;
    let mut files = 0u32;
    let mut total_size = 0u64;
    let mut packed_size = 0u64;
    for i in 0..count {
        let item = rd.item(i);
        if item.is_directory() {
            folders += 1;
        } else {
            files += 1;
        }
        total_size += item.size();
        packed_size += item.packed_size();
    }

    Ok(ArchiveProperties::new(
        count,
        folders,
        files,
        total_size,
        packed_size,
        rd.is_solid(),
        rd.is_multi_volume(),
        rd.volumes_count(),
        rd.headers_size(),
        rd.has_comment(),
        {
            let sz = rd.dictionary_size();
            if sz > 0 { Some(sz) } else { None }
        },
    ))
}

fn handle_extract(
    rd: &ArchiveReader,
    req: ExtractRequest,
    ctx: OpCtx,
) -> Result<ExtractReport, ArchiveError> {
    let dest_str = req.dest.to_str().ok_or_else(|| {
        ArchiveError::Internal("destination path is not valid UTF-8".into())
    })?;

    let progress_ptr = Box::into_raw(Box::new(ctx)) as *mut std::ffi::c_void;

    // SAFETY: extract_to_cb calls the trampolines on the same thread (actor thread).
    // The OpCtx (behind progress_ptr) lives until Box::from_raw below.
    // The trampolines only run during this synchronous FFI call.
    let result = unsafe {
        rd.extract_to_cb(
            &req.indices,
            dest_str,
            progress_ptr,
            Some(crate::trampolines::extract_overwrite_callback),
            Some(crate::trampolines::progress_trampoline),
            Some(crate::trampolines::extract_file_callback),
        )
    };

    // Recover the OpCtx; we don't need it after the call
    // SAFETY: progress_ptr was created by Box::into_raw above; we own it exclusively.
    let _recovered = unsafe { Box::from_raw(progress_ptr as *mut OpCtx) };

    match result {
        Ok(()) => Ok(ExtractReport::default()),
        Err(e) => {
            if e.contains("cancel") || e.contains("Cancel") {
                Err(ArchiveError::Cancelled)
            } else {
                Err(ArchiveError::Internal(e))
            }
        }
    }
}

fn handle_test(
    rd: &ArchiveReader,
    _indices: &[u32],
    _ctx: OpCtx,
) -> Result<TestReport, ArchiveError> {
    // ArchiveReader::test() tests all items; we convert to TestReport
    let (all_ok, total, failed_count, _failed_paths, failed_errors) =
        rd.test().map_err(|e| ArchiveError::Internal(e))?;

    let mut failed = Vec::new();
    if !all_ok && failed_count > 0 {
        for i in 0..failed_count.min(total) {
            let error = failed_errors.first().cloned().unwrap_or_else(|| "test failed".into());
            failed.push(TestFailure {
                entry_path: format!("index {}", i),
                error: error.clone(),
                index: i as usize,
                path: String::new(),
                reason: TestFailureReason::ReadError(error),
            });
        }
    }

    // If indices are provided and test was on all items, filter to just the requested indices
    // For now, we report the aggregate result since the C API tests all items at once.
    Ok(TestReport {
        all_ok,
        total,
        failed,
    })
}

fn handle_plan(
    rd: &ArchiveReader,
    changes: ChangeSet,
) -> Result<ExecutionPlan, ArchiveError> {
    let count = rd.item_count();
    let mut entries = Vec::with_capacity(count as usize);
    for i in 0..count {
        let item = rd.item(i);
        entries.push(
            ArchiveEntry::builder()
                .name(item.name())
                .path(item.path())
                .size(item.size())
                .compressed_size(item.packed_size())
                .is_directory(item.is_directory())
                .is_encrypted(item.is_encrypted())
                .original_index(i)
                .build()
        );
    }
    Ok(bit7z_domain::plan::plan_changes(&entries, &changes))
}

fn handle_apply(
    rd: &ArchiveReader,
    _plan: ExecutionPlan,
    _opts: WriteOptions,
    _ctx: OpCtx,
) -> Result<(), ArchiveError> {
    let _ = rd;
    // Apply is not yet supported in the actor model (requires Editor/Writer).
    // The integration phase will wire this up.
    Err(ArchiveError::UnsupportedOperation)
}

// ============================================================================
// COM initialization helpers
// ============================================================================

#[cfg(windows)]
fn co_init_sta() -> bool {
    // SAFETY: CoInitializeEx is called once per actor thread before any FFI
    // call. The thread never crosses COM apartments because FFI handles never
    // leave this thread. COINIT_APARTMENTTHREADED matches bit7z's threading
    // model.
    unsafe {
        let hr = windows::Win32::System::Com::CoInitializeEx(
            None,
            windows::Win32::System::Com::COINIT_APARTMENTTHREADED,
        );
        // S_OK (0) or S_FALSE (1): we own the initialization and must uninit
        // RPC_E_CHANGED_MODE (0x8001010D): COM already initialized in another
        //   mode; do NOT uninit
        const RPC_E_CHANGED_MODE: i32 = -2147418099;
        let should_uninit = hr.0 >= 0;
        if hr.0 < 0 && hr.0 != RPC_E_CHANGED_MODE {
            log::warn!("CoInitializeEx returned HRESULT {:08x}", hr.0 as u32);
        }
        should_uninit
    }
}

#[cfg(not(windows))]
fn co_init_sta() -> bool {
    false
}

#[cfg(windows)]
fn co_uninit(should_uninit: bool) {
    if should_uninit {
        // SAFETY: CoUninitialize matches the successful CoInitializeEx(STA)
        // call at thread start. Called when the actor thread exits.
        unsafe {
            windows::Win32::System::Com::CoUninitialize();
        }
    }
}

#[cfg(not(windows))]
fn co_uninit(_should_uninit: bool) {}
