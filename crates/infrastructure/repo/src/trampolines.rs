use bit7z_domain::repository::OpCtx;
use std::ffi::{CStr, c_char, c_void};

/// FFI trampoline called by bit7z for extraction progress updates.
///
/// # Safety
///
/// `ctx` must point to a valid `OpCtx` on the actor thread's stack.
/// The actor thread is blocked in an FFI call (`extract_to_cb`), and the
/// `OpCtx` outlives the callback because it lives in the `handle_extract`
/// stack frame which is above the FFI call on the call stack.
/// Only this callback ever dereferences the pointer, and it runs on the
/// same thread that created the `OpCtx`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn progress_trampoline(
    processed: u64,
    total: u64,
    ctx: *mut c_void,
) -> i32 {
    // SAFETY: See function-level doc. ctx points to an OpCtx owned by the
    // actor thread's handle_extract stack frame. The FFI call is synchronous,
    // so ctx is live for the entire duration.
    let ctx = unsafe { &*(ctx as *const OpCtx) };

    if ctx.cancel.is_cancelled() {
        return 0;
    }

    ctx.pause.wait_while_paused(&ctx.cancel);

    if ctx.cancel.is_cancelled() {
        return 0;
    }

    ctx.progress.on_progress(processed, total);
    1
}

/// FFI trampoline called by bit7z for extraction file-begin events.
///
/// # Safety
///
/// Same safety invariants as `progress_trampoline`: `ctx` points to a valid
/// `OpCtx` that outlives this callback. `path` points to a null-terminated
/// C string valid for the duration of the callback.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn extract_file_callback(
    path: *const c_char,
    file_size: u64,
    ctx: *mut c_void,
) {
    // SAFETY: ctx points to a valid OpCtx owned by the actor thread.
    // path is a null-terminated C string provided by the FFI layer.
    let ctx = unsafe { &*(ctx as *const OpCtx) };
    let path_str = unsafe { CStr::from_ptr(path) }
        .to_string_lossy()
        .into_owned();

    let _ = file_size;
    ctx.progress.on_file(&path_str);
}

/// FFI trampoline called by bit7z for overwrite-confirmation queries.
///
/// # Safety
///
/// Same safety invariants as `progress_trampoline`. `ctx` points to a valid
/// `OpCtx` that outlives this callback. `src` and `dest` point to
/// null-terminated C strings valid for the duration of the callback.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn extract_overwrite_callback(
    _src: *const c_char,
    _dest: *const c_char,
    _existing_size: u64,
    _src_size: u64,
    _src_mtime: i64,
    _dest_mtime: i64,
    _ctx: *mut c_void,
) -> i32 {
    // Always overwrite for now; integration phase will wire up the
    // overwrite dialog via the OpCtx.
    0
}
