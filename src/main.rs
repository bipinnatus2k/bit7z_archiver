#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

mod theme;
mod domain;
mod ffi;
mod gui;
mod application;
mod adapters;
mod cli;
mod error;
mod ipc;
mod ipc_connect;
mod instance;
mod worker;

use std::path::Path;
use std::sync::Arc;
use clap::Parser;
use cli::Cli;

fn main() {
    println!("bit7z Archiver v{}", env!("CARGO_PKG_VERSION"));

    let cli = Cli::parse();

    let lib_path = adapters::platform::find_7z_library()
        .expect("7-Zip library not found. Install 7-Zip or p7zip.");
    let lib_path_str = lib_path.to_string_lossy();
    let lib = adapters::bit7z::Library::open(&lib_path_str)
        .expect("Failed to load 7-Zip library");
    let repo: Arc<dyn domain::repository::ArchiveRepository> =
        Arc::new(adapters::repository::Bit7zRepository::new(lib));

    if cli.worker {
        worker::run_worker(repo);
        return;
    }

    // Try CLI→GUI handoff: if command has an archive path, try forwarding to existing GUI
    if let Some(ref cmd) = cli.command {
        if let Some(archive_path) = get_command_path(cmd) {
            let path = Path::new(&archive_path);
            // Try to send command to existing GUI instance via IPC
            let ipc_cmd = command_to_gui_cmd(cmd);
            if ipc_connect::try_send(path, &ipc_cmd) {
                return; // Successfully forwarded to existing GUI
            }
            // No existing GUI instance — launch GUI with this archive
            let pw = get_command_password(cmd);
            gui::run_gui_with_path(Some(archive_path.into()), pw);
            return;
        }
    }

    // Fall through to CLI mode
    if let Some(_) = &cli.command {
        cli::run_cli(repo, &cli);
    } else {
        gui::run_gui();
    }
}

/// Extract the archive path from a CLI command, if applicable.
fn get_command_path(cmd: &cli::Commands) -> Option<String> {
    use cli::Commands::*;
    match cmd {
        Open { path, .. } | Extract { path, .. } | Test { path, .. }
            | Preview { path, .. } | Add { path, .. } | Delete { path, .. }
            | Rename { path, .. } => Some(path.clone()),
        Compress { to, .. } => to.clone(),
        ShellInstall | ShellUninstall => None,
    }
}

/// Extract password from a CLI command, if applicable.
fn get_command_password(cmd: &cli::Commands) -> Option<String> {
    use cli::Commands::*;
    match cmd {
        Open { password, .. } | Extract { password, .. } | Test { password, .. }
            | Preview { password, .. } | Add { password, .. } | Delete { password, .. }
            | Rename { password, .. } => password.clone(),
        Compress { password, .. } => password.clone(),
        ShellInstall | ShellUninstall => None,
    }
}

/// Convert a CLI command to the equivalent GuiCommand for IPC forwarding.
fn command_to_gui_cmd(cmd: &cli::Commands) -> ipc::GuiCommand {
    use cli::Commands::*;
    match cmd {
        Open { path, password } => {
            ipc::GuiCommand::Open { path: path.clone(), password: password.clone() }
        }
        _ => ipc::GuiCommand::Activate,
    }
}


