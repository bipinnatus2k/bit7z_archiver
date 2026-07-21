# Hexagonal Backend Architecture: Runtime, Capability & Session-Centric Design

## 1. Overview

This document proposes a backend refactor for the bit7z archiver. The current `ArchiveRepository` trait has grown into a god object that mixes FFI resource management, VFS state, editing transactions, and I/O operations. The goal is to replace this monolithic repository with a layered, hexagonal architecture centered around `ArchiveSession`.

### Goals

- Decouple resource management, execution scheduling, capability discovery, and business logic.
- Make the architecture backend-agnostic so that bit7z can be replaced or supplemented by other backends (e.g., a pure-Rust 7z implementation, libarchive, mock backends).
- Provide a clean runtime layer for background jobs, cancellation, dependency scheduling, and resource arbitration.
- Keep the domain model pure and independent of infrastructure concerns.

### Non-Goals

- This refactor does not change the GPUI presentation layer or the user-facing feature set.
- It does not introduce distributed execution; scheduling remains in-process.
- It does not replace the existing VFS semantics; it restructures how VFS is owned and accessed.

---

## 2. Guiding Principles

1. **ArchiveSession is the core.** All other layers exist to serve the archive editing session.
2. **Domain is pure.** Domain models have no dependency on runtime, infrastructure, or application layers.
3. **Ports are thin abstractions.** A port is an interface that describes a capability, not a concrete operation list.
4. **Application constructs requests, not execution plans.** `OperationRequest` describes intent; the runtime decides how to execute it.
5. **Runtime is a framework.** Job scheduling, resource arbitration, session lifecycle, and execution are runtime concerns.
6. **Capability resolution is a policy layer.** The capability resolver decides whether and how an operation can be executed before it reaches the runtime.

---

## 3. Layered Architecture

```text
Presentation
        │
        ▼
Application (Use Case / Service)
        │
        ▼
Capability
├── CapabilityRegistry
└── CapabilityResolver
        │
        ▼
Runtime
├── JobManager
├── Scheduler
├── Executor
├── SessionManager
├── ResourceManager
└── RuntimeContext
        │
        ▼
Domain
├── ArchiveSession
├── VFS / OverlayVFS
├── DirtyTree
├── EditQueue
├── ChangeSet
├── EditTransaction
└── Entry / Metadata
        │
        ▼
Ports
├── ArchiveReader
├── ArchiveWriter
├── VfsProvider
├── CryptoProvider
├── ProgressReporter
├── FileSystem
├── TempStorage
└── SessionStore
        │
        ▼
Infrastructure
├── Bit7zReaderAdapter
├── Bit7zWriterAdapter
├── InMemoryVfs
├── RealFileSystem
└── RealTempStorage
```

---

## 4. Domain Layer

The domain layer owns `ArchiveSession` and everything inside it. It is a pure, synchronous layer with no async, no FFI, and no I/O.

### 4.1 ArchiveSession

`ArchiveSession` is the aggregate root of the domain. It is a value-like object that can be cloned or referenced, but its authoritative state lives inside the runtime `SessionManager`.

```rust
pub struct ArchiveSession {
    pub id: SessionId,
    pub path: PathBuf,
    pub format: ArchiveFormat,
    pub password: Option<Password>,
}

pub struct SessionState {
    pub session: ArchiveSession,
    pub vfs: OverlayVfs,
    pub dirty_tree: DirtyTree,
    pub edit_queue: EditQueue,
    pub metadata_cache: MetadataCache,
}
```

### 4.2 VFS

The entire VFS subsystem belongs to the Domain layer, not Infrastructure:

- `BaseVfs` / `Tree` — base archive entry tree.
- `OverlayVfs` — working tree overlay on top of the base tree.
- `DirtyTree` — tracks added/deleted/renamed nodes.
- `EditQueue` — undo/redo history.
- `ChangeSet` — diff between base and working tree.

These are pure in-memory data structures and algorithms. They have no I/O, no FFI, and no async runtime dependency. They are part of the `ArchiveSession` aggregate.

### 4.3 Domain Services

Pure functions that operate on domain models:

