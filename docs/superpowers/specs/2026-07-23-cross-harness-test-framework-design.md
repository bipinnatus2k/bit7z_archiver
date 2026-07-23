# Cross-Harness Test Framework Design

## 1. Overview

Current tests are sparse (21 `#[cfg(test)]` modules, only 2 integration tests exercising real bit7z adapter) and rely on dynamically created temp zip files with no ground truth comparison. This spec proposes a reusable cross-harness test framework.

The core idea: define ground truth using existing VFS types (`Tree` + `VfsMetadata`), create test archives from that truth using both bit7z and 7z/rar CLI, then verify that both harnesses produce `SessionState` matching the ground truth — and matching each other.

### Goals

- Provide a `TestHarness` trait as a programmatic interface to archive operations
- Two implementations: `Bit7zHarness` (wraps usecase layer) and `CliReferee` (wraps system 7z/rar)
- Define ground truth using existing VFS types (`Tree`, `VfsNode`, `VfsMetadata`, `SessionState`) — zero new domain types
- Verify entry metadata (CRC, size, timestamps, etc.) and extracted content via CRC against known ground truth
- Cover all core operations: list (open → read entries → `SessionState`), extract, test
- Cover all supported formats: zip, 7z, tar, tar.gz, tar.bz2, rar
- Cross-validate: bit7z vs ground truth, CLI vs ground truth, bit7z vs CLI

### Non-Goals

- No changes to domain types (no `content_hash` field added)
- No change to usecase signatures, runtime internals, or domain types
- Minor additions to `ArchiveService` (one new accessor method) are acceptable
- No CLI argument parsing tests
- No GUI/presentation layer tests

## 2. Architecture

### New crate: `crates/testing/`

```
crates/testing/
├── Cargo.toml
├── tests/
│   ├── test_list.rs
│   ├── test_extract.rs
│   ├── test_test.rs
│   └── fixtures/
│       ├── mod.rs
│       ├── simple.rs
│       ├── nested.rs
│       ├── encrypted.rs
│       └── edge_cases.rs
└── src/
    ├── lib.rs
    ├── harness.rs          # TestHarness trait
    ├── bit7z_harness.rs    # impl via usecase layer
    ├── cli_referee.rs      # impl via std::process::Command
    ├── fixture.rs          # ground truth builder + archive creation
    └── assert.rs           # comparison logic
```

### Module dependencies

```
┌────────────────────────────────────────────────────────┐
│                   tests/                                │
│  for each fixture: create archive via two paths,        │
│  open with both harnesses, compare vs ground truth      │
└──┬──────────────────────┬──────────────────────────┬───┘
   │ uses                 │ uses                     │ uses
   ▼                      ▼                          ▼
┌──────────┐    ┌────────────────┐    ┌──────────────────────┐
│ fixture  │    │ bit7z_harness  │    │    cli_referee       │
│ (ground  │    │ (-> usecases)  │    │ (-> 7z/rar process)  │
│  truth)  │    └───────┬────────┘    └──────────┬───────────┘
└──────────┘            │                        │
                        ▼                        ▼
              ┌────────────────────────────────────┐
              │          domain types               │
              │  SessionState, Tree, VfsNode,       │
              │  VfsMetadata, ArchiveEntry, CRC     │
              └────────────────────────────────────┘
```

### Key: Domain types ARE the comparison interface

No new types are created for testing. Ground truth is a `SessionState`:

```rust
// domain::vfs already has:
pub struct SessionState {
    pub session: ArchiveSession,
    pub vfs: OverlayVfs,
    pub dirty_tree: DirtyTree,
    pub edit_queue: EditQueue,
    pub metadata_cache: HashMap<VfsNodeId, VfsMetadata>,
}
```

`VfsNode` carries the tree structure, `VfsMetadata` carries all metadata (size, CRC, timestamps, etc.).

## 3. TestHarness trait

```rust
// crates/testing/src/harness.rs

pub struct TestIntegrity {
    pub passed: bool,
    pub failures: Vec<String>,
}

pub trait TestHarness: Send + Sync {
    /// Open an archive and return the full session state (VFS tree + metadata).
    /// This is equivalent to usecase: open → read_entries → build VFS.
    fn open_archive(&self, path: &Path, password: Option<&str>)
        -> Result<SessionState>;

    /// Extract all entries to dest. Verify extracted files against ground truth
    /// by comparing CRC and file tree structure.
    fn extract_all(&self, path: &Path, dest: &Path, password: Option<&str>)
        -> Result<()>;

    /// Run integrity test on archive.
    fn test_archive(&self, path: &Path, password: Option<&str>)
        -> Result<TestIntegrity>;
}
```

### 3.1 Bit7zHarness

`Bit7zHarness` needs access to the `SessionState` after opening. The usecases only return `ArchiveHandle` — they don't expose the VFS. So `Bit7zHarness` holds the runtime directly and gets state from it.

Requirement: `ArchiveService` needs a new public method:

```rust
pub fn get_session_state(&self, archive: &ArchiveHandle) -> Result<SessionState>
```

