use super::{TrayCommand, TrayEvent};
use crossbeam::channel::{Receiver, Sender};

pub fn run_tray_loop_windows(cmd_rx: Receiver<TrayCommand>, event_tx: Sender<TrayEvent>) {
    // Win32 tray icon message pump
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    unsafe {
        // Register window class
        let class_name: Vec<u16> = OsStr::new("Bit7zTrayClass")
            .encode_wide().chain(std::iter::once(0)).collect();

        let wc = windows_sys::Win32::UI::WindowsAndMessaging::WNDCLASSEXW {
            cbSize: std::mem::size_of::<windows_sys::Win32::UI::WindowsAndMessaging::WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(tray_wndproc),
            lpszClassName: class_name.as_ptr(),
            style: Default::default(),
            cbClsExtra: Default::default(),
            cbWndExtra: Default::default(),
            hInstance: Default::default(),
            hIcon: Default::default(),
            hCursor: Default::default(),
            hbrBackground: Default::default(),
            lpszMenuName: Default::default(),
            hIconSm: Default::default(),
        };

        let _atom = windows_sys::Win32::UI::WindowsAndMessaging::RegisterClassExW(&wc);
        let hwnd = windows_sys::Win32::UI::WindowsAndMessaging::CreateWindowExW(
            0, class_name.as_ptr(), class_name.as_ptr(), 0,
            0, 0, 0, 0, std::ptr::null_mut(), std::ptr::null_mut(),
            std::ptr::null_mut(), std::ptr::null_mut(),
        );

        // Create tray icon
        let mut nid: windows_sys::Win32::UI::Shell::NOTIFYICONDATAW = std::mem::zeroed();
        nid.cbSize = std::mem::size_of::<windows_sys::Win32::UI::Shell::NOTIFYICONDATAW>() as u32;
        nid.hWnd = hwnd;
        nid.uFlags = windows_sys::Win32::UI::Shell::NIF_MESSAGE | windows_sys::Win32::UI::Shell::NIF_TIP;
        nid.uCallbackMessage = 0x8001; // WM_APP+1
        std::ptr::write(nid.szTip.as_mut_ptr(), 0);
        windows_sys::Win32::UI::Shell::Shell_NotifyIconW(
            windows_sys::Win32::UI::Shell::NIM_ADD, &nid,
        );

        // Message loop
        let mut msg: windows_sys::Win32::UI::WindowsAndMessaging::MSG = std::mem::zeroed();
        loop {
            while windows_sys::Win32::UI::WindowsAndMessaging::PeekMessageW(
                &mut msg, std::ptr::null_mut(), 0, 0,
                windows_sys::Win32::UI::WindowsAndMessaging::PM_REMOVE,
            ) != 0 {
                if msg.message == 0x8001 {
                    // Tray icon clicked
                    match msg.lParam {
                        v if v == windows_sys::Win32::UI::WindowsAndMessaging::WM_LBUTTONUP as isize => {
                            let _ = event_tx.send(TrayEvent::LeftClick);
                        }
                        v if v == windows_sys::Win32::UI::WindowsAndMessaging::WM_RBUTTONUP as isize => {
                            let _ = event_tx.send(TrayEvent::RightClick);
                        }
                        _ => {}
                    }
                }
                windows_sys::Win32::UI::WindowsAndMessaging::TranslateMessage(&msg);
                windows_sys::Win32::UI::WindowsAndMessaging::DispatchMessageW(&msg);
            }

            // Check for commands from GPUI thread
            if let Ok(cmd) = cmd_rx.try_recv() {
                match cmd {
                    TrayCommand::Exit => break,
                    TrayCommand::UpdateProgress { message, percent: _ } => {
                        let tip: Vec<u16> = OsStr::new(&message)
                            .encode_wide().chain(std::iter::once(0)).collect();
                        std::ptr::copy_nonoverlapping(tip.as_ptr(), nid.szTip.as_mut_ptr(), tip.len().min(128));
                        windows_sys::Win32::UI::Shell::Shell_NotifyIconW(
                            windows_sys::Win32::UI::Shell::NIM_MODIFY, &nid,
                        );
                    }
                    _ => {}
                }
            }

            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        // Cleanup
        windows_sys::Win32::UI::Shell::Shell_NotifyIconW(
            windows_sys::Win32::UI::Shell::NIM_DELETE, &nid,
        );
    }
}

unsafe extern "system" fn tray_wndproc(
    hwnd: windows_sys::Win32::Foundation::HWND,
    msg: u32, wparam: windows_sys::Win32::Foundation::WPARAM,
    lparam: windows_sys::Win32::Foundation::LPARAM,
) -> windows_sys::Win32::Foundation::LRESULT {
    windows_sys::Win32::UI::WindowsAndMessaging::DefWindowProcW(hwnd, msg, wparam, lparam)
}


