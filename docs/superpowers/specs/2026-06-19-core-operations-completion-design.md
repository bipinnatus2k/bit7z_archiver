# Core Operations Completion — Design Spec

**Date:** 2026-06-19  
**Status:** Design approved  
**Scope:** Wire all 9 partially-implemented subsystems + add menu bar and properties windows

## Overview

bit7z_archiver has ~9 items defined but orphaned (FFI exists, not wired; windows built, never shown). This spec completes all of them and adds a menu bar and properties windows as essential supporting UI.

## Architecture Layers

Work proceeds bottom-up: L1 → L2 → L3 → L4. Items within L1 are independent.

```
L4: Views           (menu, toolbar, windows, context menu)
L3: ViewModels      (ArchiveVM, ProgressState, TestResult)
L2: Application     (use cases + progress channel)
L1: Adapters        (repository FFI, CLI, Linux platform)
```

---

## Layer 1 — Adapters

### 1.1 Bit7zRepository: Wire add/delete/rename/test

All required C++ bridge methods already exist (`bit7z_editor_*`, `bit7z_reader_test`, `bit7z_writer_add_*`). Safe Rust wrappers exist in `adapters/bit7z/mod.rs`. Only wiring needed.

| Method | Implementation |
|---|---|
| `test_archive(&Handle)` | Call `bit7z_reader_test()` on existing `Reader` |
| `test_entries(&Handle, indices)` | Extract each entry to buffer, compare CRC. Returns per-item pass/fail. If a directory index is given, recursively test all child entries. If bit7z exposes per-item verify API, use it; otherwise extract-to-memory + CRC compare |
| `delete(&Handle, indices)` | Open `Editor`, call `editor.delete(i)` for each index, `editor.apply()` once |
| `rename(&Handle, index, name)` | Open `Editor`, call `editor.rename(idx, name)` preserving directory portion, `editor.apply()` |
| `add(&Handle, paths)` | Open archive in update mode via `Writer`, call `bit7z_writer_add_files(paths)`, compress |

### 1.2 Compress CLI

Replaces the placeholder that prints "pending". Flow:

```text
compress files.. --to <path> --format <fmt> --password <pwd>
  1. Enumerate input paths (recursive for directories)
  2. Create archive at --to with specified format + optional password
  3. Add enumerated files via Writer::add_file()
  4. Print summary: files added, total size, compression ratio
```

Format defaults to `.7z` when not specified.

### 1.3 Linux Platform

**Tray icon:** Replace sleep-loop stub with real D-Bus `StatusNotifierItem` using the `zbus` crate. Register icon, show context menu (Open/Exit), respond to Activate signal (restore window).

**File dialogs:** Replace `#[cfg(not(windows))]` stubs in `platform.rs` that return `None`. Use the cross-platform `rfd` crate for `pick_archive_file()` and `pick_folder()`.

### 1.4 Interface Changes

`ArchiveRepository` trait adds one method:

```rust
fn set_progress_sender(&mut self, tx: Sender<ProgressUpdate>);
```

Existing `add()`, `delete()`, `rename()`, `test()` signatures already match what is needed — no trait breakage.

New type:

```rust
struct ArchiveProperties {
    format: ArchiveFormat,
    location: PathBuf,
    uncompressed_size: u64,
    compressed_size: u64,
    ratio: f64,
    file_count: usize,
    folder_count: usize,
    modified: Option<SystemTime>,
    solid: bool,
    encrypted: bool,
    encrypted_names: bool,
    multi_volume: bool,
    has_comment: bool,
    comment_size: Option<usize>,
    has_recovery_record: bool,
    locked: bool,
    dictionary_size: Option<u64>,
}
```

New trait method:

```rust
fn get_archive_properties(&self, handle: &ArchiveHandle) -> Result<ArchiveProperties>;
```

---

## Layer 2 — Application

### 2.1 Progress Channel

All long-running operations share a common progress channel.

```rust
struct ProgressUpdate {
    current: u64,          // items processed
    total: u64,            // total items
    bytes_processed: u64,
    current_file: Option<String>,
    error: Option<String>, // non-None on fatal error
}
```

Channel: `crossbeam::unbounded::<(Sender<ProgressUpdate>, Receiver<ProgressUpdate>)>`. Created per-operation by the view layer, sender passed to repo, receiver passed to ProgressState.

### 2.2 Use Case Changes

