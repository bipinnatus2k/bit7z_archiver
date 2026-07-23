use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use bit7z_domain::archive::{ArchiveSession, ArchiveFormat};
use bit7z_domain::vfs::{
    next_vfs_id, OverlayVfs, SessionState, Tree, VfsMetadata, VfsNode, VfsNodeId,
};
use chrono::{DateTime, NaiveDateTime, Utc};

use crate::harness::{TestHarness, TestIntegrity};

pub struct CliReferee {
    pub seven_zip_path: PathBuf,
}

impl CliReferee {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            seven_zip_path: path.into(),
        }
    }

    fn run_7z(&self, args: &[&str]) -> Result<String, String> {
        let output = Command::new(&self.seven_zip_path)
            .args(args)
            .output()
            .map_err(|e| format!("failed to run 7z: {e}"))?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        if !output.status.success() {
            return Err(format!(
                "7z failed ({}):\nstdout:\n{}\nstderr:\n{}",
                output.status, stdout, stderr
            ));
        }
        Ok(stdout)
    }
}

impl Default for CliReferee {
    fn default() -> Self {
        Self {
            #[cfg(windows)]
            seven_zip_path: PathBuf::from("7z.exe"),
            #[cfg(not(windows))]
            seven_zip_path: PathBuf::from("7z"),
        }
    }
}

#[derive(Default, Debug)]
struct ParsedEntry {
    path: String,
    size: u64,
    compressed_size: u64,
    modified: Option<DateTime<Utc>>,
    created: Option<DateTime<Utc>>,
    accessed: Option<DateTime<Utc>>,
    crc: Option<u32>,
    is_encrypted: bool,
    is_directory: bool,
    is_symlink: bool,
    attributes: Option<u32>,
    posix_attrib: Option<u32>,
    host_os: Option<u8>,
    compression_method: Option<String>,
}

impl ParsedEntry {
    fn apply(&mut self, key: &str, value: &str) {
        match key {
            "Path" => self.path = value.to_string(),
            "Size" => self.size = value.parse().unwrap_or(0),
            "Compressed" | "Packed Size" => self.compressed_size = value.parse().unwrap_or(0),
            "CRC" if value != "-" => {
                self.crc = u32::from_str_radix(value, 16).ok();
            }
            "Encrypted" => self.is_encrypted = value != "-",
            "Method" => self.compression_method = Some(value.to_string()),
            "Modified" => self.modified = parse_7z_time(value),
            "Created" => self.created = parse_7z_time(value),
            "Accessed" => self.accessed = parse_7z_time(value),
            "Attributes" => {
                if value.starts_with('D') {
                    self.is_directory = true;
                }
                if !(value.starts_with("A ") || value.starts_with("D ")) {
                    self.attributes = u32::from_str_radix(value.trim(), 16).ok();
                }
            }
            "SymLink" => self.is_symlink = value != "-",
            "Host OS" => {
                self.host_os = match value {
                    "FAT" => Some(0),
                    "Windows" => Some(1),
                    "Unix" => Some(2),
                    _ => None,
                };
            }
            _ => {}
        }
    }
}

fn parse_7z_time(s: &str) -> Option<DateTime<Utc>> {
    if s == "-" {
        return None;
    }
    NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
        .ok()
        .map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc))
}

fn detect_format(path: &Path) -> ArchiveFormat {
    match path.extension().and_then(|e| e.to_str()) {
        Some("7z") => ArchiveFormat::SevenZip,
        Some("zip") => ArchiveFormat::Zip,
        Some("tar") => ArchiveFormat::Tar,
        Some("gz") => {
            if path.to_string_lossy().ends_with(".tar.gz") {
                ArchiveFormat::TarGz
            } else {
                ArchiveFormat::Tar
            }
        }
        Some("bz2") => {
            if path.to_string_lossy().ends_with(".tar.bz2") {
                ArchiveFormat::TarBz2
            } else {
                ArchiveFormat::Tar
            }
        }
        Some("xz") => {
            if path.to_string_lossy().ends_with(".tar.xz") {
                ArchiveFormat::TarXz
            } else {
                ArchiveFormat::Tar
            }
        }
        Some("rar") => ArchiveFormat::Rar,
        _ => ArchiveFormat::Zip,
    }
}

