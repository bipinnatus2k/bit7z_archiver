use bit7z_domain::archive::SessionId;
use bit7z_domain::repository::ArchiveError;
use bit7z_domain::vfs::OverlayVfs;
use bit7z_runtime::session::SessionManager;

/// Expand a list of entry indices so that any directory entry is replaced
/// by all of its descendant file indices. Non-directory entries are kept as-is.
pub fn expand_directory_entries(
    session_manager: &dyn SessionManager,
    session_id: SessionId,
    indices: &[u32],
) -> Result<Vec<u32>, ArchiveError> {
    let state = session_manager
        .get(session_id)
        .ok_or_else(|| ArchiveError::NotFound("session not found".into()))?;
    let vfs = &state.vfs;

    let mut expanded: Vec<u32> = Vec::new();
    for &idx in indices {
        let page = vfs.list_page(0, usize::MAX);
        let entry = page.items.iter().find(|e| e.original_index == idx);
        match entry {
            Some(e) if e.is_directory => collect_descendants(vfs, idx, &mut expanded),
            _ => expanded.push(idx),
        }
    }
    expanded.sort();
    expanded.dedup();
    Ok(expanded)
}

fn collect_descendants(vfs: &OverlayVfs, dir_idx: u32, out: &mut Vec<u32>) {
    let page = vfs.list_page(0, usize::MAX);
    let dir_path = page
        .items
        .iter()
        .find(|p| p.original_index == dir_idx)
        .map(|p| &*p.path);

    let Some(dir_path) = dir_path else {
        return;
    };

    let prefix = if dir_path.ends_with('/') {
        dir_path.to_string()
    } else {
        format!("{}/", dir_path)
    };

    let children: Vec<_> = page
        .items
        .iter()
        .filter(|e| e.path.starts_with(&prefix) && e.original_index != dir_idx)
        .cloned()
        .collect();

    for child in &children {
        if child.is_directory {
            collect_descendants(vfs, child.original_index, out);
        } else {
            out.push(child.original_index);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bit7z_domain::archive::{ArchiveEntry, ArchiveSession, ArchiveFormat, SessionId};
    use bit7z_domain::vfs::SessionState;
    use bit7z_ports::session::{SessionRef, SessionStore};
    use std::sync::{Arc, Mutex};
    use std::path::PathBuf;

    struct MockSessionStore {
        state: Mutex<Option<SessionState>>,
    }

    impl SessionStore for MockSessionStore {
        fn insert(&self, _state: SessionState) {}
        fn get(&self, _id: SessionId) -> Option<SessionRef> {
            self.state.lock().unwrap().clone().map(Arc::new)
        }
        fn remove(&self, _id: SessionId) -> Option<SessionState> { None }
        fn contains(&self, _id: SessionId) -> bool { true }
        fn all(&self) -> Vec<SessionId> { vec![] }
    }

    #[test]
    fn test_expand_directory_entries_flattens_dirs() {
        let dir = ArchiveEntry {
            name: "dir".into(),
            path: "dir".into(),
            original_index: 0,
            is_directory: true,
            ..ArchiveEntry::default()
        };
        let file1 = ArchiveEntry {
            name: "a.txt".into(),
            path: "dir/a.txt".into(),
            original_index: 1,
            ..ArchiveEntry::default()
        };
        let file2 = ArchiveEntry {
            name: "b.txt".into(),
            path: "dir/b.txt".into(),
            original_index: 2,
            ..ArchiveEntry::default()
        };

        let vfs = OverlayVfs::build(PathBuf::from("test.zip"), Some(ArchiveFormat::Zip), &[dir, file1, file2]);

        let session = ArchiveSession::new(PathBuf::from("test.zip"), ArchiveFormat::Zip);
        let store = Arc::new(MockSessionStore {
            state: Mutex::new(Some(SessionState {
                session,
                vfs,
                dirty_tree: Default::default(),
                edit_queue: Default::default(),
                metadata_cache: Default::default(),
            })),
        });
        let session_manager = bit7z_runtime::session::DefaultSessionManager::new(store);

        let result = expand_directory_entries(&session_manager, 1, &[0]).unwrap();
        assert_eq!(result, vec![1, 2]);
    }

    #[test]
    fn test_expand_directory_entries_does_not_match_sibling_prefix() {
        let dir = ArchiveEntry {
            name: "dir".into(),
            path: "dir".into(),
            original_index: 0,
            is_directory: true,
            ..ArchiveEntry::default()
        };
        let sibling = ArchiveEntry {
            name: "dir_extras".into(),
            path: "dir_extras".into(),
            original_index: 1,
            is_directory: true,
            ..ArchiveEntry::default()
        };
        let child = ArchiveEntry {
            name: "a.txt".into(),
            path: "dir/a.txt".into(),
            original_index: 2,
            ..ArchiveEntry::default()
        };
        let sibling_child = ArchiveEntry {
            name: "b.txt".into(),
            path: "dir_extras/b.txt".into(),
            original_index: 3,
            ..ArchiveEntry::default()
        };

        let vfs = OverlayVfs::build(
            PathBuf::from("test.zip"),
            Some(ArchiveFormat::Zip),
            &[dir, sibling, child, sibling_child],
        );

        let session = ArchiveSession::new(PathBuf::from("test.zip"), ArchiveFormat::Zip);
        let store = Arc::new(MockSessionStore {
            state: Mutex::new(Some(SessionState {
                session,
                vfs,
                dirty_tree: Default::default(),
                edit_queue: Default::default(),
                metadata_cache: Default::default(),
            })),
        });
        let session_manager = bit7z_runtime::session::DefaultSessionManager::new(store);

        let result = expand_directory_entries(&session_manager, 1, &[0]).unwrap();
        assert_eq!(result, vec![2]); // only "dir/a.txt", not "dir_extras/b.txt"
    }
}
