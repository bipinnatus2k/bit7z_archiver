# Phase 1 — Command + Executor Refactoring Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move ~250 lines of operation orchestration from presentation layer into application layer Command + Executor pattern, fixing the runtime progress/cancel/overwrite-mode plumbing gap.

**Architecture:** Commands (pure data) → CommandExecutor (orchestrates via existing capability resolver + runtime) → fixed ExecutorImpl (wires progress/cancel/overwrite through). Extract operations return `OperationToken` for cancel/subscribe/wait; create/add_files remain synchronous.

**Tech Stack:** Rust, GPUI, existing bit7z-capability + bit7z-runtime + bit7z-app-archive crates

## Global Constraints

- `cargo check` must produce zero new errors after each task
- `cargo test` must pass after each task (all existing tests + any new ones)
- Follow existing naming: `snake_case`, `CamelCase`, `SCREAMING_SNAKE_CASE`
- Every `unsafe` block requires `// SAFETY:` comment
- No new external dependencies

---

### Task 1: VfsUtil — Shared Directory Traversal

**Files:**
- Create: `crates/application/archive/src/vfs_util.rs` (full file)
- Modify: `crates/application/archive/src/lib.rs` (add `pub mod vfs_util;`)

**Interfaces:**
- Consumes: `OverlayVfs` from `bit7z_domain::vfs`, `SessionManager` from `bit7z_runtime::session`
- Produces: `fn expand_directory_entries(session_manager: &dyn SessionManager, session_id: SessionId, indices: &[u32]) -> Result<Vec<u32>, ArchiveError>`

