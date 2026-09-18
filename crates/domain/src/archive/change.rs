use std::path::PathBuf;

/// A single change to be applied to an archive.
#[derive(Debug, Clone)]
pub enum ArchiveChange {
    Add {
        fs_path: PathBuf,
        archive_path: String,
    },
    Update {
        fs_path: PathBuf,
        archive_path: String,
    },
    Delete {
        index: u32,
    },
    Rename {
        index: u32,
        new_path: String,
    },
}

/// A batch of changes to be applied to an archive atomically.
#[must_use = "ChangeSet must be applied via plan_changes/apply_changes"]
#[derive(Debug, Clone, Default)]
pub struct ChangeSet {
    changes: Vec<ArchiveChange>,
}

impl ChangeSet {
    pub fn new() -> Self {
        Self {
            changes: Vec::new(),
        }
    }

    pub fn add(&mut self, fs_path: PathBuf, archive_path: String) {
        self.changes.push(ArchiveChange::Add {
            fs_path,
            archive_path,
        });
    }

    pub fn update(&mut self, fs_path: PathBuf, archive_path: String) {
        self.changes.push(ArchiveChange::Update {
            fs_path,
            archive_path,
        });
    }

    pub fn delete(&mut self, index: u32) {
        self.changes.push(ArchiveChange::Delete { index });
    }

    pub fn rename(&mut self, index: u32, new_path: String) {
        self.changes.push(ArchiveChange::Rename { index, new_path });
    }

    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }

    pub fn len(&self) -> usize {
        self.changes.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &ArchiveChange> {
        self.changes.iter()
    }
}
