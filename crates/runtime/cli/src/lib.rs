use bit7z_app_archive::add_to::AddToArchiveUseCase;
use bit7z_app_archive::create::{CreateArchiveInput, CreateArchiveUseCase};
use bit7z_app_archive::runtime_service::ArchiveService;
use bit7z_app_checksum::{CalculateChecksumUseCase, ChecksumAlgorithm};
use bit7z_app_preview::PreviewEntryUseCase;
use bit7z_domain::archive::{
    ArchiveFormat, ArchiveHandle, EncryptionConfig, EncryptionMethod, OverwriteMode, Password,
};
use bit7z_domain::repository::{ArchiveError, ArchiveProperties, ExtractOptions, NoopNotifier};
use bit7z_infra_shell::ShellIntegration;
use clap::{Parser, Subcommand};
use humansize::{BINARY, format_size};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Parser)]
#[command(
    name = "bit7z_archiver",
    version,
    about = "Cross-platform compressed file viewer and editor"
)]
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
        indices: Option<String>,
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
    /// List contents of an archive
    List {
        path: PathBuf,
        #[arg(long)]
        password: Option<String>,
    },
    /// Compute checksum of an entry
    Checksum {
        path: PathBuf,
        index: usize,
        #[arg(long)]
        algorithm: Option<String>,
        #[arg(long)]
        password: Option<String>,
    },
    /// Create a new folder in an archive
    NewFolder {
        path: PathBuf,
        folder_path: String,
        #[arg(long)]
        password: Option<String>,
    },
    /// Register shell context menu entries
    ShellInstall,
    /// Unregister shell context menu entries
    ShellUninstall,
}

/// Helper to open an archive, run a closure, and ensure the handle is closed.
fn with_archive<F, R>(
    service: &Arc<ArchiveService>,
    path: &Path,
    password: Option<&Password>,
    f: F,
) -> Result<R, ArchiveError>
where
    F: FnOnce(&ArchiveHandle) -> Result<R, ArchiveError>,
{
    let handle = service.open(path, password)?;
    let result = f(&handle);
    service.close(&handle);
    result
}

