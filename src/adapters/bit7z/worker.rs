use super::{ArchiveReader, Library, Writer, WriterCompressionLevel, WriterFormat};
use crate::adapters::bit7z::UpdateMode;
use crossbeam::channel::{Receiver, Sender};
use std::ffi::{CStr, c_char, c_void};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

/// Events emitted by the worker to the UI thread.
#[derive(Debug)]
pub enum WorkerEvent {
    Progress { current: u64, total: u64, file: String },
    Conflict { src: String, dest: String, existing_size: u64 },
    Complete { total_files: u64, total_bytes: u64 },
    Error(String),
}

/// User's response to a conflict query.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictAction {
    Overwrite,
    Skip,
}

struct WorkerCtx {
    cancel: Arc<AtomicBool>,
    pause: Arc<AtomicBool>,
    event_tx: Sender<WorkerEvent>,
    conflict_rx: Receiver<ConflictAction>,
}

unsafe impl Send for WorkerCtx {}
unsafe impl Sync for WorkerCtx {}

extern "C" fn overwrite_trampoline(
    src: *const c_char,
    dest: *const c_char,
    existing_size: u64,
    ctx: *mut c_void,
) -> i32 {
    let ctx = unsafe { &*(ctx as *const WorkerCtx) };
    let src_str = unsafe { CStr::from_ptr(src) }.to_string_lossy().into_owned();
    let dest_str = unsafe { CStr::from_ptr(dest) }.to_string_lossy().into_owned();

    if ctx.event_tx.send(WorkerEvent::Conflict {
        src: src_str,
        dest: dest_str,
        existing_size,
    }).is_err() {
        return 1;
    }

    match ctx.conflict_rx.recv() {
        Ok(ConflictAction::Overwrite) => 0,
        _ => 1,
    }
}

extern "C" fn progress_trampoline(processed: u64, total: u64, ctx: *mut c_void) -> i32 {
    let ctx = unsafe { &*(ctx as *const WorkerCtx) };

    if ctx.cancel.load(Ordering::Relaxed) {
        return 0;
    }

    while ctx.pause.load(Ordering::Relaxed) {
        std::thread::sleep(std::time::Duration::from_millis(50));
        if ctx.cancel.load(Ordering::Relaxed) {
            return 0;
        }
    }

    let _ = ctx.event_tx.send(WorkerEvent::Progress {
        current: processed,
        total,
        file: String::new(),
    });
    1
}

extern "C" fn file_trampoline(path: *const c_char, ctx: *mut c_void) {
    let ctx = unsafe { &*(ctx as *const WorkerCtx) };
    let path_str = unsafe { CStr::from_ptr(path) }.to_string_lossy().into_owned();
    let _ = ctx.event_tx.send(WorkerEvent::Progress {
        current: 0,
        total: 0,
        file: path_str,
    });
}

/// Spawn a background thread that extracts items with progress, cancel, pause,
/// and per-file conflict support.
///
/// # Parameters
/// - `reader`: the archive reader (consumed — moved into the thread)
/// - `indices`: zero-based item indices to extract
/// - `dest`: destination directory path
/// - `cancel`: set to `true` to cancel the operation
/// - `pause`: set to `true` to pause the operation
/// - `event_tx`: channel for receiving progress/conflict/complete/error events
/// - `conflict_rx`: channel where the UI sends conflict responses
///
/// # Panics
/// Panics if the C-linkage callback bridge function cannot be found (linkage error).
#[allow(clippy::too_many_arguments)]
pub fn spawn_extract(
    reader: ArchiveReader,
    indices: Vec<u32>,
    dest: String,
    cancel: Arc<AtomicBool>,
    pause: Arc<AtomicBool>,
    event_tx: Sender<WorkerEvent>,
    conflict_rx: Receiver<ConflictAction>,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        let ctx = WorkerCtx {
            cancel,
            pause,
            event_tx: event_tx.clone(),
            conflict_rx,
        };

        let result = unsafe {
            reader.extract_to_cb(
                &indices,
                &dest,
                &ctx as *const WorkerCtx as *mut c_void,
                Some(overwrite_trampoline as unsafe extern "C" fn(_, _, _, _) -> _),
                Some(progress_trampoline as unsafe extern "C" fn(_, _, _) -> _),
                Some(file_trampoline as unsafe extern "C" fn(_, _)),
            )
        };

        match result {
            Ok(()) => {
                let _ = event_tx.send(WorkerEvent::Complete {
                    total_files: indices.len() as u64,
                    total_bytes: 0,
                });
            }
            Err(e) => {
                let _ = event_tx.send(WorkerEvent::Error(e));
            }
        }
    })
}

