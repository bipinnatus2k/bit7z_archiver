use crate::application::preview::PreviewEntryUseCase;
use crate::domain::archive::Password;
use crate::domain::repository::ArchiveRepository;
use crate::adapters::shell::ShellIntegration;
use clap::{Parser, Subcommand};
use std::path::Path;
use std::sync::Arc;

#[derive(Parser)]
#[command(name = "bit7z_archiver", version, about = "Cross-platform compressed file viewer and editor")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Run as a background worker process (spawned by parent for IPC operations)
    #[arg(long, global = true, hide = true)]
    pub worker: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    Open {
        path: String,
        #[arg(long)]
        password: Option<String>,
    },
    Extract {
        path: String,
        #[arg(long)]
        to: Option<String>,
        #[arg(long)]
        password: Option<String>,
    },
    Test {
        path: String,
        #[arg(long)]
        password: Option<String>,
    },
    Compress {
        #[arg(num_args = 1..)]
        files: Vec<String>,
        #[arg(long)]
        to: Option<String>,
        #[arg(long, default_value = "7z")]
        format: String,
        #[arg(long)]
        password: Option<String>,
    },
    Preview {
        path: String,
        index: u32,
        #[arg(long)]
        password: Option<String>,
        #[arg(long, default_value = "4096")]
        max_bytes: usize,
    },
    Add {
        path: String,
        #[arg(num_args = 1..)]
        files: Vec<String>,
        #[arg(long)]
        password: Option<String>,
    },
    Delete {
        path: String,
        #[arg(num_args = 1..)]
        indices: Vec<u32>,
        #[arg(long)]
        password: Option<String>,
    },
    Rename {
        path: String,
        index: u32,
        name: String,
        #[arg(long)]
        password: Option<String>,
    },
    /// Register shell context menu entries
    ShellInstall,
    /// Unregister shell context menu entries
    ShellUninstall,
}