(This is a thin wrapper around the existing `lookup_session` + `session_manager.get`.)

```rust
// crates/testing/src/bit7z_harness.rs

pub struct Bit7zHarness {
    service: Arc<ArchiveService>,
}

impl TestHarness for Bit7zHarness {
    fn open_archive(&self, path, password) -> Result<SessionState> {
        // 1. OpenArchiveUseCase::execute(path, password) → handle
        // 2. ArchiveService::get_session_state(&handle) → SessionState
        // 3. Return SessionState (contains Tree + metadata_cache)
    }

    fn extract_all(&self, path, dest, password) -> Result<()> {
        // 1. OpenArchiveUseCase::execute
        // 2. ExtractEntriesUseCase::execute(archive, &all_indices, dest, options)
    }

    fn test_archive(&self, path, password) -> Result<TestIntegrity> {
        // 1. Open → test via ArchiveService
    }
}
```

### 3.2 CliReferee

```rust
// crates/testing/src/cli_referee.rs

pub struct CliReferee {
    pub seven_zip_path: PathBuf,   // "7z" or "/usr/bin/7z"
    pub rar_path: Option<PathBuf>, // "rar" or "/usr/bin/rar"
}

impl TestHarness for CliReferee {
    fn open_archive(&self, path, password) -> Result<SessionState> {
        // 1. Run `7z l -slt archive.7z` (or equivalent)
        // 2. Parse output lines into VfsNode + VfsMetadata
        // 3. Build Tree + metadata_cache → SessionState
    }

    fn extract_all(&self, path, dest, password) -> Result<()> {
        // 1. Run `7z x archive.7z -odest -y`
    }

    fn test_archive(&self, path, password) -> Result<TestIntegrity> {
        // 1. Run `7z t archive.7z`
        // 2. Parse pass/fail from exit code and output
    }
}
```

#### CLI output parsing

The `7z l -slt` (list with technical info) format:

```
Path = dir/file.txt
Size = 1234
Compressed = 567
CRC = A1B2C3D4
Encrypted = -
Method = Store
Attributes = A -rw-rw-rw-
...
```

Parsing is per-entry, building a `VfsNode` for the tree position and a `VfsMetadata` for all fields. The parser handles:
- `7z l -slt` (7z/zip/tar/gz)
- `rar v -t` (rar format)
- Directory vs file detection
- Missing/inapplicable fields (directories have Size = 0, Compressed = 0)

## 4. Fixture ground truth

```rust
// crates/testing/src/fixture.rs

/// A known-good archive produced from a known file tree.
pub struct ArchiveFixture {
    pub description: &'static str,
    pub format: ArchiveFormat,
    pub password: Option<&'static str>,

    /// Ground truth: the expected VFS state after opening this archive.
    pub expected_state: SessionState,

    /// Path to the archive file, created on disk during fixture setup.
    pub archive_path: PathBuf,
}

impl ArchiveFixture {
    /// Build ground truth SessionState from a list of (path, content, metadata) specs.
    /// Then create real files in a temp dir, compress to archive.
    pub fn build(
        description: &'static str,
        format: ArchiveFormat,
        password: Option<&'static str>,
        entries: &[FileSpec],
    ) -> Result<Self>;

    /// Build the archive using bit7z (primary) and also via CLI (for cross-check).
    pub fn build_with_bit7z(..) -> Result<Self>;
    pub fn build_with_cli(..) -> Result<Self>;
}

pub struct FileSpec {
    pub path: &'static str,       // "dir/file.txt"
    pub content: &'static [u8],   // known bytes → known CRC
    pub is_directory: bool,
    pub modified: Option<DateTime<Utc>>,
    pub attributes: Option<u32>,
}
```

`FileSpec` is converted into `SessionState` by:
1. Assigning `VfsNodeId`s to each path component
2. Creating `VfsNode`s for each directory/file
3. Computing CRC from `content` (we know it at build time)
4. Building `metadata_cache` from the computed CRC, size, etc.
5. Building the archive from the materialized file tree

## 5. Assertion logic

```rust
// crates/testing/src/assert.rs

/// Compare two SessionState for structural + metadata equality.
/// - Tree structure: every VfsNodeId maps to same path
/// - Per node: VfsMetadata fields (including CRC) equal
pub fn assert_session_state_eq(expected: &SessionState, actual: &SessionState);

/// Verify an extracted directory matches the ground truth file tree:
/// - Same directory structure
/// - Each file's bytes match expected content (verified via CRC or full read)
pub fn assert_extraction_matches(ground_truth: &SessionState, extracted_root: &Path);

/// Run both harnesses on the same archive and assert they agree.
pub fn assert_harnesses_agree(harness_a: &dyn TestHarness, harness_b: &dyn TestHarness, archive: &Path);
```

### Comparison strategy

