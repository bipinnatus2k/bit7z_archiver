use crossbeam::channel::{unbounded, Receiver, Sender};

pub use crate::domain::repository::{ProgressNotifier, ProgressUpdate};

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
