use crate::adapters::view_models::archive_vm::ArchiveViewModel;
use gpui::*;
use gpui_component::button::Button;

pub struct Toolbar {
    archive_vm: Entity<ArchiveViewModel>,
}

impl Toolbar {
    pub fn new(archive_vm: Entity<ArchiveViewModel>) -> Self {
        Self { archive_vm }
    }
}

impl Render for Toolbar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {

        gpui_component::h_flex().gap_2().p_2()
            .child(
                Button::new("open")
                    .label("Open")
                    .on_click({
                        let vm = self.archive_vm.clone();
                        move |_, _, cx| {
                            if let Some(path) = pick_archive_file_windows() {
                                vm.update(cx, |vm, cx| {
                                    vm.open_archive(&path, None, cx);
                                });
                            }
                        }
                    })
            )
            .child(
                Button::new("create")
                    .label("Create")
                    .on_click({
                        let vm = self.archive_vm.clone();
                        move |_, _, cx| {
                            vm.update(cx, |vm, cx| vm.request_create(cx));
                        }
                    })
            )
            .child(
                Button::new("extract")
                    .label("Extract")
                    .on_click({
                        let vm = self.archive_vm.clone();
                        move |_, _, cx| {
                            vm.update(cx, |vm, cx| vm.request_extract(cx));
                        }
                    })
            )
            .child(
                Button::new("test")
                    .label("Test")
                    .on_click({
                        let vm = self.archive_vm.clone();
                        move |_, _, cx| {
                            vm.update(cx, |vm, cx| vm.request_test(cx));
                        }
                    })
            )
    }
}

// ---------------------------------------------------------------------------
// Raw Win32 file dialog via GetOpenFileNameW (avoids windows-sys feature deps)
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
fn pick_archive_file_windows() -> Option<std::path::PathBuf> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;

    // OPENFILENAMEW struct (declared manually to avoid windows-sys dependency)
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
        // Filter: "Archive files\0*.7z;*.zip;*.rar\0All files\0*.*\0"
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
            // Find the null terminator
            let len = (0..file_buf.len())
                .find(|&i| file_buf[i] == 0)
                .unwrap_or(0);
            if len > 0 {
                let os_str = OsString::from_wide(&file_buf[..len]);
                return Some(std::path::PathBuf::from(os_str));
            }
        }
    }
    None
}

#[cfg(not(target_os = "windows"))]
fn pick_archive_file_windows() -> Option<std::path::PathBuf> {
    None
}