/// Spawn a background thread that compresses files with progress, cancel and pause support.
///
/// # Parameters
/// - `lib`: the 7-Zip library handle
/// - `format`: output archive format
/// - `files`: list of file/directory paths to compress
/// - `dest`: output archive file path
/// - `threads`: number of compression threads (0 = auto)
/// - `compression_level`: compression level
/// - `password`: optional encryption password
/// - `update_mode`: how to handle existing archives
/// - `cancel`: set to `true` to cancel
/// - `pause`: set to `true` to pause
/// - `event_tx`: channel for progress/complete/error events
#[allow(clippy::too_many_arguments)]
pub fn spawn_compress(
    lib: &Library,
    format: WriterFormat,
    files: Vec<String>,
    dest: String,
    threads: u32,
    compression_level: WriterCompressionLevel,
    password: Option<String>,
    update_mode: UpdateMode,
    cancel: Arc<AtomicBool>,
    pause: Arc<AtomicBool>,
    event_tx: Sender<WorkerEvent>,
) -> Result<JoinHandle<()>, String> {
    let writer = Writer::create(lib, format)?;
    writer.set_threads(threads);
    writer.set_compression_level(compression_level);
    if let Some(ref pw) = password {
        writer.set_password(pw);
    }
    writer.set_update_mode(update_mode);
    for f in &files {
        writer.add_file(f).map_err(|e| format!("add_file {}: {}", f, e))?;
    }

    Ok(std::thread::spawn(move || {
        let ctx = WorkerCtx {
            cancel,
            pause,
            event_tx: event_tx.clone(),
            conflict_rx: crossbeam::channel::bounded(1).1, // dummy, never used
        };

        let result = unsafe {
            writer.compress_to_cb(
                &dest,
                &ctx as *const WorkerCtx as *mut c_void,
                Some(progress_trampoline as unsafe extern "C" fn(_, _, _) -> _),
                Some(file_trampoline as unsafe extern "C" fn(_, _)),
            )
        };

        match result {
            Ok(()) => {
                let _ = event_tx.send(WorkerEvent::Complete {
                    total_files: files.len() as u64,
                    total_bytes: 0,
                });
            }
            Err(e) => {
                let _ = event_tx.send(WorkerEvent::Error(e));
            }
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    fn make_ctx(
        event_tx: Sender<WorkerEvent>,
        conflict_rx: Receiver<ConflictAction>,
    ) -> WorkerCtx {
        WorkerCtx {
            cancel: Arc::new(AtomicBool::new(false)),
            pause: Arc::new(AtomicBool::new(false)),
            event_tx,
            conflict_rx,
        }
    }

    fn as_ctx_ptr(ctx: &WorkerCtx) -> *mut c_void {
        ctx as *const WorkerCtx as *mut c_void
    }

    // -- Progress trampoline tests --

    #[test]
    fn test_progress_sends_event() {
        let (tx, rx) = crossbeam::channel::unbounded();
        let (_, conflict_rx) = crossbeam::channel::bounded(1);
        let ctx = make_ctx(tx, conflict_rx);

        let result = progress_trampoline(42, 100, as_ctx_ptr(&ctx));

        assert_eq!(result, 1, "expected continue");
        let event = rx.try_recv().expect("expected progress event");
        match event {
            WorkerEvent::Progress { current, total, file } => {
                assert_eq!(current, 42);
                assert_eq!(total, 100);
                assert!(file.is_empty());
            }
            other => panic!("unexpected event: {:?}", other),
        }
    }

    #[test]
    fn test_progress_cancel_returns_zero() {
        let (tx, _) = crossbeam::channel::unbounded();
        let (_, conflict_rx) = crossbeam::channel::bounded(1);
        let mut ctx = make_ctx(tx, conflict_rx);
        ctx.cancel = Arc::new(AtomicBool::new(true));

        let result = progress_trampoline(10, 100, as_ctx_ptr(&ctx));
        assert_eq!(result, 0, "expected cancel");
    }

    #[test]
    fn test_progress_unpause_after_pause() {
        let (tx, rx) = crossbeam::channel::unbounded();
        let (_, conflict_rx) = crossbeam::channel::bounded(1);
        let pause = Arc::new(AtomicBool::new(true));
        let ctx = WorkerCtx {
            cancel: Arc::new(AtomicBool::new(false)),
            pause: pause.clone(),
            event_tx: tx,
            conflict_rx,
        };

        // Spawn a thread that unpauses after a short delay
        let pause_clone = pause.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(100));
            pause_clone.store(false, Ordering::Relaxed);
        });

        let result = progress_trampoline(50, 200, as_ctx_ptr(&ctx));
        assert_eq!(result, 1, "expected continue after unpause");

        let event = rx.try_recv().expect("expected progress event after unpause");
        match event {
            WorkerEvent::Progress { current, total, .. } => {
                assert_eq!(current, 50);
                assert_eq!(total, 200);
            }
            other => panic!("unexpected: {:?}", other),
        }
    }

    // -- File trampoline tests --

    #[test]
    fn test_file_sends_event() {
        let (tx, rx) = crossbeam::channel::unbounded();
        let (_, conflict_rx) = crossbeam::channel::bounded(1);
        let ctx = make_ctx(tx, conflict_rx);
        let path = CString::new("dir/file.txt").unwrap();

        file_trampoline(path.as_ptr(), as_ctx_ptr(&ctx));

        let event = rx.try_recv().expect("expected file event");
        match event {
            WorkerEvent::Progress { file, .. } => {
                assert_eq!(file, "dir/file.txt");
            }
            other => panic!("unexpected: {:?}", other),
        }
    }

    // -- Overwrite trampoline tests --

    #[test]
    fn test_overwrite_sends_conflict_event() {
        let (tx, rx) = crossbeam::channel::unbounded();
        let (conflict_tx, conflict_rx) = crossbeam::channel::bounded(1);
        let ctx = make_ctx(tx, conflict_rx);
        let src = CString::new("old.txt").unwrap();
        let dest = CString::new("/out/old.txt").unwrap();

        // Spawn a thread to respond to the conflict
        let conflict_tx_clone = conflict_tx.clone();
        let handle = std::thread::spawn(move || {
            conflict_tx_clone.send(ConflictAction::Overwrite).ok();
        });

        let result = overwrite_trampoline(
            src.as_ptr(),
            dest.as_ptr(),
            1024,
            as_ctx_ptr(&ctx),
        );

        handle.join().ok();
        assert_eq!(result, 0, "expected Overwrite => 0");

        let event = rx.try_recv().expect("expected conflict event");
        match event {
            WorkerEvent::Conflict { src, dest, existing_size } => {
                assert_eq!(src, "old.txt");
                assert_eq!(dest, "/out/old.txt");
                assert_eq!(existing_size, 1024);
            }
            other => panic!("unexpected: {:?}", other),
        }
    }

    #[test]
    fn test_overwrite_skip_returns_one() {
        let (tx, _rx) = crossbeam::channel::unbounded();
        let (conflict_tx, conflict_rx) = crossbeam::channel::bounded(1);
        let ctx = make_ctx(tx, conflict_rx);
        let src = CString::new("f.txt").unwrap();
        let dest = CString::new("/out/f.txt").unwrap();

        let handle = std::thread::spawn(move || {
            conflict_tx.send(ConflictAction::Skip).ok();
        });

        let result = overwrite_trampoline(
            src.as_ptr(),
            dest.as_ptr(),
        0,
            as_ctx_ptr(&ctx),
        );

        handle.join().ok();
        assert_eq!(result, 1, "expected Skip => 1");
    }

    // -- ConflictAction type tests --

    #[test]
    fn test_conflict_action_clone_copy_eq() {
        assert_eq!(ConflictAction::Overwrite, ConflictAction::Overwrite);
        assert_eq!(ConflictAction::Skip, ConflictAction::Skip);
        assert_ne!(ConflictAction::Overwrite, ConflictAction::Skip);
    }

    // -- Channels are wired correctly --

    #[test]
    fn test_conflict_response_roundtrip() {
        let (event_tx, event_rx) = crossbeam::channel::unbounded();
        let (conflict_tx, conflict_rx) = crossbeam::channel::bounded(1);
        let ctx = make_ctx(event_tx, conflict_rx);

        let src = CString::new("a.zip").unwrap();
        let dest = CString::new("/out/a.zip").unwrap();

        // Respond from a separate thread (simulating UI response)
        let handle = std::thread::spawn(move || {
            // Wait for the conflict event
            let event = event_rx.recv().ok();
            assert!(event.is_some());
            // Send the user's decision
            conflict_tx.send(ConflictAction::Overwrite).ok();
        });

        let result = overwrite_trampoline(
            src.as_ptr(),
            dest.as_ptr(),
            0,
            as_ctx_ptr(&ctx),
        );

        handle.join().ok();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_conflict_response_channel_full_then_block() {
        let (tx, _rx) = crossbeam::channel::unbounded();
        // bounded(1) means only one response queued
        let (conflict_tx, conflict_rx) = crossbeam::channel::bounded(1);
        let ctx = make_ctx(tx, conflict_rx);
        let src = CString::new("x").unwrap();
        let dest = CString::new("y").unwrap();

        // Send response before the trampoline blocks
        conflict_tx.send(ConflictAction::Skip).ok();

        let result = overwrite_trampoline(
            src.as_ptr(),
            dest.as_ptr(),
            0,
            as_ctx_ptr(&ctx),
        );
        assert_eq!(result, 1);
    }

    // -- Error handling: channel disconnect --

    #[test]
    fn test_overwrite_disconnected_channel_returns_one() {
        let (tx, _rx) = crossbeam::channel::unbounded();
        let (conflict_tx, conflict_rx) = crossbeam::channel::bounded::<ConflictAction>(1);
        drop(conflict_tx); // disconnect so recv() fails

        let ctx = make_ctx(tx, conflict_rx);
        let src = CString::new("src").unwrap();
        let dest = CString::new("dst").unwrap();

        let result = overwrite_trampoline(
            src.as_ptr(),
            dest.as_ptr(),
            0,
            as_ctx_ptr(&ctx),
        );
        // When recv() fails, default is Skip => 1
        assert_eq!(result, 1);
    }
}
