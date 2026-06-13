//! JSON-line IPC protocol for parent-worker and inter-process communication.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WorkerMessage {
    #[serde(rename = "progress")]
    Progress {
        current: u64,
        total: u64,
        #[serde(default)]
        file: String,
        #[serde(default)]
        bytes: u64,
    },
    #[serde(rename = "conflict")]
    Conflict {
        path: String,
        existing_size: u64,
        incoming_size: u64,
    },
    #[serde(rename = "complete")]
    Complete {
        total_files: u64,
        total_bytes: u64,
        duration_ms: u64,
    },
    #[serde(rename = "error")]
    Error {
        code: i32,
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ParentMessage {
    #[serde(rename = "conflict_resolution")]
    ConflictResolution {
        action: String,  // "overwrite" | "skip" | "rename"
        #[serde(default)]
        new_name: Option<String>,
        #[serde(default)]
        apply_to_all: bool,
    },
    #[serde(rename = "cancel")]
    Cancel,
}
