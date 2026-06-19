use crossbeam::channel::{unbounded, Receiver, Sender};

// ProgressUpdate is defined in domain/repository.rs to avoid circular imports.
pub use crate::domain::repository::ProgressUpdate;

pub type ProgressSender = Sender<ProgressUpdate>;
pub type ProgressReceiver = Receiver<ProgressUpdate>;

pub fn progress_channel() -> (ProgressSender, ProgressReceiver) {
    unbounded()
}