- `plan_changes(state: &SessionState, change_set: &ChangeSet) -> ExecutionPlan`
- `generate_changeset(state: &SessionState) -> ChangeSet`

These functions produce plans but do not execute them.

---

## 5. Ports Layer

The ports layer lives in a dedicated crate, e.g., `crates/ports` or `crates/domain-ports`. It contains only trait definitions. The types used in those traits (`ArchiveSession`, `ArchiveEntry`, `ChangeSet`, etc.) are defined in the Domain layer and imported by Ports. Ports does not define domain models.

Ports depends on Domain. Traits reference domain models but do not own them.

### 5.1 Reader / Writer Ports

```rust
pub trait ArchiveReader: Send + Sync {
    fn open(&self, path: &Path, password: Option<&Password>) -> Result<ArchiveSession, ArchiveError>;
    fn read_entries(&self, session: &ArchiveSession) -> Result<Vec<ArchiveEntry>, ArchiveError>;
    fn read_metadata(&self, session: &ArchiveSession, index: u32) -> Result<VfsMetadata, ArchiveError>;
    fn extract(&self, session: &ArchiveSession, indices: &[u32], dest: &Path, options: &ExtractOptions) -> Result<(), ArchiveError>;
    fn extract_to_buffer(&self, session: &ArchiveSession, index: u32) -> Result<Vec<u8>, ArchiveError>;
    fn test(&self, session: &ArchiveSession) -> Result<TestResult, ArchiveError>;
}

pub trait ArchiveWriter: Send + Sync {
    fn create(&self, path: &Path, format: ArchiveFormat, encryption: Option<&EncryptionConfig>) -> Result<ArchiveSession, ArchiveError>;
    fn commit(&self, session: &ArchiveSession, changeset: &ChangeSet) -> Result<(), ArchiveError>;
}
```

### 5.2 Supporting Ports

```rust
pub trait VfsProvider: Send + Sync {
    fn build(&self, entries: &[ArchiveEntry]) -> OverlayVfs;
}

pub trait CryptoProvider: Send + Sync {
    fn hash(&self, data: &[u8], algorithm: HashAlgorithm) -> Result<String, CryptoError>;
}

pub trait ProgressReporter: Send + Sync {
    fn report(&self, update: ProgressUpdate);
}

pub trait FileSystem: Send + Sync {
    async fn read(&self, path: &Path) -> Result<Vec<u8>, FsError>;
    async fn write(&self, path: &Path, data: &[u8]) -> Result<(), FsError>;
    async fn remove(&self, path: &Path) -> Result<(), FsError>;
}

pub trait TempStorage: Send + Sync {
    fn allocate(&self, prefix: &str) -> Result<TempDir, TempError>;
}

pub trait SessionStore: Send + Sync {
    fn insert(&self, state: SessionState);
    fn get(&self, id: SessionId) -> Option<Ref<SessionState>>;
    fn remove(&self, id: SessionId) -> Option<SessionState>;
}
```

---

## 6. Capability Layer

The capability layer lives in `crates/capability`. It is positioned between Application and Runtime. It answers the question: "Can this operation be executed, and by whom?"

### 6.1 CapabilityRegistry

Responsible for registering and unregistering backend capabilities.

```rust
pub struct Capability {
    pub id: CapabilityId,
    pub backend: BackendId,
    pub kind: CapabilityKind,
    pub formats: Vec<ArchiveFormat>,
    pub availability: Availability,
    pub metadata: CapabilityMetadata,
}

pub trait CapabilityRegistry: Send + Sync {
    fn register(&self, cap: Capability);
    fn unregister(&self, id: CapabilityId);
    fn all(&self) -> Vec<Capability>;
    fn by_backend(&self, backend: BackendId) -> Vec<Capability>;
}
```

### 6.2 CapabilityResolver

Responsible for matching an operation request against registered capabilities and producing an execution descriptor.

