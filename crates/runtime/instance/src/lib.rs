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
    #[allow(dead_code)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_path_key_consistent() {
        let p = Path::new(r"C:\test\archive.7z");
        let k1 = path_key(p);
        let k2 = path_key(p);
        assert_eq!(k1, k2);
        assert!(k1.starts_with("bit7z_"));
    }

    #[test]
    fn test_path_key_different_for_different_paths() {
        let k1 = path_key(Path::new(r"C:\a.7z"));
        let k2 = path_key(Path::new(r"C:\b.7z"));
        assert_ne!(k1, k2);
    }

    #[test]
    fn test_path_key_handles_unicode() {
        let k = path_key(Path::new("\\server\\共享\\文件.7z"));
        assert!(k.starts_with("bit7z_"));
        assert_eq!(k.len(), 6 + 16);
    }

    #[test]
    fn test_path_key_length() {
        let k = path_key(Path::new("test.7z"));
        assert_eq!(k.len(), 22); // "bit7z_" + 16 hex chars
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_acquire_release() {
        let p = Path::new(r"C:\unlikely_to_exist_lock_test_12345.7z");
        let lock = acquire(p);
        // Should succeed — no other instance holds this key
        assert!(lock.is_ok());
        // Dropping releases the lock
        drop(lock);
        // Acquiring again should also succeed
        let lock2 = acquire(p);
        assert!(lock2.is_ok());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_acquire_twice_returns_already_open() {
        let p = Path::new(r"C:\unlikely_to_exist_lock_test_67890.7z");
        let _lock = acquire(p).expect("first acquire should succeed");
        let second = acquire(p);
        assert!(matches!(second, Err(InstanceError::AlreadyOpen)));
    }
}
