# Command + Executor Refactoring (Phase 1)

## Problem

The presentation layer (GPUI views/dialogs) contains ~250+ lines of business logic that should belong in the application layer:

1. **Extraction orchestration** duplicated 3× in `root.rs` — channel setup, VFS traversal, cancel/pause atomics, progress polling
2. **Create/Add-files dialog orchestration** in `create.rs` / `add_files.rs` — progress management, error formatting
3. **VFS directory traversal** duplicated in `root.rs`, `test/mod.rs`, and `TestEntriesUseCase`
4. **Runtime progress/cancel not wired** — `executor_impl.rs` creates fresh `NoopNotifier` and `AtomicBool`, ignoring UI-provided options
5. **`OperationKind::Extract` drops overwrite_mode** — hardcoded to `Overwrite` in executor

## Solution: Command + Executor Pattern

### New modules in `crates/application/archive/src/`

#### `commands/` — Pure-data command structs

Commands carry user intent with zero behavior. Each maps 1:1 to a runtime `OperationKind`.

```rust
pub struct ExtractArchive {
    pub handle: ArchiveHandle,
    pub entry_indices: Vec<u32>,
    pub destination: PathBuf,
    pub overwrite_mode: OverwriteMode,
}

pub struct CreateArchive {
    pub path: PathBuf,
    pub format: ArchiveFormat,
    pub encryption: Option<EncryptionConfig>,
    pub files: Vec<FileToAdd>,
}

pub struct AddFilesToArchive {
    pub handle: ArchiveHandle,
    pub files: Vec<FileToAdd>,
    pub password: Option<Password>,
    pub encrypt_filenames: bool,
}
```

#### `token.rs` — OperationToken

Lightweight wrapper around the existing `OperationHandle`:

```rust
pub struct OperationToken {
    pub(crate) handle: OperationHandle,
    pub(crate) runtime: Arc<Runtime>,
}

impl OperationToken {
    pub fn cancel(&self) -> Result<(), RuntimeError>;
    pub fn state(&self) -> Option<OperationState>;
    pub fn subscribe(&self) -> OperationEventReceiver;
    pub fn wait(self) -> Option<JobResult>;
}
```

#### `executor.rs` — CommandExecutor

Encapsulates the full lifecycle of each operation:

```rust
pub struct CommandExecutor {
    service: Arc<ArchiveService>,
    runtime: Arc<Runtime>,
    resolver: Arc<dyn CapabilityResolver<ArchiveCapMeta>>,
}

impl CommandExecutor {
    pub fn execute_extract(&self, cmd: ExtractArchive) -> Result<OperationToken, ArchiveError>;
    pub fn execute_create(&self, cmd: CreateArchive) -> Result<OperationToken, ArchiveError>;
    pub fn execute_add_files(&self, cmd: AddFilesToArchive) -> Result<OperationToken, ArchiveError>;
}
```

Each `execute_*` method:
1. Expands directory entries via shared VFS traversal
2. Resolves capability via ArchiveService
3. Submits job to Runtime
4. Returns `OperationToken` (UI polls/subscribes/cancels via this)

#### `vfs_util.rs` — Shared directory traversal

Single function replacing 3 duplicate implementations:

```rust
pub fn expand_directory_entries(
    session_manager: &dyn SessionManager,
    handle: &ArchiveHandle,
    indices: &[u32],
) -> Result<Vec<u32>, ArchiveError>;
```

### Changes to `crates/runtime/`

#### `job.rs` — `OperationKind::Extract` gains `overwrite_mode`

```rust
pub enum OperationKind {
    Extract {
        session_id: SessionId,
        indices: Vec<u32>,
        destination: PathBuf,
        overwrite_mode: OverwriteMode,   // NEW
    },
    // ... other variants unchanged
}

// JobKind gets the same field
```

#### `executor_impl.rs` — Fix progress/cancel/overwrite

Before: fresh `Arc<AtomicBool>` + `NoopNotifier` + `OverwriteMode::Overwrite`.
After: wire through `ctx.cancellation` + `ctx.progress` + job's `overwrite_mode`.

