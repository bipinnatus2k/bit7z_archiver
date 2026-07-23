# Cross-Harness Test Framework Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a reusable test framework with `TestHarness` trait, `Bit7zHarness` (via usecases) and `CliReferee` (via 7z CLI), that defines ground truth using existing VFS types and cross-validates results.

**Architecture:** New `crates/testing/` crate containing the harness trait, two implementations, fixture builder, and assertion helpers. Ground truth uses existing `SessionState` / `Tree` / `VfsMetadata` types from domain. Integration tests under `crates/testing/tests/` define fixtures and run comparisons.

**Tech Stack:** Rust, existing `domain` / `runtime` / `application/archive` crates, `std::process::Command` for CLI, `tempfile` for scratch dirs, `crc32fast` for ground truth CRC computation.

## Global Constraints

- No new domain types. No fields added to `ArchiveEntry`, `VfsNode`, or `VfsMetadata`.
- CRC verification uses the existing `VfsMetadata.crc: Option<u32>` field.
- `Bit7zHarness` must go through the usecase layer (not bypass it).
- Test archives must be created programmatically (no checked-in binary fixtures).

---

### Task 1: Add OverlayVfs structural comparison method

**Files:**
- Modify: `crates/domain/src/vfs/overlay.rs`
- Modify: `crates/domain/src/vfs/tree.rs`

**Interfaces:**
- Produces: `OverlayVfs::assert_structural_eq(&self, other: &OverlayVfs)` and `Tree::all_paths()`

- [ ] **Step 1: Add `all_paths()` to `Tree`**

In `crates/domain/src/vfs/tree.rs`, inside `impl Tree`:

```rust
/// Return all node paths sorted.
pub fn all_paths(&self) -> Vec<String> {
    let mut paths: Vec<String> = self.nodes.keys()
        .filter_map(|&id| self.path_of(id))
        .collect();
    paths.sort();
    paths
}
```

- [ ] **Step 2: Add `assert_structural_eq` to `OverlayVfs`**

In `crates/domain/src/vfs/overlay.rs`, inside `impl OverlayVfs`:

```rust
/// Panics if `other` has a different tree structure or metadata.
/// Used by the cross-harness test framework.
pub fn assert_structural_eq(&self, other: &OverlayVfs) {
    let base = self.base_tree();
    let other_base = other.base_tree();

    let paths = base.all_paths();
    let other_paths = other_base.all_paths();
    assert_eq!(paths, other_paths, "tree structure mismatch");

    for path in &paths {
        let id = base.resolve_path(path).unwrap();
        let other_id = other_base.resolve_path(path).unwrap();
        let meta = self.metadata_cache.get(&id);
        let other_meta = other.metadata_cache.get(&other_id);
        assert_eq!(meta, other_meta, "metadata mismatch for '{path}'");
    }
}
```

- [ ] **Step 3: Run existing domain tests**

```bash
cargo test -p bit7z-domain
```

- [ ] **Step 4: Commit**

```bash
git add -A && git commit -m "feat: add OverlayVfs::assert_structural_eq and Tree::all_paths"
```

---

### Task 2: Add get_session_state to ArchiveService

**Files:**
- Modify: `crates/application/archive/src/runtime_service.rs`

**Interfaces:**
- Produces: `ArchiveService::get_session_state(&self, archive: &ArchiveHandle) -> Result<SessionState>`

- [ ] **Step 1: Implement the method**

In `crates/application/archive/src/runtime_service.rs`, inside `impl ArchiveService`:

```rust
/// Retrieve the full session state (VFS tree + metadata) for an archive.
/// Used by the test framework to inspect entries.
pub fn get_session_state(&self, archive: &ArchiveHandle) -> Result<SessionState, ArchiveError> {
    let session = self.lookup_session(archive)?;
    self.runtime
        .session_manager
        .get(session.id)
        .ok_or_else(|| ArchiveError::NotFound(format!("session {}", session.id)))
        .map(|ref_state| (*ref_state).clone())
}
```

- [ ] **Step 2: Build check**

```bash
cargo check -p bit7z-app-archive
```

- [ ] **Step 3: Commit**

```bash
git add -A && git commit -m "feat: add ArchiveService::get_session_state for testing"
```

---

