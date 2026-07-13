use bit7z_domain::archive::OverwriteMode;
use bit7z_domain::repository::{OpCtx, OverwriteDecision, OverwriteInfo, OverwriteResolver};
use std::ffi::{CStr, c_char, c_void};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Composite context passed through FFI callbacks during extraction.
/// Contains both the per-call OpCtx and the overwrite mode from the request.
pub(crate) struct ExtractCtx {
    pub ctx: OpCtx,
    pub overwrite: OverwriteMode,
    pub resolver: Option<Arc<dyn OverwriteResolver>>,
    pub global_decision: AtomicBool,
    pub has_global_decision: AtomicBool,
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

    // SAFETY: ProgressSink call is wrapped in catch_unwind to prevent unwinding
    // across the FFI boundary, which is UB.
    if let Err(e) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        ctx.progress.on_progress(processed, total);
    })) {
        log::warn!("progress sink panicked: {:?}", e);
        // A panicked progress sink is unsafe to continue calling.
        return 0;
    }
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
    // SAFETY: ProgressSink call is wrapped in catch_unwind to prevent unwinding
    // across the FFI boundary, which is UB.
    if let Err(e) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        ectx.ctx.progress.on_file(&path_str);
    })) {
        log::warn!("file callback sink panicked: {:?}", e);
    }
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
    src: *const c_char,
    dest: *const c_char,
    existing_size: u64,
    src_size: u64,
    src_mtime: i64,
    dest_mtime: i64,
    user_data: *mut c_void,
) -> i32 {
    // SAFETY: user_data points to a valid ExtractCtx on the same thread.
    let ectx = unsafe { &*(user_data as *const ExtractCtx) };

    // Check if we have a global decision (from "apply to all")
    if ectx.has_global_decision.load(Ordering::Relaxed) {
        return if ectx.global_decision.load(Ordering::Relaxed) { 0 } else { 1 };
    }

    let src_path = unsafe { CStr::from_ptr(src) }.to_string_lossy().into_owned();
    let dest_path = unsafe { CStr::from_ptr(dest) }.to_string_lossy().into_owned();

    let info = OverwriteInfo {
        src_path,
        dest_path,
        existing_size,
        src_size,
        src_mtime,
        dest_mtime,
    };

    let decision = match ectx.overwrite {
        OverwriteMode::Overwrite => OverwriteDecision::Overwrite,
        OverwriteMode::Skip => OverwriteDecision::Skip,
        OverwriteMode::Ask => {
            // Use the resolver if available, otherwise default to overwrite
            if let Some(ref resolver) = ectx.resolver {
                resolver.resolve(&info)
            } else {
                OverwriteDecision::Overwrite
            }
        }
        OverwriteMode::RenameExtracted => {
            // For rename, we extract to a unique name by appending a counter.
            // The actual rename logic would need FFI support to modify dest path.
            // For now, fall back to overwrite with a warning.
            log::warn!("RenameExtracted not fully supported, falling back to overwrite");
            OverwriteDecision::Overwrite
        }
    };

    // Handle "apply to all" decisions
    match decision {
        OverwriteDecision::OverwriteAll => {
            ectx.global_decision.store(true, Ordering::Relaxed);
            ectx.has_global_decision.store(true, Ordering::Relaxed);
            0
        }
        OverwriteDecision::SkipAll => {
            ectx.global_decision.store(false, Ordering::Relaxed);
            ectx.has_global_decision.store(true, Ordering::Relaxed);
            1
        }
        OverwriteDecision::Overwrite => 0,
        OverwriteDecision::Skip => 1,
    }
}

/// FFI trampoline called by bit7z for compression file-begin events.
///
/// # Safety
///
/// Same safety invariants as `progress_trampoline`. `user_data` points to a
/// valid `ExtractCtx` that outlives this callback. `path` points to a
/// null-terminated C string valid for the duration of the callback.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn compress_file_callback(
    path: *const c_char,
    user_data: *mut c_void,
) {
    // SAFETY: user_data points to a valid ExtractCtx owned by the actor thread.
    // path is a null-terminated C string provided by the FFI layer.
    let ectx = unsafe { &*(user_data as *const ExtractCtx) };

    let path_str = unsafe { CStr::from_ptr(path) }
        .to_string_lossy()
        .into_owned();

    // SAFETY: ProgressSink call is wrapped in catch_unwind to prevent unwinding
    // across the FFI boundary, which is UB.
    if let Err(e) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        ectx.ctx.progress.on_file(&path_str);
    })) {
        log::warn!("compress file callback sink panicked: {:?}", e);
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