| Use Case | Change |
|---|---|
| `ExtractEntriesUseCase` | Accept `Sender<ProgressUpdate>`, pass to repo before `extract()` |
| `CreateArchiveUseCase` | Accept sender, wire progress from compress callback |
| `AddToArchiveUseCase` | Accept sender, wire through repo. Validate input paths exist before calling repo |
| `DeleteEntriesUseCase` | Accept sender, progress = items deleted / total |
| `RenameEntryUseCase` | No progress needed (single item, instant) |
| `TestEntriesUseCase` | New: accepts `indices: Option<Vec<usize>>` (None = entire archive). Progress = items tested / total. Returns `TestResult` |
| `TestArchiveUseCase` | Convenience: calls `TestEntriesUseCase` with `indices: None` |

### 2.3 TestResult

```rust
struct TestResult {
    passed: usize,
    failed: Vec<TestFailure>,
}

struct TestFailure {
    index: usize,
    path: String,
    reason: TestFailureReason,
}

enum TestFailureReason {
    CrcMismatch { expected: u32, actual: u32 },
    ReadError(String),
}
```

`TestEntriesUseCase` succeeds even when some entries fail — failures are aggregated, not errors. Only a total inability to read the archive produces an `Err`.

---

## Layer 3 — ViewModels

### 3.1 ArchiveVM Changes

| Method | Implementation |
|---|---|
| `delete_selected()` | Call `DeleteEntriesUseCase` with selected indices, refresh listing, clear selection |
| `add_files()` | Open platform file/folder picker, call `AddToArchiveUseCase`, refresh listing |
| `rename_entry(index)` | Open inline rename input or small prompt, call `RenameEntryUseCase`, refresh listing |
| `test_selected()` | Call `TestEntriesUseCase` with selected indices; if no selection, test entire archive. Open Test Results window on failures |

### 3.2 ProgressState

```rust
struct ProgressState {
    is_active: bool,
    is_complete: bool,
    error_message: Option<String>,
    current: u64,
    total: u64,
    bytes_processed: u64,
    current_file: Option<String>,
}

impl Global for ProgressState {}
```

Registration: `gui.rs` calls `cx.set_global(ProgressState::default())` at startup.

Start flow:
1. View layer creates `(tx, rx)` channel
2. Calls `ProgressState::global(cx).start(rx)`
3. Opens `ProgressDialog` window
4. `ProgressState` spawns a background task reading `rx` in a loop, updating fields
5. When `tx` is dropped → `rx` closes → `ProgressState::complete()`
6. If an update has `error: Some(...)` → `ProgressState` sets `error_message`

`ProgressDialog` reads `ProgressState::global(cx)` each frame — no prop drilling.

### 3.3 Test Result Flow

After `TestEntriesUseCase` completes:
- All passed: brief status bar message "X files tested, all passed"
- Any failed: open Test Results window with collapsed failed list

---

## Layer 4 — Views

All dialogs are independent GPUI windows opened via `cx.open_window()`.

### 4.1 Menu Bar

Placed above the toolbar in `RootView`. Every menu action dispatches the same events as toolbar/context menu — no logic duplication. Keyboard shortcuts shown inline.

| Menu | Items |
|---|---|
| **File** | Open Archive (Ctrl+O), Create Archive (Ctrl+N), Add Files, ─, Close Archive, ─, Properties (Alt+Enter), ─, Exit |
| **Edit** | Select All (Ctrl+A), Invert Selection, ─, Copy, Cut, Paste, ─, Delete (Del), Rename (F2) |
| **View** | Large Icons, Small Icons, List, Details, ─, Flat View, ─, Show: Toolbar, Status Bar, Preview Panel, Directory Tree |
| **Tools** | Test Archive, ─, Settings |
| **Favorites** | Add to Favorites, Organize Favorites, ─, (recent archives list) |
| **Help** | About |

### 4.2 Toolbar

| Current | New |
|---|---|
| Open, Create, Extract, Test, Close | + **Add** (adds files to open archive), + **Settings** (gear icon, right-aligned) |

Buttons dispatch same events as corresponding menu items.

### 4.3 Context Menu (Archived File List)

Right-click on entries:

```
Extract...
Add to archive...
Test
──────────────
Rename
Delete
──────────────
Select All
Clear Selection
──────────────
Properties
──────────────
Refresh
```

### 4.4 Settings Window

