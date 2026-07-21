use bit7z_infra_tray::TrayCommand;
use bit7z_infra_progress::ProgressReceiver;
use crossbeam_channel::{Sender, TryRecvError};
use gpui::*;
use std::sync::{Arc, Mutex};

/// Global progress state for long-running operations.
/// Used by dialogs and tray to track and display progress.
#[derive(Clone)]
pub struct ProgressState {
    pub is_active: bool,
    pub is_paused: bool,
    pub is_complete: bool,
    pub message: String,
    pub file_current: u64,
    pub file_total: u64,
    pub current_file: Option<String>,
    pub items_done: u64,
    pub items_total: u64,
    pub bytes_done: u64,
    pub bytes_total: u64,
    pub current: u64,
    pub total: u64,
    pub error: Option<String>,
    pub receiver: Option<Arc<Mutex<ProgressReceiver>>>,
    pub tray_sender: Option<Sender<TrayCommand>>,
    pub is_hidden_to_tray: bool,
}

impl Default for ProgressState {
    fn default() -> Self {
        Self {
            is_active: false,
            is_paused: false,
            is_complete: false,
            message: String::new(),
            file_current: 0,
            file_total: 0,
            current_file: None,
            items_done: 0,
            items_total: 0,
            bytes_done: 0,
            bytes_total: 0,
            current: 0,
            total: 0,
            error: None,
            receiver: None,
            tray_sender: None,
            is_hidden_to_tray: false,
        }
    }
}

impl Global for ProgressState {}

/// Register the global ProgressState with the given tray sender.
pub fn init(cx: &mut App, tray_sender: Sender<TrayCommand>) {
    let mut state = ProgressState::default();
    state.tray_sender = Some(tray_sender);
    cx.set_global(state);
}

impl ProgressState {
    pub fn start(message: impl Into<String>, total: u64, rx: ProgressReceiver, cx: &mut App) {
        cx.update_global::<Self, _>(|state, _cx| {
            state.is_active = true;
            state.is_complete = false;
            state.is_paused = false;
            state.message = message.into();
            state.current = 0;
            state.total = total;
            state.file_current = 0;
            state.file_total = 0;
            state.current_file = None;
            state.items_done = 0;
            state.items_total = 0;
            state.bytes_done = 0;
            state.bytes_total = 0;
            state.error = None;
            state.is_hidden_to_tray = false;
            state.receiver = Some(Arc::new(Mutex::new(rx)));
        });
    }

    pub fn update(current: u64, message: impl Into<String>, cx: &mut App) {
        cx.update_global::<Self, _>(|state, _cx| {
            state.current = current;
            state.message = message.into();
        });
    }

    pub fn complete(message: impl Into<String>, cx: &mut App) {
        cx.update_global::<Self, _>(|state, _cx| {
            state.is_active = false;
            state.is_complete = true;
            state.message = message.into();
            state.receiver = None;
            state.is_hidden_to_tray = false;
        });
    }

    pub fn error_msg(message: impl Into<String>, cx: &mut App) {
        cx.update_global::<Self, _>(|state, _cx| {
            state.is_active = false;
            state.is_complete = true;
            state.error = Some(message.into());
            state.receiver = None;
            state.is_hidden_to_tray = false;
        });
    }

    pub fn pause(cx: &mut App) {
        cx.update_global::<Self, _>(|state, _cx| {
            state.is_paused = true;
        });
    }

    pub fn resume(cx: &mut App) {
        cx.update_global::<Self, _>(|state, _cx| {
            state.is_paused = false;
        });
    }

    pub fn percent(&self) -> f32 {
        if self.total == 0 {
            0.0
        } else {
            self.current as f32 / self.total as f32
        }
    }

