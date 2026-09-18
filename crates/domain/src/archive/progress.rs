use crate::archive::*;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

#[derive(Debug, Clone)]
pub struct ProgressUpdate {
    pub file_current: u64,
    pub file_total: u64,
    pub current_file: Option<String>,
    pub items_done: u64,
    pub items_total: u64,
    pub bytes_done: u64,
    pub bytes_total: u64,
    pub error: Option<String>,
}

impl Default for ProgressUpdate {
    fn default() -> Self {
        Self {
            file_current: 0,
            file_total: 0,
            current_file: None,
            items_done: 0,
            items_total: 0,
            bytes_done: 0,
            bytes_total: 0,
            error: None,
        }
    }
}

pub trait ProgressNotifier: Send + Sync {
    fn notify(&self, update: &ProgressUpdate);
}

/// A no-op notifier that discards all progress updates.
pub struct NoopNotifier;

impl ProgressNotifier for NoopNotifier {
    fn notify(&self, _update: &ProgressUpdate) {}
}

pub struct ExtractOptions {
    pub overwrite_mode: OverwriteMode,
    pub cancel: Arc<AtomicBool>,
    pub paused: Arc<AtomicBool>,
    pub notifier: Arc<dyn ProgressNotifier>,
}

pub struct WriteOptions {
    pub cancel: Arc<AtomicBool>,
    pub paused: Arc<AtomicBool>,
    pub notifier: Arc<dyn ProgressNotifier>,
}