### Task 3: Create testing crate skeleton

**Files:**
- Create: `crates/testing/Cargo.toml`
- Create: `crates/testing/src/lib.rs`
- Modify: `Cargo.toml` (workspace members)

- [ ] **Step 1: Create `crates/testing/Cargo.toml`**

```toml
[package]
name = "bit7z-testing"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
bit7z-domain = { path = "../domain" }
bit7z-app-archive = { path = "../application/archive" }
bit7z-runtime = { path = "../runtime" }
bit7z-ports = { path = "../ports" }
bit7z-infra-bit7z = { path = "../infrastructure/bit7z" }
bit7z-infra-persistence = { path = "../infrastructure/persistence" }
tempfile = { workspace = true }
chrono = { workspace = true }
crc32fast = "1"
```

- [ ] **Step 2: Create `crates/testing/src/lib.rs`**

```rust
pub mod harness;
pub mod bit7z_harness;
pub mod cli_referee;
pub mod fixture;
pub mod assert;
```

- [ ] **Step 3: Add to workspace**

In root `Cargo.toml`, add `"crates/testing"` to the `members` list (alphabetically).

- [ ] **Step 4: Verify**

```bash
cargo check -p bit7z-testing
```

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat: add testing crate skeleton"
```

---

### Task 4: TestHarness trait + types

**Files:**
- Create: `crates/testing/src/harness.rs`

**Interfaces:**
- Produces: `TestHarness` trait, `TestIntegrity` struct

- [ ] **Step 1: Write the trait and types**

```rust
use std::path::Path;
use bit7z_domain::vfs::SessionState;

/// Result of an integrity test.
#[derive(Debug, Clone, PartialEq)]
pub struct TestIntegrity {
    pub passed: bool,
    pub failures: Vec<String>,
}

/// Programmatic interface to archive operations.
///
/// Two implementations: `Bit7zHarness` (via usecases) and
/// `CliReferee` (via 7z/rar CLI).
pub trait TestHarness: Send + Sync {
    /// Open an archive and return the full session state.
    fn open_archive(
        &self,
        path: &Path,
        password: Option<&str>,
    ) -> Result<SessionState, String>;

    /// Extract all entries to destination directory.
    fn extract_all(
        &self,
        path: &Path,
        dest: &Path,
        password: Option<&str>,
    ) -> Result<(), String>;

    /// Run integrity test on archive.
    fn test_archive(
        &self,
        path: &Path,
        password: Option<&str>,
    ) -> Result<TestIntegrity, String>;
}
```

- [ ] **Step 2: Build check**

```bash
cargo check -p bit7z-testing
```

- [ ] **Step 3: Commit**

```bash
git add -A && git commit -m "feat: add TestHarness trait"
```

---

### Task 5: Bit7zHarness implementation

**Files:**
- Create: `crates/testing/src/bit7z_harness.rs`

**Interfaces:**
- Produces: `Bit7zHarness` struct implementing `TestHarness`

- [ ] **Step 1: Write Bit7zHarness**

```rust
use std::path::Path;
use std::sync::Arc;

use bit7z_app_archive::runtime_service::ArchiveService;
use bit7z_domain::archive::{Password, ArchiveHandle};
use bit7z_domain::vfs::SessionState;
use bit7z_domain::repository::ArchiveError;

use crate::harness::{TestHarness, TestIntegrity};

pub struct Bit7zHarness {
    pub service: Arc<ArchiveService>,
}

impl Bit7zHarness {
    pub fn new(service: Arc<ArchiveService>) -> Self {
        Self { service }
    }

    fn to_string(e: ArchiveError) -> String {
        e.to_string()
    }

    fn open_inner(
        &self,
        path: &Path,
        password: Option<&str>,
    ) -> Result<(ArchiveHandle, SessionState), String> {
        let pw = password.map(|p| Password::new(p.to_string()));
        let session = self.service.open(path, pw.as_ref()).map_err(Self::to_string)?;
        let state = self
            .service
            .get_session_state(&session)
            .map_err(Self::to_string)?;
        Ok((session, state))
    }
}

impl TestHarness for Bit7zHarness {
    fn open_archive(
        &self,
        path: &Path,
        password: Option<&str>,
    ) -> Result<SessionState, String> {
        Ok(self.open_inner(path, password)?.1)
    }

