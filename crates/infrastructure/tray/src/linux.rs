use super::{TrayCommand, TrayEvent};
use crossbeam_channel::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use zbus::blocking::Connection;
use zbus::dbus_interface;

pub struct TraySharedState {
    tooltip_title: String,
    tooltip_body: String,
}

pub struct TrayIcon {
    event_tx: Sender<TrayEvent>,
    state: Arc<Mutex<TraySharedState>>,
}

#[dbus_interface(name = "org.kde.StatusNotifierItem")]
impl TrayIcon {
    #[dbus_interface(property)]
    fn category(&self) -> &str {
        "ApplicationStatus"
    }

    #[dbus_interface(property)]
    fn id(&self) -> &str {
        "bit7z_archiver"
    }

    #[dbus_interface(property)]
    fn title(&self) -> &str {
        "bit7z Archiver"
    }

    #[dbus_interface(property)]
    fn status(&self) -> &str {
        "Active"
    }

    #[dbus_interface(property)]
    fn window_id(&self) -> i32 {
        0
    }

    #[dbus_interface(property)]
    fn icon_name(&self) -> &str {
        "package-x-generic"
    }

    #[dbus_interface(property)]
    fn item_is_menu(&self) -> bool {
        false
    }

    #[dbus_interface(property)]
    fn tool_tip(
        &self,
    ) -> (
        String,
        Vec<(i32, i32, Vec<u8>)>,
        String,
        String,
    ) {
        let s = self.state.lock().unwrap();
        (
            s.tooltip_title.clone(),
            vec![],
            s.tooltip_body.clone(),
            String::new(),
        )
    }

    fn activate(&self, _x: i32, _y: i32) {
        let _ = self.event_tx.send(TrayEvent::LeftClick);
    }

    fn secondary_activate(&self, _x: i32, _y: i32) {
        let _ = self.event_tx.send(TrayEvent::RightClick);
    }

    fn context_menu(&self, _x: i32, _y: i32) {
        let _ = self.event_tx.send(TrayEvent::RightClick);
    }
}

pub fn run_tray_loop_linux(cmd_rx: Receiver<TrayCommand>, event_tx: Sender<TrayEvent>) {
    let state = Arc::new(Mutex::new(TraySharedState {
        tooltip_title: "bit7z Archiver".to_string(),
        tooltip_body: "Archive manager".to_string(),
    }));

    let conn = match Connection::session() {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("D-Bus session unavailable ({}) — tray disabled", e);
            fallback_loop(cmd_rx);
            return;
        }
    };

    if let Err(e) = conn.request_name("org.bit7z.TrayIcon") {
        tracing::warn!("Failed to register D-Bus name ({}) — tray disabled", e);
        fallback_loop(cmd_rx);
        return;
    }

    let icon = TrayIcon {
        event_tx: event_tx.clone(),
        state: state.clone(),
    };

    if let Err(e) = conn.object_server().at("/org/bit7z/TrayIcon", icon) {
        tracing::warn!("Failed to register tray D-Bus object ({}) — tray disabled", e);
        fallback_loop(cmd_rx);
        return;
    }

    tracing::info!("Linux tray icon registered via D-Bus StatusNotifierItem");

    loop {
        match cmd_rx.try_recv() {
            Ok(TrayCommand::Exit) => break,
            Ok(TrayCommand::UpdateProgress { message, percent }) => {
                let tooltip = format!("bit7z — {} {}%", message, percent as u32);
                {
                    let mut s = state.lock().unwrap();
                    s.tooltip_title = tooltip.clone();
                    s.tooltip_body = tooltip;
                }
                let _ = emit_new_tool_tip(&conn);
            }
            Ok(_) => {}
            Err(crossbeam_channel::TryRecvError::Empty) => {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            Err(crossbeam_channel::TryRecvError::Disconnected) => break,
        }
    }

    tracing::info!("Linux tray loop exiting");
}

fn emit_new_tool_tip(conn: &Connection) -> zbus::Result<()> {
    conn.emit_signal(
        "/org/bit7z/TrayIcon",
        "org.kde.StatusNotifierItem",
        "NewToolTip",
        &(),
    )
}

fn fallback_loop(cmd_rx: Receiver<TrayCommand>) {
    tracing::info!("Tray loop started (Linux — stub, no D-Bus)");
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
        if let Ok(cmd) = cmd_rx.try_recv() {
            if matches!(cmd, TrayCommand::Exit) {
                break;
            }
        }
    }
}
