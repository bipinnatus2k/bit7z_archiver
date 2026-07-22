//! System tray icon manager.

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "windows")]
pub mod windows;

use crossbeam_channel::{Receiver, Sender};
use gpui::Global;
use std::sync::Arc;

#[derive(Clone)]
pub struct TrayGlobal(pub Arc<TrayManager>);
impl Global for TrayGlobal {}

pub enum TrayCommand {
    Show,
    Hide,
    UpdateProgress { message: String, percent: f32 },
    Idle,
    Exit,
}

pub enum TrayEvent {
    LeftClick,
    RightClick,
}

pub struct TrayManager {
    pub cmd_tx: Sender<TrayCommand>,
    pub event_rx: Receiver<TrayEvent>,
}

impl TrayManager {
    pub fn new() -> Self {
        let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded();
        let (event_tx, event_rx) = crossbeam_channel::unbounded();
        std::thread::spawn(move || {
            #[cfg(target_os = "windows")]
            windows::run_tray_loop_windows(cmd_rx, event_tx);
            #[cfg(target_os = "linux")]
            linux::run_tray_loop_linux(cmd_rx, event_tx);
        });
        Self { cmd_tx, event_rx }
    }

    pub fn send(&self, cmd: TrayCommand) {
        let _ = self.cmd_tx.send(cmd);
    }
}