impl CliReferee {
    fn parse_list_output(&self, stdout: &str, archive_path: &Path) -> Result<SessionState, String> {
        let mut entries: Vec<ParsedEntry> = Vec::new();
        let mut current: Option<ParsedEntry> = None;
        let mut in_entries = false;

        for line in stdout.lines() {
            let line = line.trim();
            if line.starts_with("----------") {
                in_entries = true;
                continue;
            }
            if !in_entries {
                continue;
            }
            if line.is_empty() {
                if let Some(entry) = current.take()
                    && !entry.path.is_empty()
                {
                    entries.push(entry);
                }
                continue;
            }
            if line == "--" {
                continue;
            }
            if current.is_none() && line.contains(" = ") {
                current = Some(ParsedEntry::default());
            }
            if let Some(ref mut entry) = current
                && let Some((key, value)) = line.split_once(" = ")
            {
                entry.apply(key.trim(), value.trim());
            }
        }
        if let Some(entry) = current.take()
            && !entry.path.is_empty()
        {
            entries.push(entry);
        }

        let root_id = next_vfs_id();
        let mut base_tree = Tree::new(root_id);
        let mut metadata_cache = HashMap::new();

        base_tree
            .insert_node(VfsNode {
                id: root_id,
                parent: None,
                name: String::new(),
                is_directory: true,
                original_index: None,
                fs_path: None,
            })
            .ok();

        let mut path_map: HashMap<String, VfsNodeId> = HashMap::new();
        path_map.insert(String::new(), root_id);

        for (idx, entry) in entries.iter().enumerate() {
            let node_id = next_vfs_id();
            let path = &entry.path;
            let is_dir = entry.is_directory;

            let parent_path = path
                .rsplit_once('/')
                .map(|(p, _)| p.to_string())
                .unwrap_or_default();

            if !path_map.contains_key(&parent_path) {
                Self::ensure_parents(&mut base_tree, &mut path_map, root_id, &parent_path);
            }
            let parent_id = *path_map.get(&parent_path).unwrap_or(&root_id);
            let name = path
                .rsplit_once('/')
                .map(|(_, n)| n.to_string())
                .unwrap_or_else(|| path.clone());

            base_tree
                .insert_node(VfsNode {
                    id: node_id,
                    parent: Some(parent_id),
                    name,
                    is_directory: is_dir,
                    original_index: if is_dir { None } else { Some(idx as u32) },
                    fs_path: None,
                })
                .ok();

            path_map.insert(path.clone(), node_id);

            metadata_cache.insert(
                node_id,
                VfsMetadata {
                    size: entry.size,
                    compressed_size: entry.compressed_size,
                    modified: entry.modified,
                    accessed: entry.accessed,
                    created: entry.created,
                    crc: entry.crc,
                    is_encrypted: entry.is_encrypted,
                    is_symlink: entry.is_symlink,
                    attributes: entry.attributes,
                    posix_attrib: entry.posix_attrib,
                    host_os: entry.host_os,
                    compression_method: entry.compression_method.clone(),
                    comment: None,
                    user: None,
                    group: None,
                    extension: None,
                    hardlink: None,
                },
            );
        }

        let fmt = detect_format(archive_path);
        let session = ArchiveSession::new(archive_path.to_path_buf(), fmt);
        let vfs = OverlayVfs::build_for_test(base_tree, metadata_cache.clone());

        Ok(SessionState {
            session,
            vfs,
            dirty_tree: Default::default(),
            edit_queue: Default::default(),
            metadata_cache,
        })
    }

    fn ensure_parents(
        tree: &mut Tree,
        path_map: &mut HashMap<String, VfsNodeId>,
        root_id: VfsNodeId,
        path: &str,
    ) {
        if path.is_empty() || path_map.contains_key(path) {
            return;
        }
        let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
        let mut current = String::new();
        for part in parts {
            let child = if current.is_empty() {
                part.to_string()
            } else {
                format!("{current}/{part}")
            };
            if !path_map.contains_key(&child) {
                let id = next_vfs_id();
                let parent = *path_map.get(&current).unwrap_or(&root_id);
                tree.insert_node(VfsNode {
                    id,
                    parent: Some(parent),
                    name: part.to_string(),
                    is_directory: true,
                    original_index: None,
                    fs_path: None,
                })
                .ok();
                path_map.insert(child.clone(), id);
            }
            current = child;
        }
    }
}

impl TestHarness for CliReferee {
    fn open_archive(
        &self,
        path: &Path,
        password: Option<&str>,
    ) -> Result<SessionState, String> {
        let path_str = path.to_string_lossy().to_string();
        let pw_arg;
        let mut args = vec!["l", "-slt"];
        if let Some(pw) = password {
            pw_arg = format!("-p{pw}");
            args.push(&pw_arg);
        }
        args.push(&path_str);
        let stdout = self.run_7z(&args)?;
        self.parse_list_output(&stdout, path)
    }

    fn extract_all(
        &self,
        path: &Path,
        dest: &Path,
        password: Option<&str>,
    ) -> Result<(), String> {
        let path_str = path.to_string_lossy().to_string();
        let dest_str = dest.to_string_lossy().to_string();
        let pw_arg;
        let out_arg = format!("-o{dest_str}");
        let mut args = vec!["x", &path_str];
        if let Some(pw) = password {
            pw_arg = format!("-p{pw}");
            args.push(&pw_arg);
        }
        args.push(&out_arg);
        args.push("-y");
        self.run_7z(&args)?;
        Ok(())
    }

    fn test_archive(
        &self,
        path: &Path,
        password: Option<&str>,
    ) -> Result<TestIntegrity, String> {
        let path_str = path.to_string_lossy().to_string();
        let pw_arg;
        let mut args = vec!["t", &path_str];
        if let Some(pw) = password {
            pw_arg = format!("-p{pw}");
            args.push(&pw_arg);
        }
        let output = Command::new(&self.seven_zip_path)
            .args(&args)
            .output()
            .map_err(|e| format!("failed to run 7z: {e}"))?;
        let passed = output.status.success();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let failures: Vec<String> = stdout
            .lines()
            .filter(|l| l.contains("ERROR") || l.contains("FAILED"))
            .map(|l| l.to_string())
            .collect();
        Ok(TestIntegrity { passed, failures })
    }
}
