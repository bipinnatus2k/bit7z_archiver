use bit7z_domain::repository::{ProgressNotifier, ProgressUpdate};
use crossbeam_channel::{Receiver, Sender, unbounded};

pub type ProgressSender = Sender<ProgressUpdate>;
pub type ProgressReceiver = Receiver<ProgressUpdate>;

pub fn progress_channel() -> (ProgressSender, ProgressReceiver) {
    unbounded()
}

/// Adapter: bridges crossbeam::Sender to the domain ProgressNotifier trait.
pub struct CrossbeamNotifier(pub ProgressSender);

impl ProgressNotifier for CrossbeamNotifier {
    fn notify(&self, update: &ProgressUpdate) {
        let _ = self.0.send(update.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_channel_roundtrip() {
        let (tx, rx) = progress_channel();
        let update = ProgressUpdate {
            file_current: 50,
            file_total: 100,
            current_file: Some("test.txt".into()),
            items_done: 1,
            items_total: 10,
            bytes_done: 500,
            bytes_total: 1000,
            error: None,
        };
        tx.send(update.clone()).unwrap();
        let received = rx.recv().unwrap();
        assert_eq!(received.file_current, 50);
        assert_eq!(received.current_file, Some("test.txt".into()));
    }

    #[test]
    fn test_crossbeam_notifier_forwards_update() {
        let (tx, rx) = progress_channel();
        let notifier = CrossbeamNotifier(tx);
        let update = ProgressUpdate::default();
        notifier.notify(&update);
        let received = rx.recv().unwrap();
        assert_eq!(received.file_current, 0);
    }

    #[test]
    fn test_progress_update_default_is_zero() {
        let u = ProgressUpdate::default();
        assert_eq!(u.file_current, 0);
        assert_eq!(u.file_total, 0);
        assert!(u.current_file.is_none());
        assert_eq!(u.items_done, 0);
        assert_eq!(u.items_total, 0);
        assert_eq!(u.bytes_done, 0);
        assert_eq!(u.bytes_total, 0);
        assert!(u.error.is_none());
    }
}
