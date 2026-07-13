use bit7z_domain::archive::OverwriteMode;
use bit7z_domain::repository::OpCtx;
use std::ffi::{CStr, c_char, c_void};

/// Composite context passed through FFI callbacks during extraction.
/// Contains both the per-call OpCtx and the overwrite mode from the request.
pub(crate) struct ExtractCtx {
    pub ctx: OpCtx,
    pub overwrite: OverwriteMode,
}

/// FFI trampoline called by bit7z for extraction progress updates.
///
/// # Safety
///
/// `user_data` must point to a valid `ExtractCtx` on the actor thread's stack.
/// The actor thread is blocked in an FFI call (`extract_to_cb`), and the
/// `ExtractCtx` outlives the callback because it lives in the `handle_extract`
/// stack frame which is above the FFI call on the call stack.
/// Only this callback ever dereferences the pointer, and it runs on the
/// same thread that created the `ExtractCtx`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn progress_trampoline(
    processed: u64,
    total: u64,
    user_data: *mut c_void,
) -> i32 {
    // SAFETY: See function-level doc. user_data points to an ExtractCtx owned
    // by the actor thread's handle_extract stack frame.
    let ectx = unsafe { &*(user_data as *const ExtractCtx) };
    let ctx = &ectx.ctx;

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
/// Same safety invariants as `progress_trampoline`: `user_data` points to a
/// valid `ExtractCtx` that outlives this callback. `path` points to a
/// null-terminated C string valid for the duration of the callback.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn extract_file_callback(
    path: *const c_char,
    file_size: u64,
    user_data: *mut c_void,
) {
    // SAFETY: user_data points to a valid ExtractCtx owned by the actor thread.
    // path is a null-terminated C string provided by the FFI layer.
    let ectx = unsafe { &*(user_data as *const ExtractCtx) };

    let path_str = unsafe { CStr::from_ptr(path) }
        .to_string_lossy()
        .into_owned();

    let _ = file_size;
    ectx.ctx.progress.on_file(&path_str);
}

/// FFI trampoline called by bit7z for overwrite-confirmation queries.
/// Return 0 to overwrite, 1 to skip, non-zero to cancel.
///
/// # Safety
///
/// Same safety invariants as `progress_trampoline`. `user_data` points to a
/// valid `ExtractCtx` that outlives this callback. `src` and `dest` point to
/// null-terminated C strings valid for the duration of the callback.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn extract_overwrite_callback(
    _src: *const c_char,
    _dest: *const c_char,
    _existing_size: u64,
    _src_size: u64,
    _src_mtime: i64,
    _dest_mtime: i64,
    user_data: *mut c_void,
) -> i32 {
    // SAFETY: user_data points to a valid ExtractCtx on the same thread.
    let ectx = unsafe { &*(user_data as *const ExtractCtx) };

    match ectx.overwrite {
        OverwriteMode::Overwrite => 0,
        OverwriteMode::Skip => 1,
        OverwriteMode::Ask => {
            // Notify progress sink about the file being extracted
            // (caller can observe this to understand overwrite decisions).
            // Default to overwrite for now; full interactive Ask support
            // requires a bi-directional callback mechanism (future work).
            0
        }
        OverwriteMode::RenameExtracted => {
            // Rename logic is complex (requires generating unique names);
            // fall back to overwrite for now.
            // TODO: implement rename-extracted by modifying the destination path
            0
        }
    }
}

/// RAII guard that reclaims a Boxed `ExtractCtx` when the guard goes out of scope,
/// including during panic unwind. This prevents memory leaks if the FFI call panics
/// before `Box::from_raw` is reached.
pub(crate) struct CtxGuard(pub(crate) *mut ExtractCtx);

impl Drop for CtxGuard {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: pointer was created by Box::into_raw in handle_extract;
            // this guard is the sole owner.
            unsafe {
                drop(Box::from_raw(self.0));
            }
        }
    }
}