```rust
pub struct CapabilityRequest {
    pub kind: CapabilityKind,
    pub format: ArchiveFormat,
    pub constraints: Vec<Constraint>,
}

pub struct ExecutionDescriptor {
    pub backend: BackendId,
    pub capabilities: Vec<CapabilityId>,
    pub resource_claim: ResourceClaim,
    pub policy: ExecutionPolicy,
}

pub trait CapabilityResolver: Send + Sync {
    fn resolve(&self, request: CapabilityRequest) -> Result<ExecutionDescriptor, CapabilityError>;
}
```

If no capability matches, the resolver returns an error before the runtime is involved.

---

## 7. Runtime Layer

The runtime layer lives in `crates/runtime`. It is a general-purpose execution framework, not tied to archive concepts.

### 7.1 Runtime

The top-level facade.

```rust
pub struct Runtime {
    pub job_manager: Arc<dyn JobManager>,
    pub scheduler: Arc<dyn Scheduler>,
    pub executor: Arc<dyn Executor>,
    pub session_manager: Arc<dyn SessionManager>,
    pub resource_manager: Arc<dyn ResourceManager>,
    pub context: RuntimeContext,
}

impl Runtime {
    pub fn submit(&self, request: OperationRequest) -> OperationHandle;
    pub fn cancel(&self, handle: OperationHandle) -> Result<(), RuntimeError>;
    pub fn state(&self, handle: OperationHandle) -> Option<OperationState>;
}
```

### 7.2 JobManager

Accepts operations, manages job lifecycle, and exposes status.

```rust
pub trait JobManager: Send + Sync {
    fn submit(&self, operation: OperationRequest) -> OperationHandle;
    fn cancel(&self, handle: OperationHandle) -> Result<(), RuntimeError>;
    fn state(&self, handle: OperationHandle) -> Option<OperationState>;
    fn subscribe(&self, sender: OperationEventSender);
}
```

### 7.3 Scheduler

Decides which ready jobs run next based on priority, dependencies, resources, and session affinity.

```rust
pub trait Scheduler: Send + Sync {
    fn select_next(&self, ready: &[Job], running: &[JobHandle], capacity: ResourceCapacity) -> Vec<JobId>;
}
```

Scheduling dimensions:

- **Priority**: user-initiated operations beat background tasks.
- **Dependency graph**: jobs in a graph run in topological order.
- **Resource availability**: the scheduler asks `ResourceManager` whether a claim can be granted.
- **Session affinity**: operations targeting the same `SessionId` are serialized by default unless explicitly allowed to run concurrently.
- **Fairness**: starvation prevention for lower-priority queues.

### 7.4 Executor

Runs a job by breaking it into a task graph and executing tasks.

```rust
pub trait Executor: Send + Sync {
    fn execute(&self, job: Job, ctx: ExecutionContext) -> BoxFuture<'static, JobResult>;
}

pub struct ExecutionContext {
    pub runtime: RuntimeContext,
    pub cancellation: CancellationToken,
    pub progress: Arc<dyn ProgressReporter>,
    pub ports: PortProvider,
}
```

The executor is the only component that calls domain services and infrastructure ports.

### 7.5 SessionManager

A registry and lifecycle controller for `SessionState`. It does not own domain logic; it only manages the lifecycle and lookup of sessions.

```rust
pub trait SessionManager: Send + Sync {
    fn create(&self, session: ArchiveSession, vfs: OverlayVfs) -> Result<SessionId, RuntimeError>;
    fn get(&self, id: SessionId) -> Option<Ref<SessionState>>;
    fn close(&self, id: SessionId) -> Result<(), RuntimeError>;
    fn save(&self, id: SessionId) -> Result<(), RuntimeError>;
}
```

SessionManager:

- Creates sessions.
- Destroys sessions.
- Looks up sessions by id.
- Manages session lifecycle (auto-save, close on drop, persistence triggers).
- Uses weak references or handle tables to avoid leaking sessions.

The actual `SessionState` (including VFS) is stored through the `SessionStore` port.

### 7.6 ResourceManager

Arbitrates access to shared resources.

```rust
pub trait ResourceManager: Send + Sync {
    fn acquire(&self, claim: ResourceClaim) -> Result<ResourceToken, ResourceError>;
    fn query(&self) -> ResourceCapacity;
}

pub struct ResourceClaim {
    pub session_locks: Vec<SessionLock>,
    pub concurrency_token: bool,
    pub memory_budget: Option<usize>,
    pub thread_budget: Option<usize>,
    pub temp_storage: Option<usize>,
    pub handle_budget: Option<usize>,
}
```