pub fn run_cli(service: Arc<ArchiveService>, cli: &Cli) {
    let Some(ref cmd) = cli.command else {
        eprintln!("No command specified. Use --help for usage.");
        return;
    };
    match cmd {
        Commands::Open { path, password } => {
            let pw = password.as_ref().map(|p| Password::new(p.clone()));
            if let Err(e) = with_archive(&service, Path::new(path), pw.as_ref(), |handle| {
                let props = service.get_properties(handle).unwrap_or_else(|e| {
                    eprintln!("Warning: could not read properties: {}", e);
                    ArchiveProperties::default()
                });
                println!(
                    "Opened: {} ({} items, {} files, {} folders)",
                    path, props.items_count, props.files_count, props.folders_count
                );
                Ok(())
            }) {
                eprintln!("Error: {}", e);
            }
        }
        Commands::Extract {
            path,
            to,
            indices,
            password,
        } => {
            let pw = password.as_ref().map(|p| Password::new(p.clone()));
            let dest = to.as_deref().unwrap_or(".");
            if let Err(e) = with_archive(&service, Path::new(path), pw.as_ref(), |handle| {
                let props = service.get_properties(handle).ok();
                let count = props.map(|p| p.items_count).unwrap_or(0);
                let indices: Vec<u32> = if let Some(s) = indices {
                    s.split(',')
                        .filter_map(|part| part.trim().parse::<u32>().ok())
                        .filter(|&i| i < count)
                        .collect()
                } else if count > 0 {
                    (0..count).collect()
                } else {
                    vec![]
                };
                if indices.is_empty() {
                    eprintln!("No valid indices specified");
                    return Ok(());
                }
                let options = ExtractOptions {
                    overwrite_mode: OverwriteMode::Ask,
                    cancel: Arc::new(std::sync::atomic::AtomicBool::new(false)),
                    paused: Arc::new(std::sync::atomic::AtomicBool::new(false)),
                    notifier: Arc::new(NoopNotifier),
                };
                service.extract(handle, &indices, Path::new(dest), &options)?;
                println!("Extracted {} entries to {}", indices.len(), dest);
                Ok(())
            }) {
                eprintln!("Error: {}", e);
            }
        }
        Commands::Test { path, password } => {
            let pw = password.as_ref().map(|p| Password::new(p.clone()));
            if let Err(e) = with_archive(&service, Path::new(path), pw.as_ref(), |handle| {
                match service.test(handle) {
                    Ok(result) => {
                        println!("Test result: {}/{} passed", result.passed, result.total)
                    }
                    Err(e) => eprintln!("Test error: {}", e),
                }
                Ok(())
            }) {
                eprintln!("Error: {}", e);
            }
        }
        Commands::Compress {
            files,
            to,
            format,
            password,
        } => {
            let dest = to.as_deref().unwrap_or("archive.7z");
            let archive_format = parse_format(format);
            let archive_path = PathBuf::from(dest);

            // Enumerate all input files (walk directories recursively)
            let file_paths = collect_files(files);

            // Create archive
            let create_uc = CreateArchiveUseCase::new(service.clone());
            let encryption = password.as_ref().map(|pw| {
                let method = match archive_format {
                    ArchiveFormat::Zip => EncryptionMethod::ZipCrypto,
                    _ => EncryptionMethod::Aes256,
                };
                EncryptionConfig {
                    password: Password::new(pw.clone()),
                    method,
                    encrypt_filenames: false,
                }
            });

            let input = CreateArchiveInput {
                destination: archive_path.clone(),
                format: archive_format,
                compression_level: 5,
                encryption,
            };
            let mut handle = match create_uc.execute(&input) {
                Ok(h) => h,
                Err(e) => {
                    eprintln!("Error creating archive: {}", e);
                    return;
                }
            };

            // Add files
            let add_uc = AddToArchiveUseCase::new(service.clone());
            match add_uc.execute(&mut handle, &file_paths, None) {
                Ok(()) => {}
                Err(e) => {
                    eprintln!("Error adding files: {}", e);
                    service.close(&handle);
                    return;
                }
            }

            // Compute sizes for summary
            let uncompressed: u64 = file_paths
                .iter()
                .filter_map(|p| std::fs::metadata(p).ok())
                .map(|m| m.len())
                .sum();

            // Get compressed size
            let compressed = std::fs::metadata(&archive_path)
                .map(|m| m.len())
                .unwrap_or(0);

            let ratio = if uncompressed > 0 {
                ((uncompressed - compressed) as f64 / uncompressed as f64) * 100.0
            } else {
                0.0
            };

            println!(
                "Created {} — {} files, {} -> {} ({:.1}%)",
                dest,
                file_paths.len(),
                format_size(uncompressed, BINARY),
                format_size(compressed, BINARY),
                ratio
            );

            service.close(&handle);
        }
        Commands::Preview {
            path,
            index,
            password,
            max_bytes,
        } => {
            let pw = password.as_ref().map(|p| Password::new(p.clone()));
            let uc = PreviewEntryUseCase::new(service.clone());
            match service.open(Path::new(path), pw.as_ref()) {
                Ok(handle) => {
                    match uc.execute(&handle, *index, *max_bytes) {
                        Ok(data) => match data {
                            bit7z_app_preview::PreviewData::Text(t) => println!("{}", t),
                            // bit7z_app_preview::PreviewData::Hex(h) => {
                            //     for (ci, chunk) in h.chunks(16).enumerate() {
                            //         let offset = ci * 16;
                            //         let hex: String =
                            //             chunk.iter().map(|b| format!("{:02x} ", b)).collect();
                            //         let ascii: String = chunk
                            //             .iter()
                            //             .map(|&b| {
                            //                 if b.is_ascii_graphic() || b == b' ' {
                            //                     b as char
                            //                 } else {
                            //                     '.'
                            //                 }
                            //             })
                            //             .collect();
                            //         println!("{:08x}  {:<48}  {}", offset, hex, ascii);
                            //     }
                            // }
                            bit7z_app_preview::PreviewData::Image(img) => {
                                println!("Image preview: {} bytes", img.len());
                            }
                            bit7z_app_preview::PreviewData::Unsupported(msg) => {
                                eprintln!("Unsupported: {}", msg);
                            }
                        },
                        Err(e) => eprintln!("Preview error: {}", e),
                    }
                    service.close(&handle);
                }
                Err(e) => eprintln!("Error: {}", e),
            }
        }
        Commands::Add {
            path,
            files,
            password,
        } => {
            let pw = password.as_ref().map(|p| Password::new(p.clone()));
            let mut handle = match service.open(Path::new(path), pw.as_ref()) {
                Ok(h) => h,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    return;
                }
            };
            let paths: Vec<PathBuf> =
                files.iter().map(PathBuf::from).collect();
            let uc = AddToArchiveUseCase::new(service.clone());
            match uc.execute(&mut handle, &paths, None) {
                Ok(()) => println!("Added {} files to {}", files.len(), path),
                Err(e) => eprintln!("Add error: {}", e),
            }
            service.close(&handle);
        }
        Commands::Delete {
            path,
            indices,
            password,
        } => {
            let pw = password.as_ref().map(|p| Password::new(p.clone()));
            let mut handle = match service.open(Path::new(path), pw.as_ref()) {
                Ok(h) => h,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    return;
                }
            };
            let uc = bit7z_app_archive::delete::DeleteEntriesUseCase::new(service.clone());
            match uc.execute(&mut handle, indices, None) {
                Ok(()) => println!("Deleted {} entries from {}", indices.len(), path),
                Err(e) => eprintln!("Delete error: {}", e),
            }
            service.close(&handle);
        }
        Commands::Rename {
            path,
            index,
            name,
            password,
        } => {
            let pw = password.as_ref().map(|p| Password::new(p.clone()));
            let mut handle = match service.open(Path::new(path), pw.as_ref()) {
                Ok(h) => h,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    return;
                }
            };
            let uc = bit7z_app_archive::rename::RenameEntryUseCase::new(service.clone());
            match uc.execute(&mut handle, *index, name) {
                Ok(()) => println!("Renamed entry {} to '{}'", index, name),
                Err(e) => eprintln!("Rename error: {}", e),
            }
            service.close(&handle);
        }
        Commands::List { path, password } => {
            let pw = password.as_ref().map(|p| Password::new(p.clone()));
            let handle = match service.open(path.as_path(), pw.as_ref()) {
                Ok(h) => h,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    return;
                }
            };
            match service.list_page(&handle, 0, u32::MAX as usize) {
                Ok(page) => {
                    println!(
                        "{0: <6} {1: <40} {2: >10} {3: >10} {4: >7} {5: <10}",
                        "Index", "Name", "Size", "Packed", "Ratio", "Modified"
                    );
                    println!(
                        "{:-<6} {:-<40} {:-<10} {:-<10} {:-<7} {:-<10}",
                        "", "", "", "", "", ""
                    );
                    for entry in &page.items {
                        let size_str = if entry.is_directory {
                            "<DIR>".to_string()
                        } else {
                            format_size(entry.size, BINARY)
                        };
                        let packed_str = if entry.is_directory {
                            "-".to_string()
                        } else {
                            format_size(entry.compressed_size, BINARY)
                        };
                        let ratio_str = if entry.is_directory {
                            "-".to_string()
                        } else {
                            format!("{:.0}%", entry.compression_ratio() * 100.0)
                        };
                        let date_str = entry
                            .modified
                            .map(|d| d.format("%Y-%m-%d").to_string())
                            .unwrap_or_else(|| "-".to_string());
                        let display_name = if entry.is_directory {
                            format!("{}/", entry.name)
                        } else {
                            entry.name.clone()
                        };
                        println!(
                            "{0: <6} {1: <40} {2: >10} {3: >10} {4: >7} {5: <10}",
                            entry.original_index,
                            display_name,
                            size_str,
                            packed_str,
                            ratio_str,
                            date_str
                        );
                    }
                }
                Err(e) => eprintln!("Error listing archive: {}", e),
            }
            service.close(&handle);
        }
        Commands::Checksum {
            path,
            index,
            algorithm,
            password,
        } => {
            let pw = password.as_ref().map(|p| Password::new(p.clone()));
            let handle = match service.open(path.as_path(), pw.as_ref()) {
                Ok(h) => h,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    return;
                }
            };
            let algorithms = parse_checksum_algorithms(algorithm.as_deref());
            let uc = CalculateChecksumUseCase::new(service.clone());
            match uc.execute(&handle, &[*index as u32], &algorithms) {
                Ok(results) => {
                    for result in &results {
                        println!("Checksums for {} (index {}):", result.path, index);
                        if let Some(ref crc) = result.crc32 {
                            println!("  CRC32:  {}", crc);
                        }
                        if let Some(ref md5) = result.md5 {
                            println!("  MD5:    {}", md5);
                        }
                        if let Some(ref sha1) = result.sha1 {
                            println!("  SHA1:   {}", sha1);
                        }
                        if let Some(ref sha256) = result.sha256 {
                            println!("  SHA256: {}", sha256);
                        }
                    }
                }
                Err(e) => eprintln!("Checksum error: {}", e),
            }
            service.close(&handle);
        }
        Commands::NewFolder {
            path,
            folder_path,
            password,
        } => {
            let pw = password.as_ref().map(|p| Password::new(p.clone()));
            let mut handle = match service.open(path.as_path(), pw.as_ref()) {
                Ok(h) => h,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    return;
                }
            };
            let service_clone = service.clone();
            match bit7z_app_archive::new_folder::new_folder(
                service_clone,
                &mut handle,
                &folder_path,
                pw.as_ref(),
            ) {
                Ok(()) => println!("Created folder '{}' in {}", folder_path, path.display()),
                Err(e) => eprintln!("Error: {}", e),
            }
            service.close(&handle);
        }
        Commands::ShellInstall => {
            #[cfg(target_os = "windows")]
            {
                bit7z_infra_shell::windows::WindowsShellIntegration::register()
                    .expect("Failed to register shell integration");
                println!("Shell integration registered.");
            }
            #[cfg(target_os = "linux")]
            {
                bit7z_infra_shell::linux::LinuxShellIntegration::register()
                    .expect("Failed to register shell integration");
                println!("Shell integration registered.");
            }
            #[cfg(not(any(target_os = "windows", target_os = "linux")))]
            eprintln!("Shell integration not supported on this platform.");
        }
        Commands::ShellUninstall => {
            #[cfg(target_os = "windows")]
            {
                bit7z_infra_shell::windows::WindowsShellIntegration::unregister()
                    .expect("Failed to unregister shell integration");
                println!("Shell integration unregistered.");
            }
            #[cfg(target_os = "linux")]
            {
                bit7z_infra_shell::linux::LinuxShellIntegration::unregister()
                    .expect("Failed to unregister shell integration");
                println!("Shell integration unregistered.");
            }
            #[cfg(not(any(target_os = "windows", target_os = "linux")))]
            eprintln!("Shell integration not supported on this platform.");
        }
    }
}

