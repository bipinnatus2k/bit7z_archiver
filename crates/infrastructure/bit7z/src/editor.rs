// ============================================================================
// Editor
// ============================================================================

use crate::{bit7z_editor_apply, bit7z_editor_close, bit7z_editor_delete, bit7z_editor_open, bit7z_editor_rename};
use crate::handle::Handle;
use crate::library::Library;
use crate::writer::WriterFormat;

pub struct Editor {
    raw: Handle,
}

// SAFETY: Editor wraps a raw FFI handle to a C++ archive editor.
// Editor instances are short-lived and used within a single operation.
// All access is serialized through Bit7zRepository's Mutex-protected library lock.
unsafe impl Send for Editor {}
unsafe impl Sync for Editor {}

impl Editor {
    pub fn open(
        lib: &Library,
        path: &str,
        format: WriterFormat,
        password: Option<&str>,
    ) -> Result<Self, String> {
        let c_path = std::ffi::CString::new(path).map_err(|e| format!("{}", e))?;
        let c_pw = password.and_then(|p| std::ffi::CString::new(p).ok());
        let raw = unsafe {
            bit7z_editor_open(
                lib.raw_handle().as_ptr(),
                c_path.as_ptr(),
                format as i32,
                c_pw.as_ref().map_or(std::ptr::null(), |s| s.as_ptr()),
            )
        };
        if raw.is_null() {
            Err("failed to open editor".into())
        } else {
            Ok(Self {
                raw: Handle::from_raw(raw),
            })
        }
    }

    pub fn rename(&self, index: u32, new_path: &str) -> Result<(), String> {
        let c_path = std::ffi::CString::new(new_path).map_err(|e| format!("{}", e))?;
        let ret = unsafe { bit7z_editor_rename(self.raw.as_ptr(), index, c_path.as_ptr()) };
        if ret == 0 {
            Ok(())
        } else {
            Err("rename failed".into())
        }
    }

    pub fn delete(&self, index: u32) -> Result<(), String> {
        let ret = unsafe { bit7z_editor_delete(self.raw.as_ptr(), index) };
        if ret == 0 {
            Ok(())
        } else {
            Err("delete failed".into())
        }
    }

    pub fn apply(&self) -> Result<(), String> {
        let ret = unsafe { bit7z_editor_apply(self.raw.as_ptr()) };
        if ret == 0 {
            Ok(())
        } else {
            Err("apply changes failed".into())
        }
    }
}

impl Drop for Editor {
    fn drop(&mut self) {
        unsafe {
            bit7z_editor_close(self.raw.as_ptr());
        }
    }
}
