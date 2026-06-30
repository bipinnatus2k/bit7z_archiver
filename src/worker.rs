//! Worker process entry point (no GPUI, headless archive operations).
//! Communicates with parent via JSON-line protocol on stdin/stdout.

use crate::ipc::WorkerMessage;
use crate::domain::repository::*;
use std::io::{BufRead, BufReader, Write};
use std::sync::Arc;

pub fn run_worker(repo: Arc<dyn ArchiveRepository>) {
    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();
    let mut stdin = BufReader::new(std::io::stdin());

    // Read one WorkerArgs JSON line from stdin
    let mut line = String::new();
    if stdin.read_line(&mut line).is_err() || line.trim().is_empty() {
        let _ = writeln!(stdout, r#"{{"type":"error","code":-1,"message":"no input"}}"#);
        return;
    }
    let args: WorkerArgs = match serde_json::from_str(line.trim()) {
        Ok(a) => a,
        Err(e) => {
            let _ = writeln!(stdout, r#"{{"type":"error","code":-1,"message":"invalid args: {}"}}"#, e);
            return;
        }
    };

    match args.operation {
        WorkerOperation::Extract { archive_path, dest, password, indices } => {
            // Open archive → extract → report progress → report complete/error
            let pw = password.map(crate::domain::archive::Password::new);
            match repo.open(&archive_path, pw.as_ref()) {
                Ok(archive) => {
                    send_msg(&mut stdout, &WorkerMessage::Progress {
                        current: 0, total: indices.len() as u64,
                        file: String::new(), bytes: 0,
                    });
                    let options = crate::domain::repository::ExtractOptions {
                        overwrite_mode: crate::domain::archive::OverwriteMode::Overwrite,
                        cancel: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
                        paused: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
                        notifier: std::sync::Arc::new(crate::application::progress::NoopNotifier),
                    };
                    let result = repo.extract(&archive, &indices, &dest, &options);
                    match result {
                        Ok(()) => send_msg(&mut stdout, &WorkerMessage::Complete {
                            total_files: indices.len() as u64,
                            total_bytes: 0, duration_ms: 0,
                        }),
                        Err(e) => send_msg(&mut stdout, &WorkerMessage::Error {
                            code: 1, message: e.to_string(),
                        }),
                    }
                    repo.close(&archive);
                }
                Err(e) => send_msg(&mut stdout, &WorkerMessage::Error {
                    code: 1, message: e.to_string(),
                }),
            }
        }
    }
}

fn send_msg(w: &mut impl Write, msg: &WorkerMessage) {
    let _ = writeln!(w, "{}", serde_json::to_string(msg).unwrap());
}

#[derive(Debug, serde::Deserialize)]
pub struct WorkerArgs {
    pub operation: WorkerOperation,
}

#[derive(Debug, serde::Deserialize)]
pub enum WorkerOperation {
    Extract {
        archive_path: std::path::PathBuf,
        dest: std::path::PathBuf,
        password: Option<String>,
        indices: Vec<u32>,
    },
}