- Opened via **Tools → Settings** or toolbar gear button
- Renders 4 tabs: General, Archive, Preview, Appearance
- Already fully built — only needs window wrapper and trigger
- Remove unused `settings_dialog: Option<Entity<SettingsDialog>>` from `RootView`

### 4.5 Progress Window

Opened automatically when any long operation starts. Closed on completion, error, or cancel.

```
┌─────────────────────────────────────┐
│ Extracting — archive.7z             │
├─────────────────────────────────────┤
│ ████████████░░░░░░░░░░░░  42%       │
│ Current: documents/report.pdf       │
│ 67/156 files · 5.8/14.2 MB          │
│                          [Cancel]   │
└─────────────────────────────────────┘
```

Progress flow:
1. Button click → create channel `(tx, rx)` → `ProgressState::start(rx)` → `cx.open_window(ProgressDialog)` → spawn repo thread with `tx`
2. Thread sends `ProgressUpdate` messages periodically → ProgressState updates → dialog re-renders
3. Thread ends → `tx` dropped → rx closes → `ProgressState::complete()` → dialog shows "Done" → Close button appears
4. Cancel → sends cancel signal to worker thread (reuse existing cancel mechanism)

### 4.6 Create Dialog Polish

Replace static text placeholders:
- **Format dropdown**: GPUI `PickList` bound to `selected_format` (7z, Zip, Tar, Tar.gz, Tar.xz, Tar.bz2)
- **Compression level**: Slider or dropdown (None, Fastest, Fast, Normal, Max, Ultra)
- **Advanced section** (collapsed by default): method, dictionary size, word size, solid toggle, volume size, thread count

### 4.7 Extract Dialog Polish

- **Browse button**: Opens `pick_folder()` from `platform.rs`, writes path to destination field
- **Overwrite mode dropdown**: Ask / Overwrite / Skip / Rename extracted
- **Keep broken files** checkbox: continue on CRC error

### 4.8 Add Files Dialog (New Window)

```
┌──────────────────────────────────────┐
│ Add Files — archive.7z               │
├──────────────────────────────────────┤
│ Files:  [+ Add files] [+ Add folder]│
│ ┌──────────────────────────────────┐ │
│ │ Documents\report.pdf             │ │
│ │ Images\photo.jpg                 │ │
│ └──────────────────────────────────┘ │
│                                       │
│ Update mode: [Add & replace ▾]       │
│ Compression: [Normal ▾]              │
│ Password: [···············]          │
│                                       │
│ ── Advanced (collapsed) ──           │
│                                       │
│                 [Cancel]  [OK]       │
└──────────────────────────────────────┘
```

- [+ Add files] opens `pick_archive_file()` (multi-select)
- [+ Add folder] opens `pick_folder()`
- Update mode: Add & replace, Add & update (newer only), Freshen (existing only)
- Compression/password override the archive defaults

### 4.9 Properties Window — Archive (New Window)

Triggered via **File → Properties** (with no entries selected) or Alt+Enter.

```
┌─────────────────────────────────────────┐
│ Properties — archive.7z                 │
├─────────────────────────────────────────┤
│ General                                 │
│   Type:        7-Zip                    │
│   Location:    C:\Users\...\archive.7z  │
│   Size:        14.2 MB (14,940,160)     │
│   Packed:      9.8 MB (10,289,152)      │
│   Ratio:       69%                      │
│   Files:       156                      │
│   Folders:     12                       │
│   Modified:    2026-06-19 14:22         │
│                                         │
│ Advanced                                │
│   Solid:           Yes                  │
│   Encrypted:       Yes (AES-256)        │
│   Encrypted names: Yes                  │
│   Multi-volume:    No                   │
│   Comment:         Yes (142 bytes)      │
│   Recovery record: No                   │
│   Locked:          No                   │
│   Dictionary:      64 MB                │
│                              [Close]     │
└─────────────────────────────────────────┘
```

New repository method: `get_archive_properties(handle) -> ArchiveProperties`. Most data available via existing `bit7z_reader_*` calls. New FFI may be needed for: dictionary size, volume info, comment presence, locked flag.

### 4.10 Properties Window — Selected Entries (New Window)

Triggered via **File → Properties** (with entries selected) or right-click → Properties or Alt+Enter.

