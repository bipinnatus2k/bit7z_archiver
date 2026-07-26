use std::sync::Arc;
use bit7z_domain::repository::{ProgressNotifier, ProgressUpdate};
use bit7z_ports::progress::ProgressReporter;

/// Bridge that forwards ProgressNotifier::notify() calls to a ProgressReporter.
pub struct ProgressBridge(pub Arc<dyn ProgressReporter>);

impl ProgressNotifier for ProgressBridge {
    fn notify(&self, update: &ProgressUpdate) {
        self.0.report(update.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bit7z_domain::repository::{ProgressNotifier, ProgressUpdate};
    use bit7z_ports::progress::ProgressReporter;
    use std::sync::{Arc, Mutex};

    struct CaptureReporter {
        updates: Mutex<Vec<ProgressUpdate>>,
    }

    impl ProgressReporter for CaptureReporter {
        fn report(&self, update: ProgressUpdate) {
            self.updates.lock().unwrap().push(update);
        }
    }

    #[test]
    fn test_progress_bridge_forwards_update() {
        let reporter = Arc::new(CaptureReporter { updates: Mutex::new(vec![]) });
        let bridge = ProgressBridge(reporter.clone());
        let update = ProgressUpdate {
            bytes_done: 50,
            bytes_total: 100,
            ..Default::default()
        };
        bridge.notify(&update);
        assert_eq!(reporter.updates.lock().unwrap().len(), 1);
        assert_eq!(reporter.updates.lock().unwrap()[0].bytes_done, 50);
    }
}
