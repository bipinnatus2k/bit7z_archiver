use crate::archive::*;
use std::collections::HashMap;
use std::path::PathBuf;

#[must_use = "ExecutionPlan must be passed to apply_changes"]
#[derive(Debug)]
pub struct ExecutionPlan {
    pub deletes: Vec<u32>,
    pub renames: Vec<(u32, String)>,
    pub adds: Vec<(PathBuf, String)>,
    pub updates: Vec<(PathBuf, String)>,
    pub conflicts: Vec<Conflict>,
}

#[derive(Debug, Clone)]
pub struct Conflict {
    pub change_index: usize,
    pub archive_path: String,
    pub existing: ArchiveEntry,
    pub incoming_size: u64,
    pub incoming_mtime: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictResolution {
    Apply,
    Skip,
}

impl ExecutionPlan {
    pub fn has_writes(&self) -> bool {
        !self.deletes.is_empty()
            || !self.renames.is_empty()
            || !self.adds.is_empty()
            || !self.updates.is_empty()
    }

    pub fn has_conflicts(&self) -> bool {
        !self.conflicts.is_empty()
    }

    pub fn apply_resolutions(&mut self, resolutions: &[ConflictResolution]) {
        debug_assert_eq!(resolutions.len(), self.conflicts.len());

        let conflicts = std::mem::take(&mut self.conflicts);
        for (conflict, resolution) in conflicts.into_iter().zip(resolutions.iter()) {
            if *resolution == ConflictResolution::Apply {
                self.updates.push((PathBuf::new(), conflict.archive_path));
            }
        }
    }
}

pub fn plan_changes(snapshot: &[ArchiveEntry], change_set: &ChangeSet) -> ExecutionPlan {
    let mut plan = ExecutionPlan {
        deletes: Vec::new(),
        renames: Vec::new(),
        adds: Vec::new(),
        updates: Vec::new(),
        conflicts: Vec::new(),
    };

    let existing_paths: HashMap<&str, &ArchiveEntry> =
        snapshot.iter().map(|e| (e.path.as_str(), e)).collect();

    for (i, change) in change_set.iter().enumerate() {
        match change {
            ArchiveChange::Add {
                fs_path,
                archive_path,
            } => {
                if let Some(existing) = existing_paths.get(archive_path.as_str()) {
                    let incoming_size = std::fs::metadata(fs_path).map(|m| m.len()).unwrap_or(0);
                    let incoming_mtime = std::fs::metadata(fs_path)
                        .ok()
                        .and_then(|m| m.modified().ok())
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs() as i64);

                    plan.conflicts.push(Conflict {
                        change_index: i,
                        archive_path: archive_path.clone(),
                        existing: (*existing).clone(),
                        incoming_size,
                        incoming_mtime,
                    });
                } else {
                    plan.adds.push((fs_path.clone(), archive_path.clone()));
                }
            }
            ArchiveChange::Update {
                fs_path,
                archive_path,
            } => {
                plan.updates.push((fs_path.clone(), archive_path.clone()));
            }
            ArchiveChange::Delete { index } => {
                plan.deletes.push(*index);
            }
            ArchiveChange::Rename { index, new_path } => {
                plan.renames.push((*index, new_path.clone()));
            }
        }
    }

    plan
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_changes_add_no_conflict() {
        let snapshot = vec![ArchiveEntry {
            path: "existing.txt".into(),
            ..Default::default()
        }];
        let mut change_set = ChangeSet::new();
        change_set.add(PathBuf::from("/tmp/new.txt"), "new.txt".into());

        let plan = plan_changes(&snapshot, &change_set);

        assert_eq!(plan.adds.len(), 1);
        assert!(plan.conflicts.is_empty());
    }

    #[test]
    fn test_plan_changes_add_with_conflict() {
        let snapshot = vec![ArchiveEntry {
            path: "existing.txt".into(),
            size: 100,
            ..Default::default()
        }];
        let mut change_set = ChangeSet::new();
        change_set.add(PathBuf::from("/tmp/existing.txt"), "existing.txt".into());

        let plan = plan_changes(&snapshot, &change_set);

        assert!(plan.adds.is_empty());
        assert_eq!(plan.conflicts.len(), 1);
        assert_eq!(plan.conflicts[0].archive_path, "existing.txt");
    }

    #[test]
    fn test_plan_changes_delete() {
        let snapshot = vec![ArchiveEntry {
            path: "file.txt".into(),
            original_index: 0,
            ..Default::default()
        }];
        let mut change_set = ChangeSet::new();
        change_set.delete(0);

        let plan = plan_changes(&snapshot, &change_set);

        assert_eq!(plan.deletes.len(), 1);
        assert_eq!(plan.deletes[0], 0);
    }

    #[test]
    fn test_plan_changes_rename() {
        let snapshot = vec![ArchiveEntry {
            path: "old.txt".into(),
            original_index: 0,
            ..Default::default()
        }];
        let mut change_set = ChangeSet::new();
        change_set.rename(0, "new.txt".into());

        let plan = plan_changes(&snapshot, &change_set);

        assert_eq!(plan.renames.len(), 1);
        assert_eq!(plan.renames[0], (0, "new.txt".into()));
    }

    #[test]
    fn test_execution_plan_has_writes() {
        let plan = ExecutionPlan {
            deletes: vec![],
            renames: vec![],
            adds: vec![],
            updates: vec![],
            conflicts: vec![],
        };
        assert!(!plan.has_writes());

        let plan = ExecutionPlan {
            deletes: vec![0],
            renames: vec![],
            adds: vec![],
            updates: vec![],
            conflicts: vec![],
        };
        assert!(plan.has_writes());
    }
}
