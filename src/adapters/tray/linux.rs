use super::{TrayCommand, TrayEvent};
use crossbeam::channel::{Receiver, Sender};

pub fn run_tray_loop_linux(_cmd_rx: Receiver<TrayCommand>, _event_tx: Sender<TrayEvent>) {
    // D-Bus StatusNotifierItem
    // Uses zbus for D-Bus communication with the notification area.
    // TODO: implement when zbus dependency is added.
    tracing::info!("Tray loop started (Linux — stub)");
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
        // Check for exit command
        if let Ok(cmd) = _cmd_rx.try_recv() {
            if matches!(cmd, TrayCommand::Exit) { break; }
        }
    }
}
