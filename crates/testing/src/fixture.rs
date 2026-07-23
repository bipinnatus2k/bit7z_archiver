use std::collections::HashMap;
use std::path::{Path, PathBuf};

use bit7z_domain::archive::{ArchiveFormat, ArchiveSession};
use bit7z_domain::vfs::{
    next_vfs_id, OverlayVfs, SessionState, Tree, VfsMetadata, VfsNode, VfsNodeId,
};
use chrono::{DateTime, Utc};

pub struct FileSpec {
    pub path: &'static str,
    pub content: &'static [u8],
    pub is_directory: bool,
    pub modified: Option<DateTime<Utc>>,
    pub attributes: Option<u32>,
}

pub struct ArchiveFixture {
    pub description: &'static str,
    pub format: ArchiveFormat,
    pub password: Option<&'static str>,
    pub expected: SessionState,
    pub archive_path: PathBuf,
    _dir: tempfile::TempDir,
}

impl ArchiveFixture {
    pub fn build(
        description: &'static str,
        format: ArchiveFormat,
        password: Option<&'static str>,
        entries: &[FileSpec],
    ) -> Result<Self, String> {
        let dir = tempfile::TempDir::new().map_err(|e| e.to_string())?;
        let root = dir.path();

        let (ground_truth, _) = Self::materialize_vfs(entries, format, password)?;
        Self::materialize_fs(entries, root)?;

        let ext = match format {
            ArchiveFormat::SevenZip => "7z",
            ArchiveFormat::Zip => "zip",
            ArchiveFormat::Tar => "tar",
            ArchiveFormat::TarGz => "tar.gz",
            _ => {
                return Err(format!(
                    "format {format:?} not supported by bit7z Writer::create"
                ))
            }
        };
        let archive_path = root.join(format!("test.{ext}"));

        let lib_path = bit7z_infra_platform::find_7z_library()
            .ok_or_else(|| "7z library not found".to_string())?;
        let lib = bit7z_infra_bit7z::Library::open(&lib_path.to_string_lossy())
            .map_err(|e| format!("open library: {e}"))?;

        let writer_format = match format {
            ArchiveFormat::SevenZip => bit7z_infra_bit7z::WriterFormat::SevenZip,
            ArchiveFormat::Zip => bit7z_infra_bit7z::WriterFormat::Zip,
            ArchiveFormat::Tar => bit7z_infra_bit7z::WriterFormat::Tar,
            ArchiveFormat::TarGz => bit7z_infra_bit7z::WriterFormat::GZip,
            _ => unreachable!(),
        };
        let writer =
            bit7z_infra_bit7z::Writer::create(&lib, writer_format).map_err(|e| e.to_string())?;

        if let Some(pw) = password {
            writer.set_password(pw);
        }

        let items: Vec<(String, &str)> = entries
            .iter()
            .filter(|e| !e.is_directory)
            .map(|e| (root.join(e.path).to_str().expect("non-UTF8 path").to_string(), e.path))
            .collect();
        let items_refs: Vec<(&str, &str)> = items.iter().map(|(p, n)| (p.as_str(), *n)).collect();

        writer
            .add_items(&items_refs)
            .map_err(|e| e.to_string())?;
        writer
            .compress_to(&archive_path.to_string_lossy())
            .map_err(|e| e.to_string())?;

        Ok(Self {
            description,
            format,
            password,
            expected: ground_truth,
            archive_path,
            _dir: dir,
        })
    }

    fn materialize_vfs(
        entries: &[FileSpec],
        format: ArchiveFormat,
        password: Option<&str>,
    ) -> Result<(SessionState, HashMap<String, VfsNodeId>), String> {
        let root_id = next_vfs_id();
        let mut base_tree = Tree::new(root_id);
        let mut metadata_cache = HashMap::new();
        let mut path_map: HashMap<String, VfsNodeId> = HashMap::new();
        let mut id_counter = 0u32;

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
        path_map.insert(String::new(), root_id);

        for entry in entries {
            let node_id = next_vfs_id();
            let path = entry.path.to_string();

            let parent_path = path
                .rsplit_once('/')
                .map(|(p, _)| p.to_string())
                .unwrap_or_default();

            if !path_map.contains_key(&parent_path) {
                Self::ensure_dir_vfs(&mut base_tree, &mut path_map, root_id, &parent_path)?;
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
                    is_directory: entry.is_directory,
                    original_index: if entry.is_directory {
                        None
                    } else {
                        Some(id_counter)
                    },
                    fs_path: None,
                })
                .ok();
            path_map.insert(path.clone(), node_id);

            let crc = if entry.is_directory {
                None
            } else {
                Some(crc32fast::hash(entry.content))
            };

            metadata_cache.insert(
                node_id,
                VfsMetadata {
                    size: entry.content.len() as u64,
                    compressed_size: 0,
                    modified: entry.modified,
                    created: None,
                    accessed: None,
                    crc,
                    is_encrypted: password.is_some() && format.supports_encryption(),
                    is_symlink: false,
                    attributes: entry.attributes,
                    posix_attrib: None,
                    host_os: None,
                    compression_method: None,
                    comment: None,
                    user: None,
                    group: None,
                    extension: None,
                    hardlink: None,
                },
            );

            if !entry.is_directory {
                id_counter += 1;
            }
        }

        let session = ArchiveSession::new(PathBuf::new(), format);
        let vfs = OverlayVfs::build_for_test(base_tree, metadata_cache.clone(), Some(format));

        Ok((
            SessionState {
                session,
                vfs,
                dirty_tree: Default::default(),
                edit_queue: Default::default(),
                metadata_cache,
            },
            path_map,
        ))
    }

    fn materialize_fs(entries: &[FileSpec], root: &Path) -> Result<(), String> {
        for entry in entries {
            if entry.is_directory {
                let dir_path = root.join(entry.path);
                std::fs::create_dir_all(&dir_path).map_err(|e| e.to_string())?;
            } else {
                let file_path = root.join(entry.path);
                if let Some(parent) = file_path.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                std::fs::write(&file_path, entry.content).map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }

    fn ensure_dir_vfs(
        tree: &mut Tree,
        path_map: &mut HashMap<String, VfsNodeId>,
        root_id: VfsNodeId,
        path: &str,
    ) -> Result<(), String> {
        if path.is_empty() || path_map.contains_key(path) {
            return Ok(());
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
                .map_err(|e| e.to_string())?;
                path_map.insert(child.clone(), id);
            }
            current = child;
        }
        Ok(())
    }
}
