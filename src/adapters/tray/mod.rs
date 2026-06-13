//! System tray icon manager.

pub mod linux;
pub mod windows;

use crossbeam::channel::{Sender, Receiver};

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
    _thread: std::thread::JoinHandle<()>,
}

impl TrayManager {
    pub fn new() -> Self {
        let (cmd_tx, cmd_rx) = crossbeam::channel::unbounded();
        let (event_tx, event_rx) = crossbeam::channel::unbounded();
        let _thread = std::thread::spawn(move || {
            // Platform-specific tray loop
            let _ = cmd_rx;
            let _ = event_tx;
        });
        Self { cmd_tx, event_rx, _thread }
    }

    pub fn send(&self, cmd: TrayCommand) {
        let _ = self.cmd_tx.send(cmd);
    }
}
