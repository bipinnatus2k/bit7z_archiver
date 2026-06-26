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
        action: String,
        #[serde(default)]
        new_name: Option<String>,
        #[serde(default)]
        apply_to_all: bool,
    },
    #[serde(rename = "cancel")]
    Cancel,
}

/// Commands sent from CLI to an existing GUI instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum GuiCommand {
    #[serde(rename = "open")]
    Open { path: String, password: Option<String> },
    #[serde(rename = "activate")]
    Activate,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worker_message_progress_roundtrip() {
        let msg = WorkerMessage::Progress {
            current: 50, total: 100,
            file: "data.bin".into(), bytes: 2048,
        };
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: WorkerMessage = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, WorkerMessage::Progress { current: 50, total: 100, file, bytes: 2048 } if file == "data.bin"));
    }

    #[test]
    fn test_worker_message_conflict_roundtrip() {
        let msg = WorkerMessage::Conflict {
            path: "output.txt".into(), existing_size: 100, incoming_size: 200,
        };
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: WorkerMessage = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, WorkerMessage::Conflict { path, existing_size: 100, incoming_size: 200 } if path == "output.txt"));
    }

    #[test]
    fn test_worker_message_complete_roundtrip() {
        let msg = WorkerMessage::Complete { total_files: 5, total_bytes: 10000, duration_ms: 1500 };
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: WorkerMessage = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, WorkerMessage::Complete { total_files: 5, total_bytes: 10000, duration_ms: 1500 }));
    }

    #[test]
    fn test_worker_message_error_roundtrip() {
        let msg = WorkerMessage::Error { code: -1, message: "corrupt data".into() };
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: WorkerMessage = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, WorkerMessage::Error { code: -1, message } if message == "corrupt data"));
    }

    #[test]
    fn test_parent_message_conflict_resolution_roundtrip() {
        let msg = ParentMessage::ConflictResolution {
            action: "rename".into(), new_name: Some("new.txt".into()), apply_to_all: true,
        };
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: ParentMessage = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, ParentMessage::ConflictResolution { action, new_name: Some(n), apply_to_all: true }
            if action == "rename" && n == "new.txt"));
    }

    #[test]
    fn test_parent_message_cancel_roundtrip() {
        let msg = ParentMessage::Cancel;
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: ParentMessage = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, ParentMessage::Cancel));
    }

    #[test]
    fn test_gui_command_open_roundtrip() {
        let msg = GuiCommand::Open { path: "/a/b.7z".into(), password: Some("secret".into()) };
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: GuiCommand = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, GuiCommand::Open { path, password: Some(pw) }
            if path == "/a/b.7z" && pw == "secret"));
    }

    #[test]
    fn test_gui_command_activate_roundtrip() {
        let msg = GuiCommand::Activate;
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: GuiCommand = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, GuiCommand::Activate));
    }

    #[test]
    fn test_worker_message_serialization_format() {
        let msg = WorkerMessage::Progress {
            current: 1, total: 10, file: "".into(), bytes: 0,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""type":"progress""#));
        assert!(json.contains(r#""current":1"#));
        assert!(json.contains(r#""total":10"#));
    }

    #[test]
    fn test_deserialize_known_worker_json() {
        let json = r#"{"type":"error","code":-1,"message":"test error"}"#;
        let msg: WorkerMessage = serde_json::from_str(json).unwrap();
        assert!(matches!(msg, WorkerMessage::Error { code: -1, message } if message == "test error"));
    }
}
