use std::path::PathBuf;

/// Trait for platform-specific file dialog operations.
pub trait DialogProvider: Send + Sync {
    /// Pick a single archive file. Returns `None` if cancelled.
    fn pick_archive_file(&self) -> Option<PathBuf>;
    /// Pick a folder. Returns `None` if cancelled.
    fn pick_folder(&self) -> Option<PathBuf>;
    /// Pick one or more files. Returns `None` if cancelled.
    fn pick_files(&self) -> Option<Vec<PathBuf>>;
}

/// Returns the platform-specific dialog provider.
pub fn dialog_provider() -> Box<dyn DialogProvider> {
    #[cfg(target_os = "windows")]
    { Box::new(WinDialogProvider) }
    #[cfg(not(target_os = "windows"))]
    { Box::new(RfdDialogProvider) }
}

/// Find the 7-Zip shared library on the current platform.
pub fn find_7z_library() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        // Common install paths for 7-Zip
        let candidates = [
            r"C:\Program Files\7-Zip\7z.dll",
            r"C:\Program Files (x86)\7-Zip\7z.dll",
        ];
        for p in &candidates {
            if std::path::Path::new(p).exists() {
                return Some(PathBuf::from(p));
            }
        }
        // Also try PATH
        if let Ok(paths) = std::env::var("PATH") {
            for dir in std::env::split_paths(&paths) {
                let candidate = dir.join("7z.dll");
                if candidate.exists() {
                    return Some(candidate);
                }
            }
        }
    }
    #[cfg(target_os = "linux")]
    {
        // Common library names on Linux
        let lib_names = ["lib7zip.so", "libp7zip.so", "lib7z.so"];
        for name in &lib_names {
            // Check standard library paths
            for dir in ["/usr/lib", "/usr/local/lib", "/usr/lib/x86_64-linux-gnu"] {
                let candidate = PathBuf::from(dir).join(name);
                if candidate.exists() {
                    return Some(candidate);
                }
            }
        }
    }
    None
}

/// Pick an archive file using the native OS file dialog.
/// Returns `None` if the dialog is cancelled.
#[cfg(target_os = "windows")]
pub fn pick_archive_file() -> Option<std::path::PathBuf> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;

    #[allow(non_snake_case)]
    #[repr(C)]
    struct OPENFILENAMEW {
        lStructSize: u32,
        hwndOwner: isize,
        hInstance: isize,
        lpstrFilter: *const u16,
        lpstrCustomFilter: *mut u16,
        nMaxCustFilter: u32,
        nFilterIndex: u32,
        lpstrFile: *mut u16,
        nMaxFile: u32,
        lpstrFileTitle: *mut u16,
        nMaxFileTitle: u32,
        lpstrInitialDir: *const u16,
        lpstrTitle: *const u16,
        Flags: u32,
        nFileOffset: u16,
        nFileExtension: u16,
        lpstrDefExt: *const u16,
        lCustData: isize,
        lpfnHook: isize,
        lpTemplateName: *const u16,
        pvReserved: *mut std::ffi::c_void,
        dwReserved: u32,
        FlagsEx: u32,
    }

    const OFN_FILEMUSTEXIST: u32 = 0x00001000;
    const OFN_HIDEREADONLY: u32 = 0x00000004;
    const OFN_PATHMUSTEXIST: u32 = 0x00000800;

    #[link(name = "comdlg32")]
    unsafe extern "system" {
        fn GetOpenFileNameW(ofn: *mut OPENFILENAMEW) -> i32;
    }

    unsafe {
        let mut file_buf: Vec<u16> = vec![0u16; 4096];
        let filter = "Archive files\0*.7z;*.zip;*.rar;*.tar;*.tar.gz;*.tar.xz;*.tar.bz2;*.gz;*.xz;*.bz2\0All files\0*.*\0";
        let filter_wide: Vec<u16> = filter.encode_utf16().collect();

        let mut ofn = OPENFILENAMEW {
            lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
            hwndOwner: 0,
            hInstance: 0,
            lpstrFilter: filter_wide.as_ptr(),
            lpstrCustomFilter: std::ptr::null_mut(),
            nMaxCustFilter: 0,
            nFilterIndex: 1,
            lpstrFile: file_buf.as_mut_ptr(),
            nMaxFile: file_buf.len() as u32,
            lpstrFileTitle: std::ptr::null_mut(),
            nMaxFileTitle: 0,
            lpstrInitialDir: std::ptr::null(),
            lpstrTitle: std::ptr::null(),
            Flags: OFN_FILEMUSTEXIST | OFN_HIDEREADONLY | OFN_PATHMUSTEXIST,
            nFileOffset: 0,
            nFileExtension: 0,
            lpstrDefExt: std::ptr::null(),
            lCustData: 0,
            lpfnHook: 0,
            lpTemplateName: std::ptr::null(),
            pvReserved: std::ptr::null_mut(),
            dwReserved: 0,
            FlagsEx: 0,
        };

        if GetOpenFileNameW(&mut ofn) != 0 {
            let len = (0..file_buf.len()).find(|&i| file_buf[i] == 0).unwrap_or(0);
            if len > 0 {
                let os_str = OsString::from_wide(&file_buf[..len]);
                return Some(std::path::PathBuf::from(os_str));
            }
        }
    }
    None
}