fn parse_format(s: &str) -> ArchiveFormat {
    match s.to_lowercase().as_str() {
        "7z" | "sevenzip" => ArchiveFormat::SevenZip,
        "zip" => ArchiveFormat::Zip,
        "tar" => ArchiveFormat::Tar,
        "tar.gz" | "tgz" | "targz" => ArchiveFormat::TarGz,
        "tar.xz" | "txz" | "tarxz" => ArchiveFormat::TarXz,
        "tar.bz2" | "tbz2" | "tarbz2" => ArchiveFormat::TarBz2,
        "gz" | "gzip" => ArchiveFormat::GZip,
        "bz2" | "bzip2" => ArchiveFormat::BZip2,
        "xz" => ArchiveFormat::Xz,
        "wim" => ArchiveFormat::Wim,
        "rar" => ArchiveFormat::Rar,
        _ => ArchiveFormat::SevenZip,
    }
}

fn collect_files(paths: &[String]) -> Vec<PathBuf> {
    let mut result = Vec::new();
    for path_str in paths {
        let path = PathBuf::from(path_str);
        if path.is_symlink() {
            eprintln!(
                "Warning: skipping symlink '{}' (may cause infinite loops)",
                path_str
            );
            continue;
        }
        if path.is_file() {
            result.push(path);
        } else if path.is_dir() {
            collect_dir(&path, &mut result);
        }
    }
    result
}

fn collect_dir(dir: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_symlink() {
                continue;
            }
            if path.is_file() {
                out.push(path);
            } else if path.is_dir() {
                collect_dir(&path, out);
            }
        }
    }
}

fn parse_checksum_algorithms(alg: Option<&str>) -> Vec<ChecksumAlgorithm> {
    match alg.map(|s| s.to_lowercase()).as_deref() {
        Some("crc32") => vec![ChecksumAlgorithm::Crc32],
        Some("md5") => vec![ChecksumAlgorithm::Md5],
        Some("sha1") => vec![ChecksumAlgorithm::Sha1],
        Some("sha256") | None => vec![ChecksumAlgorithm::Sha256],
        Some(other) => {
            eprintln!("Unknown algorithm '{}', defaulting to sha256", other);
            vec![ChecksumAlgorithm::Sha256]
        }
    }
}
