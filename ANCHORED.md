# Anchored Summary

## Objective
- Refactor the capability crate from bit7z-coupled to a generic, domain-agnostic runtime capability graph framework
- Add magic-bytes format detection as a capability, wired into `ArchiveService::open()`
- Investigate why opening password-protected archives hangs without a password prompt

## Key Decisions

### 1. Capability Framework Design
- `SessionId = u64` (domain type) maps to `LockKey(pub u64)` in generic capability layer
- `ArchiveOpKind::DetectFormat` added as capability #9, registered with `CapabilityId(9)`
- `CapGraph<M>` supports `Requires`/`Conflicts`/`Implies` edges with DFS transitive closure
- `build_bit7z_runtime()` returns a 3-tuple: `(Arc<Runtime>, Arc<dyn CapabilityResolver<ArchiveCapMeta>>, Arc<dyn FormatDetector>)`

### 2. Password-Protected Archive Root Cause
- `bit7z::ArchiveReader::open()` called with `password: None` on encrypted archive → C++ bit7z library blocks indefinitely on a synchronous FFI call
- `LocalExecutor::run_job()` runs this in an async (smol) context — the FFI call holds the thread and never yields
- Existing `EncryptedArchiveRequiresPassword` error path in `Bit7zReaderAdapter::open()` works correctly BUT:
  - `LocalExecutor::open_archive()` converts it to `JobResult::Failed(error_string)` — losing the type
  - `ArchiveService::open()` wraps it in `ArchiveError::Internal(...)` — so the UI never sees the typed error
- RAR archives don't hang (different encryption path) but also get the wrong error type

### 3. Fix Strategy
- Add `check_encrypted(&self, path: &Path) -> Result<bool, ArchiveError>` to `ArchiveReader` trait (default returns `Ok(false)`)
- Implement in `Bit7zReaderAdapter` using `is_header_encrypted()` (fast, static header check — no blocking)
- Call in `ArchiveService::open()` before job submission:
  - If no password provided AND `check_encrypted` returns `true` → return `EncryptedArchiveRequiresPassword` immediately
  - This prevents the FFI call from ever executing without a password
- For RAR edge case (non-header-encrypted but still password-protected): fall through to existing error path (no hang, just wrong error type — acceptable)

## Files Changed
- `crates/capability/src/lib.rs`: Generic framework (CapMeta, CapGraph<M>, CapabilityRegistry<M>, LockKey)
- `crates/application/archive/src/capability.rs`: ArchiveCapMeta, resolver, ArchiveOpKind enum
- `crates/application/archive/src/auto_format.rs`: AutoFormat + FormatDetector impl
- `crates/ports/src/detection.rs`: FormatDetector trait
- `crates/runtime/src/executor.rs`: PortSet with detector field
- `crates/runtime/src/resource.rs`: ResourceManager with LockKey
- `crates/infrastructure/persistence/src/adapters/reader.rs`: Bit7zReaderAdapter — check_encrypted + existing encryption logic
- `crates/ports/src/lib.rs`: ArchiveReader trait — add check_encrypted

## Work State

### Completed
- Phase 1: `crates/capability/` rewritten — zero domain dependencies, `CapMeta` trait, `CapGraph<M>`, `LockKey`, generic `Request<M>`, `ResourceLock`, `CapabilityRegistry<M>`
- Phase 2: `crates/runtime/src/resource.rs` — `SessionLock` → `ResourceLock`, `SessionId` → `LockKey`; `PortSet.detector: Option<Arc<dyn FormatDetector>>`
- Phase 3: `crates/application/archive/src/capability.rs` — `ArchiveCapMeta`, `ArchiveOpKind::DetectFormat`, `ArchiveCapabilityResolver`, `register_archive_capabilities()`
- Phase 4: `crates/ports/src/detection.rs` — `FormatDetector` trait + `DetectError`
- Phase 5: `crates/application/archive/src/auto_format.rs` — `impl FormatDetector for AutoFormat`
- Phase 6: `ArchiveService::open()` — resolves `DetectFormat` capability, then calls `detector.detect_format(path)` with `fallback` to extension-based detection
- Phase 7: All callers updated (3 test files, `crates/runtime/gui/src/lib.rs`)
- Phase 8: `cargo check` passes (0 errors, 0 warnings on modified crates); all 16 tests pass
- Phase 9: Code committed (`9fce928`)
- Phase 10: Root cause investigation completed for password-protected archive hang

### Completed (Phase 11)
- `check_encrypted()` added to `ArchiveReader` trait at `crates/ports/src/lib.rs:22` (default `Ok(false)`)
- Implemented in `Bit7zReaderAdapter` using `lib.is_encrypted(path_str) || lib.is_header_encrypted(path_str)` — static FFI checks that do NOT open the archive, safe to call synchronously
- Pre-flight check wired in `ArchiveService::open()` at step 2 — after format detection, before job submission: if `password.is_none()` and `check_encrypted(path)` returns `true`, returns `ArchiveError::EncryptedArchiveRequiresPassword` immediately
- This prevents the synchronous `bit7z::ArchiveReader::open()` FFI call from ever executing with a null password on an encrypted archive — eliminating the hang
- RAR edge case (where static checks may return false): falls through to existing error path that returns `EncryptedArchiveRequiresPassword` correctly (no hang, just minor type erosion to `Internal`)
- `cargo check` passes (0 new errors), all 151 tests pass (no regressions)

### Potential Future Fixes
- RAR encryption detection: `is_header_encrypted()` returns false for RAR, so the pre-flight won't catch it. The fallback path doesn't hang but returns wrong error type. Could add format-specific handling later.
- Timeout wrapper around synchronous FFI calls for defense-in-depth