First-version responsibilities:

- Prevent concurrent jobs from modifying the same archive session.
- Enforce a global limit on concurrent FFI operations.
- Track memory, threads, temporary directory, and handle budgets.
- Decide which capabilities must be exclusive (e.g., writing to a session).

---

## 8. Application Layer

Application services are thin orchestrators. They validate input, resolve capabilities, construct operation requests, and submit them to the runtime.

### 8.1 Example: SaveArchiveService

```rust
pub struct SaveArchiveService {
    resolver: Arc<dyn CapabilityResolver>,
    runtime: Arc<Runtime>,
}

impl SaveArchiveService {
    pub fn execute(&self, session_id: SessionId) -> Result<OperationHandle, ApplicationError> {
        let descriptor = self.resolver.resolve(CapabilityRequest {
            kind: CapabilityKind::Write,
            format: ArchiveFormat::SevenZip,
            constraints: vec![],
        })?;

        let request = OperationRequest {
            kind: OperationKind::SaveArchive { session_id },
            descriptor,
            priority: Priority::User,
        };

        Ok(self.runtime.submit(request))
    }
}
```

### 8.2 Example: ExtractService

```rust
pub struct ExtractService {
    resolver: Arc<dyn CapabilityResolver>,
    runtime: Arc<Runtime>,
}

impl ExtractService {
    pub fn execute(
        &self,
        session_id: SessionId,
        indices: Vec<u32>,
        destination: PathBuf,
    ) -> Result<OperationHandle, ApplicationError> {
        let descriptor = self.resolver.resolve(CapabilityRequest {
            kind: CapabilityKind::Extract,
            format: self.runtime.session_manager.get(session_id)?.session.format,
            constraints: vec![],
        })?;

        let request = OperationRequest {
            kind: OperationKind::Extract {
                session_id,
                indices,
                destination,
            },
            descriptor,
            priority: Priority::User,
        };

        Ok(self.runtime.submit(request))
    }
}
```

---

## 9. Data Flow Examples

### 9.1 Open Archive

1. UI calls `OpenArchiveService::execute(path)`.
2. Service resolves read capability → `ExecutionDescriptor`.
3. Service submits `OperationRequest::OpenArchive { path }` to Runtime.
4. Runtime JobManager creates a Job.
5. Scheduler dispatches the Job when resources allow.
6. Executor runs the Job:
   - Acquires a reader resource token.
   - Calls `ArchiveReader::open`.
   - Calls `ArchiveReader::read_entries`.
   - Builds VFS.
   - Stores `SessionState` via `SessionManager`.
7. Job completes; UI receives `OperationHandle` and eventual completion event.

### 9.2 Navigate Directory

1. UI calls `NavigateService::list_directory(session_id, path)`.
2. Service reads `SessionState` from `Runtime::session_manager`.
3. Service queries `OverlayVfs::list_directory` directly.
4. No Job is submitted; navigation is a synchronous domain query.

### 9.3 Commit Changes

1. UI calls `SaveArchiveService::execute(session_id)`.
2. Service resolves write capability.
3. Service submits `OperationRequest::SaveArchive { session_id }`.
4. Runtime schedules the Job.
5. Executor:
   - Acquires an exclusive write lock on the session.
   - Reads `SessionState`, generates `ChangeSet`.
   - Calls `ArchiveWriter::commit`.
   - Updates base tree and clears dirty state.
   - Releases lock.
6. UI receives completion event.

---

## 10. Error Handling & Cancellation

### 10.1 Error Propagation

- Domain errors stay in the domain.
- Port adapters convert infrastructure errors to domain errors.
- Runtime errors (scheduling, resource exhaustion) are separate from domain errors.
- Application services convert runtime/domain errors into UI-facing results.

### 10.2 Cancellation