#[cfg(not(target_os = "windows"))]
pub fn pick_archive_file() -> Option<std::path::PathBuf> {
    rfd::FileDialog::new()
        .add_filter(
            "Archives",
            &[
                "7z", "zip", "rar", "tar", "tar.gz", "tar.xz", "tar.bz2", "gz", "bz2", "xz",
            ],
        )
        .pick_file()
}

/// Pick a folder using the native OS folder dialog.
/// Returns `None` if the dialog is cancelled.
#[cfg(target_os = "windows")]
pub fn pick_folder() -> Option<std::path::PathBuf> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;

    #[allow(non_snake_case)]
    #[repr(C)]
    struct BROWSEINFOW {
        hwndOwner: isize,
        pidlRoot: isize,
        pszDisplayName: *mut u16,
        lpszTitle: *const u16,
        ulFlags: u32,
        lpfn: isize,
        lParam: isize,
        iImage: i32,
    }

    const BIF_RETURNONLYFSDIRS: u32 = 0x0001;
    #[allow(dead_code)]
    const BIF_DONTGOBELOWDOMAIN: u32 = 0x0002;
    #[allow(dead_code)]
    const BIF_STATUSTEXT: u32 = 0x0004;
    #[allow(dead_code)]
    const BIF_RETURNFSANCESTORS: u32 = 0x0008;
    const BIF_EDITBOX: u32 = 0x0010;
    #[allow(dead_code)]
    const BIF_VALIDATE: u32 = 0x0020;
    const BIF_NEWDIALOGSTYLE: u32 = 0x0040;
    const BIF_USENEWUI: u32 = BIF_EDITBOX | BIF_NEWDIALOGSTYLE;
    #[allow(dead_code)]
    const BIF_BROWSEINCLUDEURLS: u32 = 0x0080;
    #[allow(dead_code)]
    const BIF_BROWSEFORCOMPUTER: u32 = 0x1000;
    #[allow(dead_code)]
    const BIF_BROWSEFORPRINTER: u32 = 0x2000;
    #[allow(dead_code)]
    const BIF_BROWSEINCLUDEFILES: u32 = 0x4000;
    #[allow(dead_code)]
    const BIF_SHAREABLE: u32 = 0x8000;

    #[link(name = "shell32")]
    unsafe extern "system" {
        fn SHBrowseForFolderW(lpbi: *const BROWSEINFOW) -> isize;
        fn SHGetPathFromIDListW(pidl: isize, pszPath: *mut u16) -> i32;
        fn CoTaskMemFree(pv: isize);
    }

    unsafe {
        let mut display_name: Vec<u16> = vec![0u16; 260];
        let title = "Select destination folder";
        let title_wide: Vec<u16> = title.encode_utf16().chain(Some(0)).collect();

        let bi = BROWSEINFOW {
            hwndOwner: 0,
            pidlRoot: 0,
            pszDisplayName: display_name.as_mut_ptr(),
            lpszTitle: title_wide.as_ptr(),
            ulFlags: BIF_RETURNONLYFSDIRS | BIF_USENEWUI,
            lpfn: 0,
            lParam: 0,
            iImage: 0,
        };

        let pidl = SHBrowseForFolderW(&bi);
        if pidl != 0 {
            let mut path_buf: Vec<u16> = vec![0u16; 4096];
            if SHGetPathFromIDListW(pidl, path_buf.as_mut_ptr()) != 0 {
                let len = (0..path_buf.len()).find(|&i| path_buf[i] == 0).unwrap_or(0);
                if len > 0 {
                    let os_str = OsString::from_wide(&path_buf[..len]);
                    CoTaskMemFree(pidl);
                    return Some(std::path::PathBuf::from(os_str));
                }
            }
            CoTaskMemFree(pidl);
        }
    }
    None
}

#[cfg(not(target_os = "windows"))]
pub fn pick_folder() -> Option<std::path::PathBuf> {
    rfd::FileDialog::new().pick_folder()
}

