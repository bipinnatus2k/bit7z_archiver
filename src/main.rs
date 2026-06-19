use std::path::Path;
use std::sync::Arc;
use clap::Parser;
use bit7z_archiver::cli::{self, Cli};

fn main() {
    println!("bit7z Archiver v{}", env!("CARGO_PKG_VERSION"));

    let cli = Cli::parse();

    let lib_path = bit7z_archiver::adapters::platform::find_7z_library()
        .expect("7-Zip library not found. Install 7-Zip or p7zip.");
    let lib_path_str = lib_path.to_string_lossy();
    let lib = bit7z_archiver::adapters::bit7z::Library::open(&lib_path_str)
        .expect("Failed to load 7-Zip library");
    let repo: Arc<dyn bit7z_archiver::domain::repository::ArchiveRepository> =
        Arc::new(bit7z_archiver::adapters::repository::Bit7zRepository::new(lib));

    if cli.worker {
        bit7z_archiver::worker::run_worker(repo);
        return;
    }

    if let Some(ref cmd) = cli.command {
        if let Some(archive_path) = get_command_path(cmd) {
            let path = Path::new(&archive_path);
            let ipc_cmd = command_to_gui_cmd(cmd);
            if bit7z_archiver::ipc_connect::try_send(path, &ipc_cmd) {
                return;
            }
            let pw = get_command_password(cmd);
            bit7z_archiver::gui::run_gui_with_path(Some(archive_path.into()), pw);
            return;
        }
    }

    if let Some(_) = &cli.command {
        cli::run_cli(repo, &cli);
    } else {
        bit7z_archiver::gui::run_gui();
    }
}

fn get_command_path(cmd: &cli::Commands) -> Option<String> {
    use cli::Commands::*;
    match cmd {
        Open { path, .. } | Extract { path, .. } | Test { path, .. }
            | Preview { path, .. } | Add { path, .. } | Delete { path, .. }
            | Rename { path, .. } => Some(path.clone()),
        List { path, .. } | Checksum { path, .. } | NewFolder { path, .. } => {
            Some(path.to_string_lossy().into_owned())
        }
        Compress { to, .. } => to.clone(),
        ShellInstall | ShellUninstall => None,
    }
}

fn get_command_password(cmd: &cli::Commands) -> Option<String> {
    use cli::Commands::*;
    match cmd {
        Open { password, .. } | Extract { password, .. } | Test { password, .. }
            | Preview { password, .. } | Add { password, .. } | Delete { password, .. }
            | Rename { password, .. } | List { password, .. }
            | Checksum { password, .. } | NewFolder { password, .. } => password.clone(),
        Compress { password, .. } => password.clone(),
        ShellInstall | ShellUninstall => None,
    }
}

fn command_to_gui_cmd(cmd: &cli::Commands) -> bit7z_archiver::ipc::GuiCommand {
    use cli::Commands::*;
    match cmd {
        Open { path, password } => {
            bit7z_archiver::ipc::GuiCommand::Open { path: path.clone(), password: password.clone() }
        }
        _ => bit7z_archiver::ipc::GuiCommand::Activate,
    }
}
