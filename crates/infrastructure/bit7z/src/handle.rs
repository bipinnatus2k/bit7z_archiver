use crate::{bit7z_editor_close, bit7z_writer_close};

/// Opaque handle wrapping a raw C++ pointer stored as usize.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Handle(usize);

impl Handle {
    #[inline]
    pub fn from_raw<T>(ptr: *mut T) -> Self {
        Handle(ptr as usize)
    }

    #[inline]
    pub fn as_ptr<T>(self) -> *mut T {
        self.0 as *mut T
    }

    #[inline]
    pub fn null() -> Self {
        Handle(0)
    }

    #[inline]
    pub fn is_null(self) -> bool {
        self.0 == 0
    }
}

// ============================================================================
// FfiHandle — RAII wrapper for C++ resource lifecycle
// ============================================================================

/// Identifies the kind of C++ resource held by an [`FfiHandle`], so that
/// `Drop` can call the correct C destructor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandleKind {
    Reader,
    Writer,
    Editor,
}

/// RAII wrapper around a raw C++ pointer stored in `Bit7zRepository.handles`.
///
/// When an `FfiHandle` is dropped it automatically calls the appropriate
/// C destructor (`bit7z_reader_close`, `bit7z_writer_close`, or
/// `bit7z_editor_close`), preventing resource leaks even on panic paths.
pub struct FfiHandle {
    ptr: *mut std::ffi::c_void,
    kind: HandleKind,
}

// SAFETY: FfiHandle wraps a raw FFI pointer. All access is serialized
// through the repository's Mutex, which ensures only one thread calls
// into the C++ bit7z library at a time on the same handle.
unsafe impl Send for FfiHandle {}
unsafe impl Sync for FfiHandle {}

impl FfiHandle {
    pub fn reader(ptr: *mut std::ffi::c_void) -> Self {
        Self {
            ptr,
            kind: HandleKind::Reader,
        }
    }

    pub fn writer(ptr: *mut std::ffi::c_void) -> Self {
        Self {
            ptr,
            kind: HandleKind::Writer,
        }
    }

    pub fn editor(ptr: *mut std::ffi::c_void) -> Self {
        Self {
            ptr,
            kind: HandleKind::Editor,
        }
    }

    pub fn ptr(&self) -> *mut std::ffi::c_void {
        self.ptr
    }

    pub fn kind(&self) -> HandleKind {
        self.kind
    }

    pub fn is_null(&self) -> bool {
        self.ptr.is_null()
    }
}

impl Drop for FfiHandle {
    fn drop(&mut self) {
        if self.ptr.is_null() {
            return;
        }
        unsafe {
            match self.kind {
                HandleKind::Reader => bit7z_ffi::bit7z_reader_close(self.ptr as *mut _),
                HandleKind::Writer => bit7z_writer_close(self.ptr as *mut _),
                HandleKind::Editor => bit7z_editor_close(self.ptr as *mut _),
            }
        }
    }
}
