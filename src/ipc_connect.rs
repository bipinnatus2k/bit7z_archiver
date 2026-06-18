//! IPC transport for CLI→GUI communication.
//! Uses a simple TCP socket on localhost with a port derived from the archive path hash.
//! This avoids platform-specific dependencies and works cross-platform.

use crate::ipc::GuiCommand;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;

/// Derive a port number from an archive path (base 34000 + hash % 10000).
fn path_port(path: &Path) -> u16 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    let port_base: u16 = 34000;
    let offset = (hasher.finish() % 10000) as u16;
    port_base + offset
}

/// Try to send a GuiCommand to an existing GUI process for the given archive path.
/// Returns `true` if the command was delivered successfully.
pub fn try_send(path: &Path, cmd: &GuiCommand) -> bool {
    let port = path_port(path);
    let addr = format!("127.0.0.1:{}", port);

    match TcpStream::connect_timeout(
        &addr.parse().unwrap(),
        std::time::Duration::from_millis(500),
    ) {
        Ok(mut stream) => {
            let json = serde_json::to_string(cmd).unwrap_or_default();
            let mut line = json;
            line.push('\n');
            stream.write_all(line.as_bytes()).is_ok()
        }
        Err(_) => false,
    }
}

/// Start an IPC listener thread that accepts GuiCommands.
/// `on_command` is called from the listener thread for each received command.
pub fn start_listener<F>(path: &Path, on_command: F)
where
    F: Fn(GuiCommand) + Send + 'static,
{
    let port = path_port(path);
    let addr = format!("127.0.0.1:{}", port);

    std::thread::Builder::new()
        .name("ipc-listener".into())
        .spawn(move || loop {
            match TcpListener::bind(&addr) {
                Ok(listener) => {
                    for stream in listener.incoming() {
                        match stream {
                            Ok(stream) => {
                                let reader = BufReader::new(stream);
                                for line in reader.lines() {
                                    if let Ok(json) = line {
                                        if let Ok(cmd) =
                                            serde_json::from_str::<GuiCommand>(&json)
                                        {
                                            on_command(cmd);
                                        }
                                    }
                                }
                            }
                            Err(_) => break,
                        }
                    }
                }
                Err(_) => {
                    // Port might be in use (another instance), retry later
                    std::thread::sleep(std::time::Duration::from_secs(1));
                }
            }
        })
        .ok();
}
