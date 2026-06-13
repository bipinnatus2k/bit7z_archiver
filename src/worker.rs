//! Worker process entry point (no GPUI, headless archive operations).
//! Communicates with parent via JSON-line protocol on stdin/stdout.

use crate::ipc::WorkerMessage;
use crate::domain::repository::*;
use std::io::{BufReader, Write};
use std::sync::Arc;

pub fn run_worker(repo: Arc<dyn ArchiveRepository>, args: WorkerArgs) {
    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();
    let _stdin = BufReader::new(std::io::stdin());

    match args.operation {
        WorkerOperation::Extract { archive_path, dest, password, indices } => {
            // Open archive → extract → report progress → report complete/error
            match repo.open(&archive_path, password.as_deref()) {
                Ok(archive) => {
                    send_msg(&mut stdout, &WorkerMessage::Progress {
                        current: 0, total: indices.len() as u64,
                        file: String::new(), bytes: 0,
                    });
                    let result = repo.extract(&archive, &indices, &dest);
                    match result {
                        Ok(()) => send_msg(&mut stdout, &WorkerMessage::Complete {
                            total_files: indices.len() as u64,
                            total_bytes: 0, duration_ms: 0,
                        }),
                        Err(e) => send_msg(&mut stdout, &WorkerMessage::Error {
                            code: 1, message: e.to_string(),
                        }),
                    }
                    repo.close(archive);
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

#[derive(Debug)]
pub struct WorkerArgs {
    pub operation: WorkerOperation,
}

#[derive(Debug)]
pub enum WorkerOperation {
    Extract {
        archive_path: std::path::PathBuf,
        dest: std::path::PathBuf,
        password: Option<String>,
        indices: Vec<u32>,
    },
}