/// Pick multiple files using the native OS file dialog.
/// Returns `None` if the dialog is cancelled.
#[cfg(target_os = "windows")]
pub fn pick_files() -> Option<Vec<std::path::PathBuf>> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;

    const OFN_ALLOWMULTISELECT: u32 = 0x00000200;
    const OFN_FILEMUSTEXIST: u32 = 0x00001000;
    const OFN_HIDEREADONLY: u32 = 0x00000004;
    const OFN_PATHMUSTEXIST: u32 = 0x00000800;

    #[allow(non_snake_case)]
    #[repr(C)]
    struct OPENFILENAMEW {
        lStructSize: u32,
        hwndOwner: isize,
        hInstance: isize,
        lpstrFilter: *const u16,
        lpstrCustomFilter: *mut u16,
        nMaxCustFilter: u32,
        nFilterIndex: u32,
        lpstrFile: *mut u16,
        nMaxFile: u32,
        lpstrFileTitle: *mut u16,
        nMaxFileTitle: u32,
        lpstrInitialDir: *const u16,
        lpstrTitle: *const u16,
        Flags: u32,
        nFileOffset: u16,
        nFileExtension: u16,
        lpstrDefExt: *const u16,
        lCustData: isize,
        lpfnHook: isize,
        lpTemplateName: *const u16,
        pvReserved: *mut std::ffi::c_void,
        dwReserved: u32,
        FlagsEx: u32,
    }

    #[link(name = "comdlg32")]
    unsafe extern "system" {
        fn GetOpenFileNameW(ofn: *mut OPENFILENAMEW) -> i32;
    }

    unsafe {
        let mut file_buf: Vec<u16> = vec![0u16; 32768];
        let filter = "All files\0*.*\0";
        let filter_wide: Vec<u16> = filter.encode_utf16().collect();

        let mut ofn = OPENFILENAMEW {
            lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
            hwndOwner: 0,
            hInstance: 0,
            lpstrFilter: filter_wide.as_ptr(),
            lpstrCustomFilter: std::ptr::null_mut(),
            nMaxCustFilter: 0,
            nFilterIndex: 1,
            lpstrFile: file_buf.as_mut_ptr(),
            nMaxFile: file_buf.len() as u32,
            lpstrFileTitle: std::ptr::null_mut(),
            nMaxFileTitle: 0,
            lpstrInitialDir: std::ptr::null(),
            lpstrTitle: std::ptr::null(),
            Flags: OFN_ALLOWMULTISELECT | OFN_FILEMUSTEXIST | OFN_HIDEREADONLY | OFN_PATHMUSTEXIST,
            nFileOffset: 0,
            nFileExtension: 0,
            lpstrDefExt: std::ptr::null(),
            lCustData: 0,
            lpfnHook: 0,
            lpTemplateName: std::ptr::null(),
            pvReserved: std::ptr::null_mut(),
            dwReserved: 0,
            FlagsEx: 0,
        };

        if GetOpenFileNameW(&mut ofn) != 0 {
            let buf = &file_buf;
            let first_end = (0..buf.len()).find(|&i| buf[i] == 0).unwrap_or(0);
            if first_end == 0 {
                return None;
            }
            let first_str = OsString::from_wide(&buf[..first_end]);
            let first_path = std::path::PathBuf::from(first_str);

            if first_end + 1 < buf.len() && buf[first_end + 1] == 0 {
                // Single file selected
                return Some(vec![first_path]);
            }

            // Multiple files: first element is directory, rest are filenames
            let mut result = Vec::new();
            let mut pos = first_end + 1;
            while pos < buf.len() && buf[pos] != 0 {
                let end = (pos..buf.len()).find(|&i| buf[i] == 0).unwrap_or(buf.len());
                if end > pos {
                    let fname = OsString::from_wide(&buf[pos..end]);
                    let full = first_path.join(std::path::PathBuf::from(fname));
                    result.push(full);
                }
                pos = end + 1;
            }
            if result.is_empty() {
                result.push(first_path);
            }
            Some(result)
        } else {
            None
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn pick_files() -> Option<Vec<std::path::PathBuf>> {
    rfd::FileDialog::new().pick_files()
}

// ============================================================================
// DialogProvider implementations
// ============================================================================

/// Windows dialog provider using raw Win32 API.
#[cfg(target_os = "windows")]
pub struct WinDialogProvider;

#[cfg(target_os = "windows")]
impl DialogProvider for WinDialogProvider {
    fn pick_archive_file(&self) -> Option<PathBuf> {
        pick_archive_file()
    }

    fn pick_folder(&self) -> Option<PathBuf> {
        pick_folder()
    }

    fn pick_files(&self) -> Option<Vec<PathBuf>> {
        pick_files()
    }
}

/// Non-Windows dialog provider using the `rfd` crate.
#[cfg(not(target_os = "windows"))]
pub struct RfdDialogProvider;

#[cfg(not(target_os = "windows"))]
impl DialogProvider for RfdDialogProvider {
    fn pick_archive_file(&self) -> Option<PathBuf> {
        pick_archive_file()
    }

    fn pick_folder(&self) -> Option<PathBuf> {
        pick_folder()
    }

    fn pick_files(&self) -> Option<Vec<PathBuf>> {
        pick_files()
    }
}