    fn extract_all(
        &self,
        path: &Path,
        dest: &Path,
        password: Option<&str>,
    ) -> Result<(), String> {
        let (handle, state) = self.open_inner(path, password)?;

        let indices: Vec<u32> = state
            .vfs
            .base_tree()
            .all_ids()
            .iter()
            .filter_map(|&id| {
                let node = state.vfs.base_tree().node(id)?;
                if !node.is_directory {
                    node.original_index
                } else {
                    None
                }
            })
            .collect();

        use bit7z_app_archive::extract::ExtractEntriesUseCase;
        use bit7z_domain::archive::OverwriteMode;
        use bit7z_domain::repository::{ExtractOptions, NoopNotifier};
        use std::sync::Arc;
        use std::sync::atomic::AtomicBool;

        let usecase = ExtractEntriesUseCase::new(self.service.clone());
        usecase
            .execute(
                &handle,
                &indices,
                dest,
                &ExtractOptions {
                    overwrite_mode: OverwriteMode::Overwrite,
                    cancel: Arc::new(AtomicBool::new(false)),
                    paused: Arc::new(AtomicBool::new(false)),
                    notifier: Arc::new(NoopNotifier),
                },
            )
            .map_err(Self::to_string)
    }

    fn test_archive(
        &self,
        path: &Path,
        password: Option<&str>,
    ) -> Result<TestIntegrity, String> {
        let (handle, _state) = self.open_inner(path, password)?;
        let result = self.service.test(&handle).map_err(Self::to_string)?;
        let failures: Vec<String> = result
            .failed
            .iter()
            .map(|f| format!("[{}] {}: {}", f.index, f.entry_path, f.error))
            .collect();
        Ok(TestIntegrity {
            passed: result.failed.is_empty(),
            failures,
        })
    }
}
```

Also add the needed re-export from `bit7z-app-archive` to expose `ExtractOptions` and `OverwriteMode`. These are already public in the `extract` module but verify with `cargo check`.

- [ ] **Step 2: Build check**

```bash
cargo check -p bit7z-testing
```

- [ ] **Step 3: Commit**

```bash
git add -A && git commit -m "feat: add Bit7zHarness"
```

---

### Task 6: CliReferee with 7z output parser

**Files:**
- Create: `crates/testing/src/cli_referee.rs`
- Modify: `crates/domain/src/vfs/overlay.rs` (add `build_for_test`)

**Interfaces:**
- Produces: `CliReferee` struct implementing `TestHarness`

- [ ] **Step 1: Add `build_for_test` to OverlayVfs**

In `crates/domain/src/vfs/overlay.rs`:

```rust
/// Build OverlayVfs from pre-constructed Tree + metadata (for testing).
pub fn build_for_test(
    base_tree: Tree,
    metadata_cache: HashMap<VfsNodeId, VfsMetadata>,
) -> Self {
    Self {
        base_tree: base_tree.clone(),
        working_tree: base_tree,
        dirty_tree: DirtyTree::new(),
        edit_queue: EditQueue::new(),
        metadata_cache,
        archive_path: PathBuf::new(),
        archive_format: None,
    }
}
```

- [ ] **Step 2: Write the 7z output parser + CliReferee**

```rust
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use bit7z_domain::vfs::{
    next_vfs_id, SessionState, VfsNode, VfsNodeId, VfsMetadata, OverlayVfs, Tree,
};
use bit7z_domain::archive::{ArchiveSession, ArchiveFormat};
use chrono::{DateTime, Utc, NaiveDateTime};

use crate::harness::{TestHarness, TestIntegrity};