```rust
async fn extract(ctx: ExecutionContext, job: &Job) -> JobResult {
    let options = ExtractOptions {
        overwrite_mode: job.overwrite_mode,
        cancel: ctx.cancellation.as_atomic(),
        paused: Arc::new(AtomicBool::new(false)),
        notifier: Box::new(ProgressBridge(ctx.progress.clone())),
    };
    // ... call reader.extract(&session, &indices, &dest, &options)
}
```

#### `cancel.rs` — Expose inner atomic

```rust
impl CancellationToken {
    pub fn as_atomic(&self) -> Arc<AtomicBool> {
        self.cancelled.clone()
    }
}
```

#### `job.rs` — ProgressBridge adapter

```rust
struct ProgressBridge(Arc<dyn ProgressReporter>);

impl ProgressNotifier for ProgressBridge {
    fn notify(&self, update: ProgressUpdate) {
        self.0.report(update);
    }
}
```

### Changes to `crates/application/archive/`

#### `runtime_service.rs` — Expose capability resolution

```rust
impl ArchiveService {
    pub fn resolve_capability(
        &self,
        kind: ArchiveOpKind,
        format: ArchiveFormat,
        session_id: Option<u64>,
    ) -> Result<ExecutionDescriptor, ArchiveError> {
        self.resolve(kind, format, session_id)
    }
}
```

### Changes to `crates/presentation/`

| File | Action | Lines |
|---|---|---|
| `views/src/root.rs` | Remove `expand_indices`, `collect_directory`, `extract_with_progress` | ~200 |
| `dialogs/src/test/mod.rs` | Remove `count_files_recursive`, `count_expanded` | ~40 |
| `views/src/root.rs` | Replace 3× extraction handlers with `executor.execute_extract(cmd)` + `token.subscribe()` | ~105 |

UI code changes from:

```rust
// Before: ~60 lines of channels, atomics, poll, error formatting
let (tx, rx) = crossbeam::bounded(32);
let cancel = Arc::new(AtomicBool::new(false));
let indices = expand_indices(...);
let options = ExtractOptions { overwrite_mode, cancel, paused, notifier };
service.extract(handle, &indices, dest, &options)?;
// ... poll rx in a spawned task ...
```

To:

```rust
// After: ~5 lines
let cmd = ExtractArchive { handle, entry_indices, destination, overwrite_mode };
let token = executor.execute_extract(cmd)?;
let progress_rx = token.subscribe();
// progress_rx drives UI spinner/progress bar
```

### Dependency changes

`crates/application/archive/Cargo.toml` — no new external dependencies. Uses existing `bit7z-runtime`, `bit7z-capability`, `bit7z-ports`.

## Data Flow Diagram

```
Presentation                     Application                  Runtime
────────────                     ───────────                  ───────
constructs Command               CommandExecutor              JobManager
  → extract cmd ──────────────►  expand_indices()             dispatch()
                                  resolve_capability()          → scheduler
                                  submit(OperationRequest)───►  → acquire
                                  return OperationToken          → execute()

                                OperationToken                LocalExecutor
get token ◄───────────────────  wraps OperationHandle           ↓
  subscribe() ──────────────►   OperationEventReceiver         extract()
  cancel()   ──────────────►   Runtime::cancel()                 → reader.extract()
  wait()     ──────────────►   Runtime::wait()                   ✓ overwrite_mode
                                                                 ✓ ctx.cancellation
                                                                 ✓ ctx.progress
```

## Error Handling

- `CommandExecutor` methods return `Result<OperationToken, ArchiveError>` for pre-flight failures (capability resolution, VFS lookup)
- Runtime failures surface through `JobResult::Failed(msg)` → mapped by `OperationToken::wait()` or via `OperationEvent::Completed`
- UI subscribes to events for progress; `Canceled` is a terminal event

## Future Phases

- Phase 2: Extract encryption config builder, format capability queries, SettingsStore persistence
- Phase 3: Compression ratio dedup, `AppError` type, domain type indirection
