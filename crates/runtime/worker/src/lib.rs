//! Worker process entry point (no GPUI, headless archive operations).
//! Communicates with parent via JSON-line protocol on stdin/stdout.

use bit7z_rt_ipc::WorkerMessage;
use bit7z_domain::repository::*;
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
            let pw = password.map(bit7z_domain::archive::Password::new);
            match repo.open(&archive_path, pw.as_ref()) {
                Ok(archive) => {
                    send_msg(&mut stdout, &WorkerMessage::Progress {
                        current: 0, total: indices.len() as u64,
                        file: String::new(), bytes: 0,
                    });
                    let req = ExtractRequest {
                        indices: indices.clone(),
                        dest: dest.clone(),
                        overwrite: bit7z_domain::archive::OverwriteMode::Overwrite,
                    };
                    let ctx = OpCtx {
                        cancel: CancellationToken::new(),
                        pause: PauseToken::new(),
                        progress: Arc::new(NoopSink),
                    };
                    let result = repo.extract(&archive, &req, &ctx);
                    match result {
                        Ok(_report) => send_msg(&mut stdout, &WorkerMessage::Complete {
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
