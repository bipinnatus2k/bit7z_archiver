//! Per-file instance lock for single-instance-per-archive behavior.
use std::path::Path;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[derive(Debug)]
pub enum InstanceError { AlreadyOpen, Internal(String) }

pub fn path_key(path: &Path) -> String {
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    format!("bit7z_{:016x}", hasher.finish())
}

#[cfg(target_os = "windows")]
mod win {
    use super::*;
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    pub fn acquire(path: &Path) -> Result<super::InstanceLockWin, super::InstanceError> {
        let key = super::path_key(path);
        let wide: Vec<u16> = OsStr::new(&key).encode_wide().chain(std::iter::once(0)).collect();
        unsafe {
            let handle = windows_sys::Win32::System::Threading::CreateMutexW(std::ptr::null(), 1, wide.as_ptr());
            if handle.is_null() { return Err(super::InstanceError::Internal("CreateMutex failed".into())); }
            if windows_sys::Win32::Foundation::GetLastError() == windows_sys::Win32::Foundation::ERROR_ALREADY_EXISTS {
                windows_sys::Win32::Foundation::CloseHandle(handle);
                return Err(super::InstanceError::AlreadyOpen);
            }
            Ok(super::InstanceLockWin { handle, key })
        }
    }
    pub struct InstanceLockWin { handle: windows_sys::Win32::Foundation::HANDLE, key: String }
    impl Drop for InstanceLockWin {
        fn drop(&mut self) { unsafe { windows_sys::Win32::Foundation::CloseHandle(self.handle); } }
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use std::os::unix::net::UnixListener;
    pub fn acquire(path: &Path) -> Result<super::InstanceLockLinux, super::InstanceError> {
        let key = super::path_key(path);
        let addr = std::os::unix::net::SocketAddr::new_abstract(key.as_bytes())
            .map_err(|e| super::InstanceError::Internal(e.to_string()))?;
        match UnixListener::bind_addr(&addr) {
            Ok(l) => Ok(super::InstanceLockLinux { listener: Some(l), key }),
            Err(_) => Err(super::InstanceError::AlreadyOpen),
        }
    }
    pub struct InstanceLockLinux { listener: Option<UnixListener>, key: String }
}

#[cfg(target_os = "windows")] pub use win::*;
#[cfg(target_os = "linux")] pub use linux::*;