- Every `OperationRequest` carries a `CancellationToken`.
- The Executor checks the token at task boundaries.
- On cancellation, the Executor aborts the current task and returns `JobResult::Cancelled`.
- `ResourceManager` releases tokens held by the cancelled job.
- `SessionManager` may roll back or mark the session as dirty depending on the operation.

---

## 11. Migration Path

The refactor is too large for a single PR. The actual implementation path is:

### Phase 1: Establish Layer Boundaries ✅

- Create `crates/ports`, `crates/capability`, and `crates/runtime` as skeleton crates.
- Define all core traits (`ArchiveReader`, `ArchiveWriter`, `CapabilityRegistry`, `CapabilityResolver`, `JobManager`, `Scheduler`, `Executor`, `SessionManager`, `ResourceManager`).
- Move `OverlayVfs` into `crates/domain` and add `ArchiveSession` / `SessionState`.
- Remove obsolete `crates/infrastructure/vfs`.
- Verify workspace compiles and tests pass.

### Phase 2: Implement Adapters ✅

Completed:

1. ✅ Implemented `Bit7zReaderAdapter` implementing `ArchiveReader` (open, read_entries, read_metadata, extract, extract_to_buffer, test).
2. ✅ Implemented `Bit7zWriterAdapter` implementing `ArchiveWriter` (create, commit).
3. ✅ Implemented `InMemorySessionStore` implementing `SessionStore`.
4. ✅ Added shared `ffi_util` helpers for entry reading and format detection.
5. ✅ Updated `ArchiveWriter::commit` port to accept a snapshot for conflict detection.
6. ✅ Kept `Bit7zRepository` unchanged; adapters are independent and ready for migration.
7. ✅ Added unit tests for `InMemorySessionStore`.

### Phase 3: Implement Runtime Internals

- Flesh out `DefaultScheduler`, `SimpleResourceManager`, and `LocalExecutor`.
- Implement `DefaultSessionManager` lifecycle and weak-reference handling.
- Add event stream publishing and progress reporting.

### Phase 4: Register Capabilities

- Register bit7z capabilities in `CapabilityRegistry`.
- Implement `DefaultCapabilityResolver` matching logic.
- Add constraints for encryption, solid archives, streaming, etc.

### Phase 5: Migrate Application Services

- Convert each use case service to resolve capabilities and submit `OperationRequest`.
- Examples: `OpenArchiveService`, `ExtractService`, `CommitService`, `TestService`.
- Keep navigation and session queries synchronous.

### Phase 6: Remove Bit7zRepository

- Once all services use the new runtime, delete `Bit7zRepository` and the `ArchiveRepository` trait.
- Clean up unused infrastructure code.

---

## 12. Crate Layout

```text
crates/
  capability/           # CapabilityRegistry + CapabilityResolver
  domain/               # Pure domain models and services (ArchiveSession, OverlayVFS, ChangeSet)
  ports/                # Abstract ports
  runtime/              # Runtime framework
  runtime/src/job.rs        # JobManager, Job, JobGraph, OperationRequest
  runtime/src/scheduler.rs  # Scheduler
  runtime/src/executor.rs   # Executor, ExecutionContext
  runtime/src/session.rs    # SessionManager
  runtime/src/resource.rs   # ResourceManager
  runtime/src/cancel.rs     # CancellationToken
  infrastructure/
    bit7z/              # FFI wrappers
    persistence/        # Bit7zReaderAdapter, Bit7zWriterAdapter
    fs/                 # FileSystem, TempStorage implementations
  application/
    archive/            # Archive use-case services
    checksum/           # Checksum services
    test/               # Test services
    preview/            # Preview services
    fs/                 # File-system use cases
```

---

## 13. Decisions

| Question | Decision |
|----------|----------|
| VFS location | `BaseVfs`, `OverlayVfs`, `DirtyTree`, and `EditQueue` all belong to the Domain layer. |
| Sync vs async | **Sync-first, async-capable.** Navigation and session queries remain synchronous. Only long-running operations (open, extract, commit, test) become Jobs. |
| Progress reporting | Runtime internally uses **event stream (pub/sub)**. The external API provides **both callback and stream** consumers. |

---

## 14. Open Questions

None remaining after the above decisions.