pub fn run_cli(repo: Arc<dyn ArchiveRepository>, cli: &Cli) {
    let Some(ref cmd) = cli.command else {
        eprintln!("No command specified. Use --help for usage.");
        return;
    };
    match cmd {
        Commands::Open { path, password } => {
            let pw = password.as_ref().map(|p| Password::new(p.clone()));
            match repo.open(Path::new(path), pw.as_ref()) {
                Ok(handle) => {
                    let props = repo.get_properties(&handle).unwrap_or_else(|e| {
                        eprintln!("Warning: could not read properties: {}", e);
                        crate::domain::repository::ArchiveProperties {
                            items_count: 0, folders_count: 0, files_count: 0,
                            total_size: 0, packed_size: 0, is_encrypted: false,
                            has_encrypted_items: false, is_multi_volume: false, is_solid: false,
                        }
                    });
                    println!("Opened: {} ({} items, {} files, {} folders)",
                        path, props.items_count, props.files_count, props.folders_count);
                    repo.close(handle);
                }
                Err(e) => eprintln!("Error: {}", e),
            }
        }
        Commands::Extract { path, to, password } => {
            let pw = password.as_ref().map(|p| Password::new(p.clone()));
            let dest = to.as_deref().unwrap_or(".");
            match repo.open(Path::new(path), pw.as_ref()) {
                Ok(handle) => {
                    let props = repo.get_properties(&handle).ok();
                    let count = props.map(|p| p.items_count).unwrap_or(0);
                    let indices: Vec<u32> = if count > 0 { (0..count).collect() } else { vec![] };
                    match repo.extract(&handle, &indices, Path::new(dest)) {
                        Ok(()) => println!("Extracted {} entries to {}", indices.len(), dest),
                        Err(e) => eprintln!("Extract error: {}", e),
                    }
                    repo.close(handle);
                }
                Err(e) => eprintln!("Error: {}", e),
            }
        }
        Commands::Test { path, password } => {
            let pw = password.as_ref().map(|p| Password::new(p.clone()));
            match repo.open(Path::new(path), pw.as_ref()) {
                Ok(handle) => {
                    match repo.test(&handle) {
                        Ok(result) => println!("Test result: {}/{} passed", result.passed, result.total),
                        Err(e) => eprintln!("Test error: {}", e),
                    }
                    repo.close(handle);
                }
                Err(e) => eprintln!("Error: {}", e),
            }
        }
        Commands::Compress { files, to, format: _, password: _ } => {
            let dest = to.as_deref().unwrap_or("archive.7z");
            eprintln!("Compress: {} files -> {} (format control pending)", files.len(), dest);
            eprintln!("Note: compression is a placeholder — files must be added after creation.");
        }
        Commands::Preview { path, index, password, max_bytes } => {
            let pw = password.as_ref().map(|p| Password::new(p.clone()));
            let uc = PreviewEntryUseCase::new(repo.clone());
            match repo.open(Path::new(path), pw.as_ref()) {
                Ok(handle) => {
                    match uc.execute(&handle, *index, *max_bytes) {
                        Ok(data) => match data {
                            crate::application::preview::PreviewData::Text(t) => println!("{}", t),
                            crate::application::preview::PreviewData::Hex(h) => {
                                for chunk in h.chunks(16) {
                                    let hex: String = chunk.iter().map(|b| format!("{:02x} ", b)).collect();
                                    let ascii: String = chunk.iter()
                                        .map(|&b| if b.is_ascii_graphic() || b == b' ' { b as char } else { '.' })
                                        .collect();
                                    println!("{:08x}  {:<48}  {}", 0, hex, ascii);
                                }
                            }
                            crate::application::preview::PreviewData::Image(img) => {
                                println!("Image preview: {} bytes", img.len());
                            }
                            crate::application::preview::PreviewData::Unsupported(msg) => {
                                eprintln!("Unsupported: {}", msg);
                            }
                        }
                        Err(e) => eprintln!("Preview error: {}", e),
                    }
                    repo.close(handle);
                }
                Err(e) => eprintln!("Error: {}", e),
            }
        }
        Commands::Add { path, files, password } => {
            let pw = password.as_ref().map(|p| Password::new(p.clone()));
            let mut handle = match repo.open(Path::new(path), pw.as_ref()) {
                Ok(h) => h,
                Err(e) => { eprintln!("Error: {}", e); return; }
            };
            let paths: Vec<std::path::PathBuf> = files.iter().map(std::path::PathBuf::from).collect();
            match repo.add(&mut handle, &paths) {
                Ok(()) => println!("Added {} files to {}", files.len(), path),
                Err(e) => eprintln!("Add error: {}", e),
            }
            repo.close(handle);
        }
        Commands::Delete { path, indices, password } => {
            let pw = password.as_ref().map(|p| Password::new(p.clone()));
            let mut handle = match repo.open(Path::new(path), pw.as_ref()) {
                Ok(h) => h,
                Err(e) => { eprintln!("Error: {}", e); return; }
            };
            match repo.delete(&mut handle, indices) {
                Ok(()) => println!("Deleted {} entries from {}", indices.len(), path),
                Err(e) => eprintln!("Delete error: {}", e),
            }
            repo.close(handle);
        }
        Commands::Rename { path, index, name, password } => {
            let pw = password.as_ref().map(|p| Password::new(p.clone()));
            let mut handle = match repo.open(Path::new(path), pw.as_ref()) {
                Ok(h) => h,
                Err(e) => { eprintln!("Error: {}", e); return; }
            };
            match repo.rename(&mut handle, *index, name) {
                Ok(()) => println!("Renamed entry {} to '{}'", index, name),
                Err(e) => eprintln!("Rename error: {}", e),
            }
            repo.close(handle);
        }
        Commands::ShellInstall => {
            #[cfg(target_os = "windows")]
            {
                crate::adapters::shell::windows::WindowsShellIntegration::register()
                    .expect("Failed to register shell integration");
                println!("Shell integration registered.");
            }
            #[cfg(target_os = "linux")]
            {
                crate::adapters::shell::linux::LinuxShellIntegration::register()
                    .expect("Failed to register shell integration");
                println!("Shell integration registered.");
            }
            #[cfg(not(any(target_os = "windows", target_os = "linux")))]
            eprintln!("Shell integration not supported on this platform.");
        }
        Commands::ShellUninstall => {
            #[cfg(target_os = "windows")]
            {
                crate::adapters::shell::windows::WindowsShellIntegration::unregister()
                    .expect("Failed to unregister shell integration");
                println!("Shell integration unregistered.");
            }
            #[cfg(target_os = "linux")]
            {
                crate::adapters::shell::linux::LinuxShellIntegration::unregister()
                    .expect("Failed to unregister shell integration");
                println!("Shell integration unregistered.");
            }
            #[cfg(not(any(target_os = "windows", target_os = "linux")))]
            eprintln!("Shell integration not supported on this platform.");
        }
    }
}
