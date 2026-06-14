use std::path::PathBuf;

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
    extern "system" {
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
    None
}