    pub fn poll(&mut self) -> bool {
        if self.is_paused {
            return false;
        }
        let mut updated = false;
        let mut disconnected = false;

        if let Some(ref rx_arc) = self.receiver {
            if let Ok(rx) = rx_arc.lock() {
                loop {
                    match rx.try_recv() {
                        Ok(update) => {
                            self.file_current = update.file_current;
                            self.file_total = update.file_total;
                            self.current_file = update.current_file;
                            self.items_done = update.items_done;
                            self.items_total = update.items_total;
                            self.bytes_done = update.bytes_done;
                            self.bytes_total = update.bytes_total;
                            self.current = update.bytes_done;
                            self.total = update.bytes_total;
                            if let Some(ref err) = update.error {
                                self.error = Some(err.clone());
                            }
                            updated = true;
                        }
                        Err(TryRecvError::Empty) => break,
                        Err(TryRecvError::Disconnected) => {
                            disconnected = true;
                            break;
                        }
                    }
                }
            }
        }

        if disconnected {
            self.is_active = false;
            self.is_complete = true;
            self.receiver = None;
            self.is_hidden_to_tray = false;
            updated = true;
        }

        if self.is_hidden_to_tray {
            if let Some(ref tx) = self.tray_sender {
                let pct = self.percent();
                let msg = if self.total > 0 {
                    format!("{} \u{2014} {}%", self.message, (pct * 100.0) as u32)
                } else {
                    self.message.clone()
                };
                let _ = tx.send(TrayCommand::UpdateProgress { message: msg, percent: pct });
            }
        }

        updated
    }
}

#[cfg(test)]
mod tests {
    use super::ProgressState;
    use crossbeam_channel;

    fn channel_pair() -> (crossbeam_channel::Sender<ProgressUpdate>, crossbeam_channel::Receiver<ProgressUpdate>) {
        crossbeam_channel::unbounded()
    }

    #[test]
    fn test_default_is_not_active() {
        let state = ProgressState::default();
        assert!(!state.is_active);
        assert!(!state.is_complete);
        assert!(!state.is_paused);
        assert!(state.error.is_none());
    }

    #[test]
    fn test_poll_updates_fields() {
        let mut state = ProgressState::default();
        let (tx, rx) = channel_pair();
        state.is_active = true;
        state.receiver = Some(std::sync::Arc::new(std::sync::Mutex::new(rx)));
        tx.send(ProgressUpdate {
            file_current: 3, file_total: 10,
            current_file: Some("test.txt".into()),
            items_done: 5, items_total: 20,
            bytes_done: 1024, bytes_total: 4096,
            error: None,
        }).unwrap();
        drop(tx);
        assert!(state.poll());
        assert_eq!(state.file_current, 3);
        assert_eq!(state.current_file, Some("test.txt".into()));
        assert_eq!(state.bytes_done, 1024);
    }

    #[test]
    fn test_is_complete_after_sender_drops() {
        let mut state = ProgressState::default();
        let (tx, rx) = channel_pair();
        state.is_active = true;
        state.receiver = Some(std::sync::Arc::new(std::sync::Mutex::new(rx)));
        drop(tx);
        state.poll();
        assert!(!state.is_active);
        assert!(state.is_complete);
    }

    #[test]
    fn test_pause_prevents_poll() {
        let mut state = ProgressState::default();
        let (tx, rx) = channel_pair();
        state.is_active = true;
        state.is_paused = true;
        state.receiver = Some(std::sync::Arc::new(std::sync::Mutex::new(rx)));
        tx.send(ProgressUpdate {
            file_current: 1, file_total: 5,
            current_file: None,
            items_done: 1, items_total: 5,
            bytes_done: 100, bytes_total: 500,
            error: None,
        }).unwrap();
        assert!(!state.poll());
        assert_eq!(state.file_current, 0);
    }

    #[test]
    fn test_error_capture() {
        let mut state = ProgressState::default();
        let (tx, rx) = channel_pair();
        state.is_active = true;
        state.receiver = Some(std::sync::Arc::new(std::sync::Mutex::new(rx)));
        tx.send(ProgressUpdate {
            file_current: 0, file_total: 1,
            current_file: None,
            items_done: 0, items_total: 1,
            bytes_done: 0, bytes_total: 0,
            error: Some("CRC mismatch".into()),
        }).unwrap();
        drop(tx);
        state.poll();
        assert_eq!(state.error, Some("CRC mismatch".into()));
    }

    #[test]
    fn test_percent_half() {
        let state = ProgressState { current: 50, total: 100, ..Default::default() };
        assert!((state.percent() - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_percent_zero_total() {
        assert_eq!(ProgressState::default().percent(), 0.0);
    }
}