- [ ] **Step 1: Write the failing test in a `#[cfg(test)]` block in vfs_util.rs**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use bit7z_domain::archive::{ArchiveEntry, ArchiveSession, ArchiveFormat, SessionId};
    use bit7z_domain::vfs::{OverlayVfs, VfsMetadata};
    use bit7z_ports::session::{SessionRef, SessionStore};
    use std::sync::{Arc, Mutex};
    use std::path::PathBuf;

    struct MockSessionStore {
        state: Mutex<Option<bit7z_domain::vfs::SessionState>>,
    }

    impl SessionStore for MockSessionStore {
        fn insert(&self, _state: bit7z_domain::vfs::SessionState) {}
        fn get(&self, _id: SessionId) -> Option<SessionRef> { self.state.lock().unwrap().clone().map(Arc::new) }
        fn remove(&self, _id: SessionId) -> Option<bit7z_domain::vfs::SessionState> { None }
        fn contains(&self, _id: SessionId) -> bool { true }
        fn all(&self) -> Vec<SessionId> { vec![] }
    }

    #[test]
    fn test_expand_directory_entries_flattens_dirs() {
        let dir = ArchiveEntry {
            name: "dir".into(),
            path: "dir".into(),
            original_index: 0,
            is_dir: true,
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
            state: Mutex::new(Some(bit7z_domain::vfs::SessionState {
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
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p bit7z-app-archive vfs_util -- --nocapture`
Expected: FAIL with "module `vfs_util` not found" or "function not defined"

- [ ] **Step 3: Write minimal implementation**

```rust
// vfs_util.rs
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
            Some(e) if e.is_dir => collect_descendants(vfs, idx, &mut expanded),
            _ => expanded.push(idx),
        }
    }
    expanded.sort();
    expanded.dedup();
    Ok(expanded)
}

fn collect_descendants(vfs: &OverlayVfs, dir_idx: u32, out: &mut Vec<u32>) {
    let page = vfs.list_page(0, usize::MAX);
    let children: Vec<_> = page
        .items
        .iter()
        .filter(|e| {
            e.path.starts_with(
                page.items
                    .iter()
                    .find(|p| p.original_index == dir_idx)
                    .map(|p| &*p.path)
                    .unwrap_or(""),
            ) && e.original_index != dir_idx
        })
        .cloned()
        .collect();

    for child in &children {
        if child.is_dir {
            collect_descendants(vfs, child.original_index, out);
        } else {
            out.push(child.original_index);
        }
    }
}
```

- [ ] **Step 4: Add `pub mod vfs_util;` to `crates/application/archive/src/lib.rs`**

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test -p bit7z-app-archive vfs_util -- --nocapture`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add crates/application/archive/src/vfs_util.rs crates/application/archive/src/lib.rs
git commit -m "feat: add shared expand_directory_entries in vfs_util.rs"
```

---

### Task 2: CancellationToken — Expose Inner AtomicBool

**Files:**
- Modify: `crates/runtime/src/cancel.rs` (add `pub fn as_atomic(&self) -> Arc<AtomicBool>`)

- [ ] **Step 1: Add method after `is_cancelled()`**

```rust
pub fn as_atomic(&self) -> Arc<AtomicBool> {
    self.cancelled.clone()
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check -p bit7z-runtime`
Expected: zero errors

- [ ] **Step 3: Commit**

```bash
git add crates/runtime/src/cancel.rs
git commit -m "feat: expose CancellationToken inner AtomicBool via as_atomic()"
```

---

### Task 3: ProgressBridge — Adapter from ProgressReporter to ProgressNotifier

**Files:**
- Create: `crates/runtime/src/progress_bridge.rs`
- Modify: `crates/runtime/src/lib.rs` (add `mod progress_bridge; pub use progress_bridge::ProgressBridge;`)

- [ ] **Step 1: Write the test first**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use bit7z_domain::repository::{ProgressNotifier, ProgressUpdate};
    use bit7z_ports::progress::ProgressReporter;
    use std::sync::{Arc, Mutex};

    struct CaptureReporter {
        updates: Mutex<Vec<ProgressUpdate>>,
    }

    impl ProgressReporter for CaptureReporter {
        fn report(&self, update: ProgressUpdate) {
            self.updates.lock().unwrap().push(update);
        }
    }

    #[test]
    fn test_progress_bridge_forwards_update() {
        let reporter = Arc::new(CaptureReporter { updates: Mutex::new(vec![]) });
        let bridge = ProgressBridge(reporter.clone());
        let update = ProgressUpdate {
            bytes_done: 50,
            bytes_total: 100,
            ..Default::default()
        };
        bridge.notify(&update);
        assert_eq!(reporter.updates.lock().unwrap().len(), 1);
        assert_eq!(reporter.updates.lock().unwrap()[0].bytes_done, 50);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p bit7z-runtime progress_bridge -- --nocapture`
Expected: FAIL with "module `progress_bridge` not found"

- [ ] **Step 3: Write minimal implementation**

```rust
use std::sync::Arc;
use bit7z_domain::repository::{ProgressNotifier, ProgressUpdate};
use bit7z_ports::progress::ProgressReporter;

/// Bridge that forwards ProgressNotifier::notify() calls to a ProgressReporter.
pub struct ProgressBridge(pub Arc<dyn ProgressReporter>);

impl ProgressNotifier for ProgressBridge {
    fn notify(&self, update: &ProgressUpdate) {
        self.0.report(update.clone());
    }
}
```

- [ ] **Step 4: Register module in lib.rs, add `mod progress_bridge;` and `pub use progress_bridge::ProgressBridge;`**

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test -p bit7z-runtime progress_bridge -- --nocapture`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add crates/runtime/src/progress_bridge.rs crates/runtime/src/lib.rs
git commit -m "feat: add ProgressBridge adapter from ProgressReporter to ProgressNotifier"
```

---

### Task 4: OperationKind/JobKind — Add `overwrite_mode` to Extract

**Files:**
- Modify: `crates/runtime/src/job.rs` (add `overwrite_mode` to both enums)

- [ ] **Step 1: Update `OperationKind::Extract`**

```rust
Extract {
    session_id: SessionId,
    indices: Vec<u32>,
    destination: PathBuf,
    overwrite_mode: bit7z_domain::archive::OverwriteMode,
},
```

- [ ] **Step 2: Update `JobKind::Extract`**

```rust
Extract {
    session_id: SessionId,
    indices: Vec<u32>,
    destination: PathBuf,
    overwrite_mode: bit7z_domain::archive::OverwriteMode,
},
```

- [ ] **Step 3: Verify compilation fails (expected — manager.rs and executor_impl.rs need updating)**

Run: `cargo check -p bit7z-runtime`
Expected: compile errors in manager.rs and executor_impl.rs

- [ ] **Step 4: Commit transitional change**

```bash
git add crates/runtime/src/job.rs
git commit -m "feat: add overwrite_mode field to OperationKind::Extract and JobKind::Extract"
```

---

### Task 5: Update operation_request_to_job and executor dispatch

**Files:**
- Modify: `crates/runtime/src/manager.rs` (lines 94-102 — add `overwrite_mode` to mapping)
- Modify: `crates/runtime/src/executor_impl.rs` (lines 33-37 — pass `overwrite_mode` to extract call)

- [ ] **Step 1: Update `manager.rs` mapping**

Replace the `OperationKind::Extract` arm in `operation_request_to_job`:

```rust
crate::job::OperationKind::Extract {
    session_id,
    indices,
    destination,
    overwrite_mode,
} => JobKind::Extract {
    session_id,
    indices,
    destination,
    overwrite_mode,
},
```

- [ ] **Step 2: Update `executor_impl.rs` match arm in `run_job`**

Replace the `JobKind::Extract` arm:

```rust
JobKind::Extract {
    session_id,
    indices,
    destination,
    overwrite_mode,
} => extract(ctx, session_id, indices, destination, overwrite_mode).await,
```

- [ ] **Step 3: Verify compilation fails (expected — `extract()` function signature needs updating)**

Run: `cargo check -p bit7z-runtime`
Expected: error about `extract()` function taking wrong number of parameters

- [ ] **Step 4: Commit**

```bash
git add crates/runtime/src/manager.rs crates/runtime/src/executor_impl.rs
git commit -m "refactor: plumb overwrite_mode through manager mapping and executor dispatch"
```

---

### Task 6: Fix executor_impl.rs extract() — Wire overwrite_mode, cancel, and progress

**Files:**
- Modify: `crates/runtime/src/executor_impl.rs` (rewrite extract() function)

**Prerequisite:** Tasks 2-3 (CancellationToken::as_atomic, ProgressBridge)

- [ ] **Step 1: Replace the existing `extract()` function**

Replace the entire function with:

```rust
async fn extract(
    ctx: ExecutionContext,
    session_id: bit7z_domain::archive::SessionId,
    indices: Vec<u32>,
    destination: std::path::PathBuf,
    overwrite_mode: bit7z_domain::archive::OverwriteMode,
) -> JobResult {
    let state = match ctx.ports.session_store.get(session_id) {
        Some(s) => s,
        None => {
            return JobResult::Failed(format!("session {} not found", session_id));
        }
    };

    let options = bit7z_domain::repository::ExtractOptions {
        overwrite_mode,
        cancel: ctx.cancellation.as_atomic(),
        paused: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        notifier: Arc::new(crate::ProgressBridge(ctx.progress)),
    };

    match ctx
        .ports
        .reader
        .extract(&state.session, &indices, &destination, &options)
    {
        Ok(()) => JobResult::Ok,
        Err(e) => JobResult::Failed(e.to_string()),
    }
}
```

Also ensure `use std::sync::Arc;` and `use std::sync::atomic::AtomicBool;` are at the top of the file.

- [ ] **Step 2: Verify compilation and tests**

Run: `cargo check -p bit7z-runtime && cargo test -p bit7z-runtime`
Expected: 0 errors, all 8 tests pass

- [ ] **Step 3: Commit**

```bash
git add crates/runtime/src/executor_impl.rs
git commit -m "fix: wire overwrite_mode, ctx.cancellation, and ctx.progress through extract()"
```

---

### Task 7: ArchiveService — Expose resolve_capability and lookup_session

**Files:**
- Modify: `crates/application/archive/src/runtime_service.rs`

- [ ] **Step 1: Make `lookup_session` accessible to CommandExecutor (same crate)**

Change from `fn lookup_session(` to `pub(crate) fn lookup_session(`

- [ ] **Step 2: Add `pub fn resolve_capability`**

```rust
pub fn resolve_capability(
    &self,
    kind: ArchiveOpKind,
    format: ArchiveFormat,
    session_id: Option<u64>,
) -> Result<ExecutionDescriptor, ArchiveError> {
    self.resolve(kind, format, session_id)
}
```

- [ ] **Step 3: Verify compilation and tests**

Run: `cargo check -p bit7z-app-archive && cargo test -p bit7z-app-archive`
Expected: 0 errors, all 9 tests pass

- [ ] **Step 4: Commit**

```bash
git add crates/application/archive/src/runtime_service.rs
git commit -m "feat: expose ArchiveService::resolve_capability() and pub(crate) lookup_session for CommandExecutor"
```

---

### Task 8: Commands Module — Pure Data Structs

**Files:**
- Create: `crates/application/archive/src/commands/mod.rs`
- Create: `crates/application/archive/src/commands/extract.rs`
- Create: `crates/application/archive/src/commands/create.rs`
- Create: `crates/application/archive/src/commands/add_files.rs`
- Modify: `crates/application/archive/src/lib.rs` (add `pub mod commands;`)

- [ ] **Step 1: Write the test in `commands/mod.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_extract_command_fields() {
        let cmd = ExtractArchive {
            handle: ArchiveHandle::new_reader().with_path(PathBuf::from("a.zip")),
            entry_indices: vec![0, 1],
            destination: PathBuf::from("/out"),
            overwrite_mode: OverwriteMode::Overwrite,
        };
        assert_eq!(cmd.entry_indices.len(), 2);
    }

    #[test]
    fn test_create_command_fields() {
        let cmd = CreateArchive {
            path: PathBuf::from("new.7z"),
            format: ArchiveFormat::SevenZip,
            encryption: None,
        };
        assert_eq!(cmd.format, ArchiveFormat::SevenZip);
    }

    #[test]
    fn test_add_files_command_fields() {
        let cmd = AddFilesToArchive {
            handle: ArchiveHandle::new_reader().with_path(PathBuf::from("a.zip")),
            files: vec![PathBuf::from("f.txt")],
        };
        assert_eq!(cmd.files.len(), 1);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p bit7z-app-archive commands -- --nocapture`
Expected: FAIL with "module `commands` not found"

- [ ] **Step 3: Write command structs**

```rust
// commands/extract.rs
use bit7z_domain::archive::{ArchiveHandle, OverwriteMode};
use std::path::PathBuf;

pub struct ExtractArchive {
    pub handle: ArchiveHandle,
    pub entry_indices: Vec<u32>,
    pub destination: PathBuf,
    pub overwrite_mode: OverwriteMode,
}
```

```rust
// commands/create.rs
use bit7z_domain::archive::{ArchiveFormat, EncryptionConfig};
use std::path::PathBuf;

pub struct CreateArchive {
    pub path: PathBuf,
    pub format: ArchiveFormat,
    pub encryption: Option<EncryptionConfig>,
}
```

```rust
// commands/add_files.rs
use bit7z_domain::archive::ArchiveHandle;
use std::path::PathBuf;

pub struct AddFilesToArchive {
    pub handle: ArchiveHandle,
    pub files: Vec<PathBuf>,
}
```

```rust
// commands/mod.rs
mod extract;
mod create;
mod add_files;

pub use extract::ExtractArchive;
pub use create::CreateArchive;
pub use add_files::AddFilesToArchive;
```

- [ ] **Step 4: Register module in lib.rs** — add `pub mod commands;`

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test -p bit7z-app-archive commands -- --nocapture`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add crates/application/archive/src/commands/ crates/application/archive/src/lib.rs
git commit -m "feat: add Commands module with ExtractArchive, CreateArchive, AddFilesToArchive"
```

---

### Task 9: OperationToken — Wrapper around OperationHandle

**Files:**
- Create: `crates/application/archive/src/token.rs`
- Modify: `crates/application/archive/src/lib.rs` (add `pub mod token;`)

- [ ] **Step 1: Write OperationToken**

```rust
// token.rs
use std::sync::Arc;
use bit7z_runtime::{
    JobResult, OperationEventReceiver, OperationHandle, OperationState,
    Runtime, RuntimeError,
};

/// Token returned by CommandExecutor for an async operation.
///
/// The holder can cancel the operation, poll its state, subscribe to
/// progress events, or block until it completes.
pub struct OperationToken {
    pub handle: OperationHandle,
    pub runtime: Arc<Runtime>,
}

impl OperationToken {
    pub fn cancel(&self) -> Result<(), RuntimeError> {
        self.runtime.cancel(self.handle)
    }

    pub fn state(&self) -> Option<OperationState> {
        self.runtime.state(self.handle)
    }

    pub fn subscribe(&self) -> OperationEventReceiver {
        let (tx, rx) = futures::channel::mpsc::unbounded();
        self.runtime.subscribe(tx);
        rx
    }

    pub fn wait(self) -> Option<JobResult> {
        self.runtime.wait(self.handle)
    }
}
```

- [ ] **Step 2: Register module** — add `pub mod token;` to lib.rs

- [ ] **Step 3: Verify compilation**

Run: `cargo check -p bit7z-app-archive`
Expected: 0 errors

- [ ] **Step 4: Commit**

```bash
git add crates/application/archive/src/token.rs crates/application/archive/src/lib.rs
git commit -m "feat: add OperationToken wrapping OperationHandle with cancel/subscribe/wait"
```

---

### Task 10: CommandExecutor — Orchestration Layer

**Files:**
- Create: `crates/application/archive/src/executor.rs`
- Modify: `crates/application/archive/src/lib.rs` (add `pub mod executor;`)

- [ ] **Step 1: Write CommandExecutor**

```rust
// executor.rs
use std::sync::Arc;
use std::path::PathBuf;
use bit7z_capability::CapabilityResolver;
use bit7z_domain::archive::{ArchiveHandle, ArchiveFormat, EncryptionConfig, OverwriteMode};
use bit7z_domain::repository::ArchiveError;
use bit7z_runtime::{OperationKind, OperationRequest, Priority, Runtime};
use crate::capability::{ArchiveCapMeta, ArchiveOpKind};
use crate::commands::{ExtractArchive, CreateArchive, AddFilesToArchive};
use crate::runtime_service::ArchiveService;
use crate::token::OperationToken;
use crate::vfs_util::expand_directory_entries;

pub struct CommandExecutor {
    service: Arc<ArchiveService>,
    runtime: Arc<Runtime>,
}

impl CommandExecutor {
    pub fn new(
        service: Arc<ArchiveService>,
        runtime: Arc<Runtime>,
    ) -> Self {
        Self { service, runtime }
    }

    /// Submit an extraction — returns a token for progress/cancel.
    pub fn execute_extract(
        &self,
        cmd: ExtractArchive,
    ) -> Result<OperationToken, ArchiveError> {
        let session = self.service.lookup_session(&cmd.handle)?;
        let indices = expand_directory_entries(
            self.runtime.session_manager.as_ref(),
            session.id,
            &cmd.entry_indices,
        )?;

        let desc = self.service.resolve_capability(
            ArchiveOpKind::Extract,
            session.format,
            Some(session.id),
        )?;

        let handle = self.runtime.submit(OperationRequest {
            kind: OperationKind::Extract {
                session_id: session.id,
                indices,
                destination: cmd.destination,
                overwrite_mode: cmd.overwrite_mode,
            },
            descriptor: desc,
            priority: Priority::User,
        });
        Ok(OperationToken { handle, runtime: self.runtime.clone() })
    }

    /// Create a new archive (synchronous).
    pub fn execute_create(
        &self,
        cmd: CreateArchive,
    ) -> Result<ArchiveHandle, ArchiveError> {
        self.service.create(&cmd.path, cmd.format, cmd.encryption.as_ref())
    }

    /// Add files to an existing archive (synchronous).
    pub fn execute_add_files(
        &self,
        cmd: AddFilesToArchive,
    ) -> Result<(), ArchiveError> {
        let use_case = crate::modify::ModifyArchiveUseCase::new(self.service.clone());
        use_case.add_files(&cmd.handle, &cmd.files, None)
    }
}
```

- [ ] **Step 2: Register module** — add `pub mod executor;` to lib.rs

- [ ] **Step 3: Verify compilation**

Run: `cargo check -p bit7z-app-archive`
Expected: 0 errors

- [ ] **Step 4: Commit**

```bash
git add crates/application/archive/src/executor.rs crates/application/archive/src/lib.rs
git commit -m "feat: add CommandExecutor with execute_extract/create/add_files"
```

---

### Task 11: Presentation Cleanup — Remove Duplicated Logic

**Files:**
- Modify: `crates/presentation/views/src/root.rs` (remove `expand_indices`, `collect_directory`, `extract_with_progress`; replace 3 extraction handlers)
- Modify: `crates/presentation/dialogs/src/test/mod.rs` (remove `count_files_recursive`, `count_expanded`)

- [ ] **Step 1: Remove `expand_indices` and `collect_directory` from root.rs**

Delete the approximately 40 lines (lines ~905-945). These are replaced by `expand_directory_entries` in vfs_util.rs.

- [ ] **Step 2: Remove `extract_with_progress` function from root.rs**

Delete the approximately 60 lines (lines ~947-1009). The executor now handles progress internally.

- [ ] **Step 3: Remove `count_files_recursive` and `count_expanded` from test dialog**

Delete from `crates/presentation/dialogs/src/test/mod.rs` (approximately lines 237-278).

- [ ] **Step 4: Replace 3 extraction handlers with CommandExecutor calls**

Each handler (`ExtractSelected`, Ctrl+E shortcut, `ExtractArchive` menu action) changes from manual orchestration to:

```rust
let cmd = ExtractArchive {
    handle: archive_state.archive().unwrap().clone(),
    entry_indices: archive_state.selected_indices(),
    destination: dest,
    overwrite_mode: mode.into_overwrite_mode(),
};
let executor = /* get CommandExecutor from app state */;
let token = executor.execute_extract(cmd)?;
let rx = token.subscribe();
cx.spawn(|mut cx| async move {
    while let Some(event) = rx.next().await {
        match event {
            OperationEvent::Progress { percent, .. } => { /* update UI */ }
            OperationEvent::Completed { result, .. } => {
                match result {
                    JobResult::Ok => { /* success notification */ }
                    JobResult::Cancelled => { /* cancelled notification */ }
                    JobResult::Failed(msg) => { /* error notification */ }
                    _ => {}
                }
                break;
            }
            _ => {}
        }
    }
}).detach();
```

Note: The wiring of `CommandExecutor` into app state depends on the GPUI context. The executor should be stored alongside the archive service (e.g., in a GPUI global or passed through the root view's state). The exact integration point depends on how the app currently accesses `ArchiveService`.

- [ ] **Step 5: Verify compilation**

Run: `cargo check`
Expected: 0 errors (pre-existing warnings OK)

- [ ] **Step 6: Run all tests**

Run: `cargo test`
Expected: All tests pass

- [ ] **Step 7: Commit**

```bash
git add crates/presentation/views/src/root.rs crates/presentation/dialogs/src/test/mod.rs
git commit -m "refactor: replace presentation-level orchestration with CommandExecutor calls"
```

---

## Summary of All Files Changed

| Action | Path |
|---|---|
| CREATE | `crates/application/archive/src/vfs_util.rs` |
| CREATE | `crates/runtime/src/progress_bridge.rs` |
| CREATE | `crates/application/archive/src/commands/mod.rs` |
| CREATE | `crates/application/archive/src/commands/extract.rs` |
| CREATE | `crates/application/archive/src/commands/create.rs` |
| CREATE | `crates/application/archive/src/commands/add_files.rs` |
| CREATE | `crates/application/archive/src/token.rs` |
| CREATE | `crates/application/archive/src/executor.rs` |
| MODIFY | `crates/runtime/src/cancel.rs` |
| MODIFY | `crates/runtime/src/job.rs` |
| MODIFY | `crates/runtime/src/manager.rs` |
| MODIFY | `crates/runtime/src/executor_impl.rs` |
| MODIFY | `crates/runtime/src/lib.rs` |
| MODIFY | `crates/application/archive/src/runtime_service.rs` |
| MODIFY | `crates/application/archive/src/lib.rs` |
| MODIFY | `crates/presentation/views/src/root.rs` |
| MODIFY | `crates/presentation/dialogs/src/test/mod.rs` |
