use std::path::Path;
use std::sync::Arc;
use clap::Parser;
use bit7z_domain::archive::Password;
use bit7z_rt_cli::{self, Cli};

fn main() {
    println!("bit7z Archiver v{}", env!("CARGO_PKG_VERSION"));

    let cli = Cli::parse();

    let lib_path = bit7z_infra_platform::find_7z_library()
        .expect("7-Zip library not found. Install 7-Zip or p7zip.");
    let lib_path_str = lib_path.to_string_lossy();
    bit7z_infra_bit7z::Library::open(&lib_path_str)
        .expect("Failed to load 7-Zip library");
    let repo: Arc<dyn bit7z_domain::repository::ArchiveRepository> =
        Arc::new(bit7z_infra_repo::supervisor::RepoSupervisor::new(&lib_path_str).expect("Failed to create RepoSupervisor"));

    if cli.worker {
        bit7z_rt_worker::run_worker(repo);
        return;
    }

    if let Some(ref cmd) = cli.command {
        if let Some(archive_path) = get_command_path(cmd) {
            let path = Path::new(&archive_path);
            let ipc_cmd = command_to_gui_cmd(cmd);
            if bit7z_rt_ipc::try_send(path, &ipc_cmd) {
                return;
            }
            let pw = get_command_password(cmd);
            bit7z_rt_gui::run_gui_with_path(Some(archive_path.into()), pw,repo);
            return;
        }
    }

    if cli.command.is_some() {
        bit7z_rt_cli::run_cli(repo, &cli);
    } else {
        bit7z_rt_gui::run_gui(repo);
    }
}

fn get_command_path(cmd: &bit7z_rt_cli::Commands) -> Option<String> {
    use bit7z_rt_cli::Commands::*;
    match cmd {
        Open { path, .. } |
        Extract { path, .. } |
        Test { path, .. } |
        Preview { path, .. } |
        Add { path, .. } |
        Delete { path, .. } |
        Rename { path, .. } => Some(path.clone()),
        List { path, .. } |
        Checksum { path, .. } |
        NewFolder { path, .. } => {
            Some(path.to_string_lossy().into_owned())
        }
        Compress { to, .. } => to.clone(),
        ShellInstall | ShellUninstall => None,
    }
}

fn get_command_password(cmd: &bit7z_rt_cli::Commands) -> Option<Password> {
    use bit7z_rt_cli::Commands::*;
    match cmd {
        Open { password, .. } | Extract { password, .. } | Test { password, .. }
            | Preview { password, .. } | Add { password, .. } | Delete { password, .. }
            | Rename { password, .. } | List { password, .. }
            | Checksum { password, .. } | NewFolder { password, .. } => Some(Password::new(password.clone().unwrap())),
        Compress { password, .. } => Some(Password::new(password.clone().unwrap())),
        ShellInstall | ShellUninstall => None,
    }
}

fn command_to_gui_cmd(cmd: &bit7z_rt_cli::Commands) -> bit7z_rt_ipc::GuiCommand {
    use bit7z_rt_cli::Commands::*;
    match cmd {
        Open { path, password } => {
            bit7z_rt_ipc::GuiCommand::Open { path: path.clone(), password: password.clone() }
        }
        _ => bit7z_rt_ipc::GuiCommand::Activate,
    }
}