| Field | How compared |
|-------|-------------|
| Tree structure | Walk `Tree.root_children()`, verify path_of matches at each node |
| Entry count | `metadata_cache.len()` |
| size | exact |
| compressed_size | exact (bit7z vs CLI may differ slightly — allow margin) |
| CRC | exact — primary content integrity check |
| modified/created/accessed | exact or within tolerance |
| is_encrypted | exact |
| is_symlink | exact |
| attributes / posix_attrib | exact |
| host_os | exact (may differ between backends — note in cross-ref) |
| compression_method | exact |

Differences that are expected between bit7z and CLI (e.g., `compression_method` naming, `host_os` values) are noted per-format in the assertion logic with `if format == X: skip field Y`.

## 6. Test structure

```rust
// tests/test_list.rs
mod fixtures;

#[test]
fn test_list_simple_zip() {
    let fixture = fixtures::simple::new(ArchiveFormat::Zip)?;
    test_list_harness(fixture)?;
}

#[test]
fn test_list_nested_7z() {
    let fixture = fixtures::nested::new(ArchiveFormat::SevenZip)?;
    test_list_harness(fixture)?;
}

fn test_list_harness(fixture: ArchiveFixture) -> Result<()> {
    let bit7z = Bit7zHarness::new(build_service());
    let cli = CliReferee::new();

    let bit7z_state = bit7z.open_archive(&fixture.archive_path, fixture.password)?;
    let cli_state = cli.open_archive(&fixture.archive_path, fixture.password)?;

    // Both must match ground truth
    assert_session_state_eq(&fixture.expected_state, &bit7z_state);
    assert_session_state_eq(&fixture.expected_state, &cli_state);

    // Cross-check
    assert_session_state_eq(&bit7z_state, &cli_state);

    Ok(())
}
```

### Fixture catalog

| Fixture | Description | Formats |
|---------|-------------|---------|
| `simple` | Single file `hello.txt` with known content | zip, 7z, tar, tar.gz |
| `nested` | `dir/a.txt`, `dir/sub/b.bin`, `root.txt` | zip, 7z, tar.gz |
| `encrypted` | Single encrypted file with known password | zip (ZipCrypto), 7z (AES-256) |
| `empty_file` | One empty file | zip, 7z |
| `empty_dir` | One empty directory | zip, 7z |
| `long_path` | Deep nesting: `a/b/c/d/e/f/g/h/i/j/file.txt` | zip, 7z |
| `large_file` | File > 1MB with deterministic content (repeat pattern for CRC) | zip, 7z |
| `special_names` | Files with spaces, unicode characters | zip, 7z |
| `multifile_rar` | Multiple files in RAR format | rar |
| `symlinks` | (platform-dependent) Archive with symlinks | 7z |

## 7. Implementation plan

### Phase 1: Domain prep

1. Add `PartialEq` derives to `VfsNode`, `VfsMetadata`, `SessionState` in domain (or implement custom comparison)
2. Ensure `OverlayVfs` exposes a method to produce its underlying `Tree` for comparison
3. Expose `Tree::path_of()` publicly (already is)

### Phase 2: testing crate skeleton

1. Create `crates/testing/` with `Cargo.toml`, `src/lib.rs`
2. Implement `harness.rs`: `TestHarness` trait + types
3. Implement `assert.rs`: comparison logic
4. Implement `fixture.rs`: `FileSpec`, `ArchiveFixture`

### Phase 3: Harness implementations

1. `Bit7zHarness` — wraps `ArchiveService` (requires `application/archive` dep)
2. `CliReferee` — wraps `std::process::Command`, parses `7z l -slt` output

### Phase 4: Fixture definitions + integration tests

1. `tests/fixtures/simple.rs` — first fixture
2. `tests/test_list.rs` — first test
3. Iterate over fixture catalog

### Phase 5: Extract + Test coverage

1. `tests/test_extract.rs`
2. `tests/test_test.rs`

## 8. Required changes to existing code

### Domain (`crates/domain/src/vfs/`)

- Derive `PartialEq` on `VfsNode`, `VfsMetadata`, `SessionState` (no new fields)
- `Tree` comparison needs a method to compare two trees structurally (paths match, children match)

### Bit7z adapter (`crates/infrastructure/bit7z/`)

No changes needed — the adapter already produces all metadata fields.

### Application archive (`crates/application/archive/`)

- `OpenArchiveUseCase` output needs to expose the `SessionState` (check if `ArchiveService` already returns it)
- May need a public method on `ArchiveService` to get `SessionState` for a given handle

## Appendix A: 7z CLI output reference

`7z l -slt archive.zip` output:

```
Listing archive: archive.zip

--
Path = hello.txt
Size = 12
Compressed = 14
Packed Size = 14
Modified = 2024-01-15 10:30:00
Created = 2024-01-15 10:30:00
Accessed = 2024-01-15 10:30:00
Attributes = A -rw-rw-rw-
CRC = A1B2C3D4
Encrypted = -
Method = Store
Solid = -
Block = 0
```

`7z t archive.zip` exit:
- Exit 0: all tests passed
- Exit 1: warning (non-fatal)
- Exit 2: fatal error / test failed
- Exit 7: command line error
- Exit 8: not enough memory
- Exit 255: user stopped process