pub struct CliReferee {
    pub seven_zip_path: PathBuf,
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
                if let Some(attrs) = value.split_whitespace().nth(1) {
                    self.attributes =
                        Some(attrs.bytes().fold(0u32, |acc, b| acc.wrapping_add(b as u32)));
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

impl CliReferee {
    fn parse_list_output(
        &self,
        stdout: &str,
        archive_path: &Path,
    ) -> Result<SessionState, String> {
        let mut entries: Vec<ParsedEntry> = Vec::new();
        let mut current: Option<ParsedEntry> = None;

        for line in stdout.lines() {
            let line = line.trim();
            if line == "--" {
                if let Some(entry) = current.take() {
                    entries.push(entry);
                }
                current = Some(ParsedEntry::default());
                continue;
            }
            if let Some(ref mut entry) = current {
                if let Some((key, value)) = line.split_once(" = ") {
                    entry.apply(key.trim(), value.trim());
                }
            }
        }
        if let Some(entry) = current.take() {
            if !entry.path.is_empty() {
                entries.push(entry);
            }
        }

        // Build VFS tree
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

            // Ensure parent exists
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

        let session = ArchiveSession::new(archive_path.to_path_buf(), ArchiveFormat::Zip);
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
        let mut args = vec!["l", "-slt"];
        if let Some(pw) = password {
            args.push(&format!("-p{pw}"));
        }
        args.push(path.to_str().unwrap());
        let stdout = self.run_7z(&args)?;
        self.parse_list_output(&stdout, path)
    }

    fn extract_all(
        &self,
        path: &Path,
        dest: &Path,
        password: Option<&str>,
    ) -> Result<(), String> {
        let mut args = vec!["x", &path.to_string_lossy()];
        if let Some(pw) = password {
            args.push(&format!("-p{pw}"));
        }
        args.push(&format!("-o{}", dest.to_string_lossy()));
        args.push("-y");
        self.run_7z(&args)?;
        Ok(())
    }

    fn test_archive(
        &self,
        path: &Path,
        password: Option<&str>,
    ) -> Result<TestIntegrity, String> {
        let mut args = vec!["t", &path.to_string_lossy()];
        if let Some(pw) = password {
            args.push(&format!("-p{pw}"));
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
```

- [ ] **Step 3: Build check**

```bash
cargo check -p bit7z-testing
```

- [ ] **Step 4: Commit**

```bash
git add -A && git commit -m "feat: add CliReferee with 7z output parser"
```

---

### Task 7: Assertion functions

**Files:**
- Create: `crates/testing/src/assert.rs`

**Interfaces:**
- Produces: `assert_session_state_eq`, `assert_extraction_matches`, `assert_harnesses_agree`

- [ ] **Step 1: Write assertion functions**

```rust
use std::path::Path;
use bit7z_domain::vfs::SessionState;
use crate::harness::TestHarness;

/// Assert two SessionStates have identical tree structure + metadata.
pub fn assert_session_state_eq(expected: &SessionState, actual: &SessionState) {
    expected.vfs.assert_structural_eq(&actual.vfs);
}

/// Assert extracted files match ground truth by CRC.
pub fn assert_extraction_matches(ground_truth: &SessionState, extracted_root: &Path) {
    let tree = ground_truth.vfs.base_tree();
    for &id in tree.all_ids() {
        let node = tree.node(id).unwrap();
        if node.is_directory {
            continue;
        }
        let path = tree.path_of(id).unwrap();
        let file_path = extracted_root.join(&path);
        assert!(
            file_path.exists(),
            "missing extracted file: {path} (expected at {})",
            file_path.display()
        );

        let meta = ground_truth
            .metadata_cache
            .get(&id)
            .expect("missing metadata");

        if let Some(expected_crc) = meta.crc {
            let actual = std::fs::read(&file_path)
                .unwrap_or_else(|e| panic!("cannot read {path}: {e}"));
            let actual_crc = crc32fast::hash(&actual);
            assert_eq!(
                expected_crc, actual_crc,
                "CRC mismatch for '{path}': expected 0x{expected_crc:08X}, got 0x{actual_crc:08X}",
            );
        }
    }
}

/// Run both harnesses on the same archive and assert they agree.
pub fn assert_harnesses_agree(
    harness_a: &dyn TestHarness,
    harness_b: &dyn TestHarness,
    archive: &Path,
    password: Option<&str>,
) {
    let state_a = harness_a
        .open_archive(archive, password)
        .expect("harness_a open_archive failed");
    let state_b = harness_b
        .open_archive(archive, password)
        .expect("harness_b open_archive failed");
    assert_session_state_eq(&state_a, &state_b);
}
```

- [ ] **Step 2: Build check**

```bash
cargo check -p bit7z-testing
```

- [ ] **Step 3: Commit**

```bash
git add -A && git commit -m "feat: add assertion functions"
```

---

### Task 8: Fixture module (ground truth builder)

**Files:**
- Create: `crates/testing/src/fixture.rs`

**Interfaces:**
- Produces: `FileSpec`, `ArchiveFixture::build()`

- [ ] **Step 1: Write fixture module**

```rust
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use bit7z_domain::vfs::{
    next_vfs_id, SessionState, VfsNode, VfsNodeId, VfsMetadata, OverlayVfs, Tree,
};
use bit7z_domain::archive::ArchiveFormat;
use chrono::{DateTime, Utc};

/// Specification for a single entry in a test archive.
pub struct FileSpec {
    pub path: &'static str,
    pub content: &'static [u8],
    pub is_directory: bool,
    pub modified: Option<DateTime<Utc>>,
    pub attributes: Option<u32>,
}

/// A known-good archive with its ground truth SessionState.
pub struct ArchiveFixture {
    pub description: &'static str,
    pub format: ArchiveFormat,
    pub password: Option<&'static str>,
    pub expected: SessionState,
    pub archive_path: PathBuf,
}

impl ArchiveFixture {
    /// Build fixture: create temp files, compress, return ground truth + archive path.
    ///
    /// Uses the bit7z low-level Writer directly (same code path as usecase
    /// creation would) to produce a known-good archive from the given specs.
    pub fn build(
        description: &'static str,
        format: ArchiveFormat,
        password: Option<&'static str>,
        entries: &[FileSpec],
    ) -> Result<Self, String> {
        let dir = tempfile::TempDir::new().map_err(|e| e.to_string())?;
        let root = dir.path();

        let (ground_truth, _) = Self::materialize_vfs(entries)?;
        Self::materialize_fs(entries, root)?;

        let ext = match format {
            ArchiveFormat::SevenZip => "7z",
            ArchiveFormat::Zip => "zip",
            ArchiveFormat::Tar => "tar",
            ArchiveFormat::TarGz => "tar.gz",
            ArchiveFormat::TarBz2 => "tar.bz2",
            ArchiveFormat::Rar => "rar",
            _ => "zip",
        };
        let archive_path = root.join(format!("test.{ext}"));

        // Use bit7z low-level writer to create the archive
        let lib = {
            let path = bit7z_infra_platform::find_7z_library()
                .ok_or_else(|| "7z library not found".to_string())?;
            bit7z_infra_bit7z::Library::open(&path.to_string_lossy())
                .map_err(|e| format!("open library: {e}"))?
        };

        // WriterFormat does not have TarGz / TarBz2 variants.
        // For those formats, create the archive via CLI (`7z a`) instead.
        let writer_format = match format {
            ArchiveFormat::SevenZip => bit7z_infra_bit7z::WriterFormat::SevenZip,
            ArchiveFormat::Zip => bit7z_infra_bit7z::WriterFormat::Zip,
            ArchiveFormat::Tar => bit7z_infra_bit7z::WriterFormat::Tar,
            _ => return Err(format!(
                "format {format:?} not yet supported by bit7z Writer::create; use CLI to create fixture"
            )),
        };
        let mut writer =
            bit7z_infra_bit7z::Writer::create(&lib, writer_format).map_err(|e| e.to_string())?;

        if let Some(pw) = password {
            writer.set_password(pw);
        }

        // Collect file paths on disk
        let file_paths: Vec<String> = entries
            .iter()
            .filter(|e| !e.is_directory)
            .map(|e| root.join(e.path).to_string_lossy().to_string())
            .collect();

        let refs: Vec<&str> = file_paths.iter().map(|s| s.as_str()).collect();
        writer.add_files(&refs).map_err(|e| e.to_string())?;
        writer
            .compress_to(&archive_path.to_string_lossy())
            .map_err(|e| e.to_string())?;

        Ok(Self {
            description,
            format,
            password,
            expected: ground_truth,
            archive_path,
        })
    }

    fn materialize_vfs(
        entries: &[FileSpec],
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
                    is_encrypted: false,
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

        let session = bit7z_domain::archive::ArchiveSession::new(
            PathBuf::new(),
            ArchiveFormat::Zip,
        );
        let vfs = OverlayVfs::build_for_test(base_tree, metadata_cache.clone());

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
```

- [ ] **Step 2: Build check**

```bash
cargo check -p bit7z-testing
```

- [ ] **Step 3: Commit**

```bash
git add -A && git commit -m "feat: add fixture ground truth builder"
```

---

### Task 9: First integration test — list entries

**Files:**
- Create: `crates/testing/tests/fixtures/mod.rs`
- Create: `crates/testing/tests/fixtures/simple.rs`
- Create: `crates/testing/tests/test_list.rs`

- [ ] **Step 1: Create fixtures module**

```rust
// crates/testing/tests/fixtures/mod.rs
pub mod simple;
```

- [ ] **Step 2: Create simple fixture**

```rust
// crates/testing/tests/fixtures/simple.rs
use bit7z_testing::fixture::{FileSpec, ArchiveFixture};

pub fn create_zip() -> Result<ArchiveFixture, String> {
    let entries = vec![
        FileSpec {
            path: "hello.txt",
            content: b"Hello, World!",
            is_directory: false,
            modified: None,
            attributes: None,
        },
        FileSpec {
            path: "empty.bin",
            content: b"",
            is_directory: false,
            modified: None,
            attributes: None,
        },
    ];
    ArchiveFixture::build("simple two-file archive", ArchiveFormat::Zip, None, &entries)
}

pub fn create_7z() -> Result<ArchiveFixture, String> {
    let entries = vec![
        FileSpec {
            path: "hello.txt",
            content: b"Hello, World!",
            is_directory: false,
            modified: None,
            attributes: None,
        },
    ];
    ArchiveFixture::build("simple one-file 7z", ArchiveFormat::SevenZip, None, &entries)
}
```

- [ ] **Step 3: Write list test**

```rust
// crates/testing/tests/test_list.rs
use std::sync::Arc;
use bit7z_app_archive::runtime_service::{ArchiveService, build_bit7z_runtime};
use bit7z_testing::harness::TestHarness;
use bit7z_testing::bit7z_harness::Bit7zHarness;
use bit7z_testing::cli_referee::CliReferee;
use bit7z_testing::assert::{assert_session_state_eq, assert_harnesses_agree};

mod fixtures;

/// Skip test if 7-Zip library is not found on this system.
fn library() -> Option<bit7z_infra_bit7z::Library> {
    let path = bit7z_infra_platform::find_7z_library()?;
    bit7z_infra_bit7z::Library::open(&path.to_string_lossy()).ok()
}

fn build_harness() -> Bit7zHarness {
    let lib = library().expect("7-Zip library is required for tests");
    let (runtime, resolver) = build_bit7z_runtime(lib);
    let service = Arc::new(ArchiveService::new(runtime, resolver));
    Bit7zHarness::new(service)
}

#[test]
fn test_list_simple_zip() {
    let lib = match library() {
        Some(l) => l,
        None => return, // skip if no 7z library
    };
    let (runtime, resolver) = build_bit7z_runtime(lib);
    let service = Arc::new(ArchiveService::new(runtime, resolver));
    let harness = Bit7zHarness::new(service);

    let fixture = fixtures::simple::create_zip().expect("fixture creation failed");
    let cli = CliReferee::default();

    // Open with bit7z
    let bit7z_state = harness
        .open_archive(&fixture.archive_path, None)
        .expect("bit7z open failed");
    assert_session_state_eq(&fixture.expected, &bit7z_state);

    // Open with CLI
    let cli_state = cli
        .open_archive(&fixture.archive_path, None)
        .expect("cli open failed");
    assert_session_state_eq(&fixture.expected, &cli_state);

    // Cross-check
    assert_session_state_eq(&bit7z_state, &cli_state);
}
```

- [ ] **Step 4: Run the test**

```bash
cargo test -p bit7z-testing --test test_list -- --nocapture
```

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat: add first integration test (list, zip, simple)"
```

---

### Task 10: Extract + Test integration tests

**Files:**
- Create: `crates/testing/tests/test_extract.rs`
- Create: `crates/testing/tests/test_test.rs`
- Create: `crates/testing/tests/fixtures/nested.rs`

- [ ] **Step 1: Create nested fixture**

```rust
// crates/testing/tests/fixtures/nested.rs
use bit7z_testing::fixture::{FileSpec, ArchiveFixture};

pub fn create_zip() -> Result<ArchiveFixture, String> {
    let entries = vec![
        FileSpec {
            path: "root.txt",
            content: b"root",
            is_directory: false,
            modified: None,
            attributes: None,
        },
        FileSpec {
            path: "sub/dir/a.txt",
            content: b"alpha",
            is_directory: false,
            modified: None,
            attributes: None,
        },
        FileSpec {
            path: "sub/dir/b.bin",
            content: &[0x00, 0x01, 0x02, 0xFF],
            is_directory: false,
            modified: None,
            attributes: None,
        },
        FileSpec {
            path: "sub/empty",
            content: b"",
            is_directory: true,
            modified: None,
            attributes: None,
        },
    ];
    ArchiveFixture::build("nested directories", ArchiveFormat::Zip, None, &entries)
}
```

- [ ] **Step 2: Write extract test**

```rust
// crates/testing/tests/test_extract.rs
use std::sync::Arc;

use bit7z_app_archive::runtime_service::{ArchiveService, build_bit7z_runtime};
use bit7z_testing::harness::TestHarness;
use bit7z_testing::bit7z_harness::Bit7zHarness;
use bit7z_testing::cli_referee::CliReferee;
use bit7z_testing::assert::assert_extraction_matches;

mod fixtures;

fn library() -> Option<bit7z_infra_bit7z::Library> {
    let path = bit7z_infra_platform::find_7z_library()?;
    bit7z_infra_bit7z::Library::open(&path.to_string_lossy()).ok()
}

#[test]
fn test_extract_simple_zip() {
    let lib = match library() {
        Some(l) => l,
        None => return,
    };
    let (runtime, resolver) = build_bit7z_runtime(lib);
    let service = Arc::new(ArchiveService::new(runtime, resolver));
    let harness = Bit7zHarness::new(service);
    let cli = CliReferee::default();

    let fixture = fixtures::simple::create_zip().expect("fixture");

    let dest = tempfile::TempDir::new().unwrap();
    harness
        .extract_all(&fixture.archive_path, dest.path(), None)
        .unwrap();
    assert_extraction_matches(&fixture.expected, dest.path());

    let dest2 = tempfile::TempDir::new().unwrap();
    cli.extract_all(&fixture.archive_path, dest2.path(), None)
        .unwrap();
    assert_extraction_matches(&fixture.expected, dest2.path());
}
```

- [ ] **Step 3: Write integrity test**

```rust
// crates/testing/tests/test_test.rs
use std::sync::Arc;

use bit7z_app_archive::runtime_service::{ArchiveService, build_bit7z_runtime};
use bit7z_testing::harness::TestHarness;
use bit7z_testing::bit7z_harness::Bit7zHarness;
use bit7z_testing::cli_referee::CliReferee;

mod fixtures;

fn library() -> Option<bit7z_infra_bit7z::Library> {
    let path = bit7z_infra_platform::find_7z_library()?;
    bit7z_infra_bit7z::Library::open(&path.to_string_lossy()).ok()
}

#[test]
fn test_integrity_simple_zip() {
    let lib = match library() {
        Some(l) => l,
        None => return,
    };
    let (runtime, resolver) = build_bit7z_runtime(lib);
    let service = Arc::new(ArchiveService::new(runtime, resolver));
    let harness = Bit7zHarness::new(service);
    let cli = CliReferee::default();

    let fixture = fixtures::simple::create_zip().expect("fixture");

    let bit7z_result = harness
        .test_archive(&fixture.archive_path, None)
        .unwrap();
    assert!(bit7z_result.passed);

    let cli_result = cli.test_archive(&fixture.archive_path, None).unwrap();
    assert!(cli_result.passed);
}
```

- [ ] **Step 4: Run all tests**

```bash
cargo test -p bit7z-testing
```

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat: add extract and integrity tests + nested fixture"
```

---

### Task 11: Build & verify

- [ ] **Step 1: Full build**

```bash
cargo build
```

- [ ] **Step 2: Full test run**

```bash
cargo test
```

- [ ] **Step 3: Clippy**

```bash
cargo clippy --all-targets 2>&1
```

- [ ] **Step 4: Fix any issues and commit**

```bash
git add -A && git commit -m "fix: address clippy and build issues"
```