**Multiple entries selected** — aggregate totals + collapsible list:
```
┌─────────────────────────────────────────┐
│ Properties — Selected (3 items)         │
├─────────────────────────────────────────┤
│ Size:    2.4 MB                         │
│ Packed:  1.8 MB                         │
│ Files:   3                              │
│ ▶ Entries                               │
│                              [Close]     │
└─────────────────────────────────────────┘
```

**Single entry** — full detail:
```
┌─────────────────────────────────────────┐
│ Properties — report.pdf                 │
├─────────────────────────────────────────┤
│ Name:      report.pdf                   │
│ Type:      PDF Document                 │
│ Path:      Documents\report.pdf         │
│ Size:      1.2 MB (1,257,472)           │
│ Packed:    850 KB (870,400)             │
│ Ratio:     70%                          │
│ CRC32:     A3F7C21B                     │
│ Modified:  2026-06-15 09:30             │
│ Host OS:   Windows                      │
│ Encrypted: No                           │
│                              [Close]     │
└─────────────────────────────────────────┘
```

Per-entry data already available on `ArchiveEntry` — no new FFI needed. Multi-select aggregates from existing entry data.

---

## Error Handling

| Layer | Strategy |
|---|---|
| **Repository** | Remove `UnsupportedOperation` for add/delete/rename/test. Return specific errors: `EntryNotFound`, `EditorError(String)`, `TestFailed { index, reason }`, `PermissionDenied` |
| **Use Cases** | `TestEntriesUseCase` aggregates per-item failures, never fails the entire use case. `AddToArchiveUseCase` validates paths exist before calling repo |
| **Progress** | Fatal errors sent as `ProgressUpdate { error: Some(...) }` before sender drops. `ProgressState` stores last error; `ProgressDialog` displays it |
| **Windows** | Failure opening child windows (Settings, Progress, Properties) logs to stderr; does not crash the main window |

---

## Testing Strategy

### Integration Tests (tests/)

| What | How |
|---|---|
| `Bit7zRepository` add/delete/rename | Test archives in `tests/fixtures/`: `.7z` and `.zip` with known contents |
| `TestEntriesUseCase` | Known-good archive (all pass), intentionally corrupted entry (some fail) |
| `TestEntriesUseCase` on directories | Test a directory entry — verifies all children pass/fail correctly |
| Compress CLI | Round-trip: compress a directory, then extract and compare |

### Unit Tests

| What | How |
|---|---|
| `ProgressState` | Send updates, verify fields, verify completion on sender drop, verify error capture |
| `ArchiveProperties` | Verify property parsing from test archives |
| `TestResult` | Verify aggregation logic |

### UI Tests (GPUI test harness)

| What | How |
|---|---|
| Settings window | Render, verify tab switching, verify save emits correct event |
| Progress window | Render with known state, verify bar/cancel/close |
| Create dialog | Verify format dropdown, compression slider bound correctly |
| Menu bar | Verify keyboard shortcuts dispatch correct events |
| Properties windows | Archive and entry variants |

### Test Fixtures

Checked into `tests/fixtures/`:

| File | Contents | Purpose |
|---|---|---|
| `basic.7z` | 3 text files, 1 subdirectory | General testing |
| `basic.zip` | Same contents in Zip format | Cross-format testing |
| `corrupted.7z` | 1 good entry + 1 CRC-mismatched entry | Test failure aggregation |
| `encrypted.7z` | Password-protected with encrypted filenames | Encryption flows |
| `empty.7z` | Empty archive | Edge case |

---

## Scope Boundaries

| In Scope (This Spec) | Out of Scope (Future Specs) |
|---|---|
| Settings window wired | Themes / skins |
| Progress for all operations | Taskbar progress bar, CLI progress output |
| Menu bar with all menus | Menu bar customization |
| Properties (archive + entries) | Comment editing |
| Add Files dialog with update mode | Include/exclude file masks |
| Create dialog with format + compression level | Multi-volume, solid/thread/dictionary config beyond basic toggle |
| Test entries (all + selected) | Recover / repair archive |
| Context menu expanded | Send To / 7-zip submenu |
| Compress CLI | CLI progress, list archives to stdout |
| Linux tray (basic D-Bus) + file dialogs (rfd) | Two-panel mode, macOS support |
| Add/Delete/Rename/Test in repository | Archive conversion, batch ops, SFX, benchmarking |
| — | Flat view, thumbnails, file type icons, drag & drop, address bar, favorites panel, wizard mode |
