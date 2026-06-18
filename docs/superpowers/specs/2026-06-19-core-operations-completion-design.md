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

#### New C wrapper functions needed (for properties, detailed in 4.9/4.10)

bit7z exposes 96 properties via `BitProperty` enum. Our `demo.h` currently wraps only 7. For properties windows we need:

| C wrapper | bit7z source | Purpose |
|---|---|---|
| `bit7z_item_mtime()` | `BitArchiveItem::lastWriteTime()` | Modified timestamp — all formats |
| `bit7z_item_ctime()` | `BitArchiveItem::creationTime()` | Creation time — NTFS, 7z, some tar |
| `bit7z_item_atime()` | `BitArchiveItem::lastAccessTime()` | Access time — NTFS, 7z |
| `bit7z_item_attributes()` | `BitArchiveItem::attributes()` | Windows file attributes |
| `bit7z_item_host_os()` | `itemProperty(HostOS)` | Host OS code |
| `bit7z_item_compression_method()` | `itemProperty(Method)` | Compression algorithm name |
| `bit7z_item_comment()` | `itemProperty(Comment)` | Per-item comment text |
| `bit7z_item_user()` | `itemProperty(User)` | Owner user (tar, 7z Unix) |
| `bit7z_item_group()` | `itemProperty(Group)` | Owner group (tar, 7z Unix) |
| `bit7z_item_is_symlink()` | `BitArchiveItem::isSymLink()` | Symlink flag |
| `bit7z_item_posix_attrib()` | `itemProperty(PosixAttrib)` | POSIX file mode (tar, 7z) |
| `bit7z_item_extension()` | `BitArchiveItem::extension()` | File extension |

Existing `bit7z_item_crc()` is already in `demo.h` but repository code hardcodes `crc: None` — just needs to be called.

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

**Tray icon:** Replace sleep-loop stub with real D-Bus `StatusNotifierItem` using the `zbus` crate. Register icon, show context menu (Open/Exit), respond to Activate signal (restore window). Additionally:
- During active operations (extract, compress, test, add), the tray icon **tooltip** updates to show progress: `"bit7z — Extracting 42%"` or `"bit7z — Compressing 58%"`
- On operation completion while minimized to tray: show a brief notification (OS toast or balloon tooltip) with result: `"Extraction complete — 156 files extracted"`
- On error: notification shows error summary
- Left-click always restores the main window. If a progress operation is running, left-click restores the progress window instead.

**File dialogs:** Replace `#[cfg(not(windows))]` stubs in `platform.rs` that return `None`. Use the cross-platform `rfd` crate for `pick_archive_file()` and `pick_folder()`.

#### Domain struct changes

Existing `ArchiveEntry` has dead fields (`is_symlink`, `modified`, `crc` hardcoded). New fields added:

```rust
struct ArchiveEntry {
    name: String,
    path: String,
    size: u64,
    compressed_size: u64,
    is_directory: bool,
    original_index: u32,
    // Now populated (were dead)
    is_symlink: bool,
    modified: Option<DateTime<Utc>>,
    crc: Option<u32>,
    // New fields
    created: Option<DateTime<Utc>>,
    accessed: Option<DateTime<Utc>>,
    attributes: Option<u32>,
    posix_attrib: Option<u32>,
    host_os: Option<u8>,
    compression_method: Option<String>,
    comment: Option<String>,
    user: Option<String>,
    group: Option<String>,
    is_encrypted: bool,
}
```

All new fields are `Option` — different formats provide different subsets. 7z provides nearly everything. Zip provides crc, modified, encryption, attributes (but creation/access times depend on Zip version; PKZip 2.04g only stores modified). Tar provides uid/gid (via user/group), posix_attrib, symlink. RAR provides crc, modified, host_os, encryption.

### 1.4 Trait Interface Changes

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
    // Per-file (top bar)
    file_current: u64,
    file_total: u64,
    current_file: Option<String>,
    // Overall (bottom bar)
    items_done: u64,
    items_total: u64,
    bytes_done: u64,
    bytes_total: u64,
    error: Option<String>,
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

### 2.4 New Use Cases

| Use Case | Purpose |
|---|---|
| `CalculateChecksumUseCase` | Temp-extract selected entries, compute CRC32/MD5/SHA1/SHA256. Accepts `algorithm: ChecksumAlgorithm`. Returns `Vec<(String, ChecksumResult)>` mapping path to hash hex string |
| `OpenEntryUseCase` | Temp-extract a single non-directory entry to system temp, then shell-execute it with the OS-associated program. Watches for file changes after the program closes; prompts user to update in archive |
| `NewFolderUseCase` | Creates an empty directory entry in the archive via the Editor. Returns the new entry's path |
| `NewFileUseCase` | Creates a new empty file in system temp, opens editor, on save adds to archive |

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
    is_paused: bool,
    error_message: Option<String>,
    // Per-file
    file_current: u64,
    file_total: u64,
    current_file: Option<String>,
    // Overall
    items_done: u64,
    items_total: u64,
    bytes_done: u64,
    bytes_total: u64,
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
| **File** | Open Archive (Ctrl+O), Create Archive (Ctrl+N), Add Files, ─, Test Archive (Ctrl+T), ─, Open (Enter), View (Ctrl+V), Edit (F4), ─, New Folder (Ctrl+Shift+N), New File, ─, Close Archive, ─, Properties (Alt+Enter), ─, Exit |
| **Edit** | Select All (Ctrl+A), Invert Selection, ─, Copy, Cut, Paste, ─, Delete (Del), Rename (F2) |
| **View** | Large Icons, Small Icons, List, Details, ─, Flat View, ─, Show: Toolbar, Status Bar, Preview Panel, Directory Tree |
| **Tools** | Checksum: CRC32, MD5, SHA1, SHA256 (submenu, on selected entries), ─, Settings |
| **Favorites** | Add to Favorites, Organize Favorites, ─, (recent archives list) |
| **Help** | About |

File menu behavior:
- **Test Archive** (Ctrl+T): with no selection → tests entire archive. With entries selected → shows submenu: "Test Selected Files" / "Test Entire Archive". Both open Test Results window.
- **Open** (Enter): opens the selected file with the OS-associated program (temp extract to system temp + shell execute). Works only on non-directory entries.
- **View** (Ctrl+V): same as existing Preview — shows file contents in preview panel.
- **Edit** (F4): temp-extracts the file, opens with associated editor, watches for changes, prompts to update in archive on close.
- **New Folder**: prompts for name, creates empty directory entry in the open archive.
- **New File**: opens a blank temp file in editor, on save adds it to the archive.

Tools menu behavior:
- **Checksum** submenu: visible only when 1+ entries selected. Calculates and displays CRC32 / MD5 / SHA1 / SHA256 of selected entries (temp extract + hash). Shows result in a small popup window.

### 4.2 Toolbar

| Current | New |
|---|---|
| Open, Create, Extract, Test, Close | + **Add** (adds files to open archive), + **Settings** (gear icon, right-aligned) |

Buttons dispatch same events as corresponding menu items.

### 4.3 Context Menu (Archived File List)

Right-click on entries:

```
Open (Enter)
View
Edit
──────────────
Extract...
Add to archive...
──────────────
Test Selected
Test Entire Archive
──────────────
Checksum ▶         CRC32
                    MD5
                    SHA1
                    SHA256
──────────────
Rename (F2)
Delete (Del)
──────────────
New Folder
New File
──────────────
Select All (Ctrl+A)
Clear Selection
──────────────
Properties (Alt+Enter)
──────────────
Refresh (F5)
```

Right-click on **empty space** (no selection) in the file list:

```
New Folder
New File
──────────────
Select All
──────────────
Refresh
```

Checksum behavior:
- Requires 1+ non-directory entries selected
- Temp-extracts each selected file and computes the hash
- Shows result in a small popup: "CRC32: A3F7C21B | MD5: d41d8cd9... | SHA1: da39a3ee... | SHA256: e3b0c442..."
- For multiple files, aggregates are shown per-file in a list

### 4.4 Settings Window

- Opened via **Tools → Settings** or toolbar gear button
- Renders 4 tabs: General, Archive, Preview, Appearance
- Already fully built — only needs window wrapper and trigger
- Remove unused `settings_dialog: Option<Entity<SettingsDialog>>` from `RootView`

### 4.5 Progress Window

Opened automatically when any long operation starts. Closed on completion, error, or cancel. **Two progress bars** — per-file and overall:

```
┌────────────────────────────────────────┐
│ Extracting — archive.7z                │
├────────────────────────────────────────┤
│ File:  ████████████████████  100%      │
│        documents/report.pdf            │
│                                        │
│ Total: ████████░░░░░░░░░░░░   42%      │
│        67 / 156 files                  │
│        5.8 MB / 14.2 MB                │
│          [Hide]  [Pause]  [Cancel]     │
└────────────────────────────────────────┘
```

**Hide to Tray:** The `[Hide]` button minimizes the progress window to the system tray. The window closes visually but the operation continues in background. The tray icon updates to show overall percentage as its icon tooltip: "bit7z — Extracting 42%". On operation completion, the tray icon shows a completion notification. Left-clicking the tray icon restores the progress window.

**Pause/Resume:** The progress window has a Pause button. On click:
1. Sends a pause signal to the worker thread (reuse existing `pause` mechanism from `worker.rs` → `AtomicBool`)
2. Pause button changes to "Resume"
3. Progress bars freeze at current position
4. On Resume → worker continues, bars resume

`ProgressState` adds a `is_paused: bool` field. The dialog shows either `[Pause]` or `[Resume]` based on this flag. Pause is supported for extract, compress (create), add files, and test operations. Rename (instant) and delete have no pause button.

**Top bar (per-file):** Resets to 0% for each new file. Shows current file name. Useful for large individual files where per-file progress matters.

**Bottom bar (overall):** Monotonic — counts files processed / total. Shows aggregate byte progress.

ProgressUpdate struct adjusted:
```rust
struct ProgressUpdate {
    // Per-file (top bar)
    file_current: u64,          // bytes processed in current file
    file_total: u64,            // total bytes of current file
    current_file: Option<String>,
    // Overall (bottom bar)
    items_done: u64,
    items_total: u64,
    bytes_done: u64,
    bytes_total: u64,
    error: Option<String>,
}
```

Progress flow:
1. Button click → create channel `(tx, rx)` → `ProgressState::start(rx)` → `cx.open_window(ProgressDialog)` → spawn repo thread with `tx`
2. Thread sends `ProgressUpdate` messages periodically → ProgressState updates → dialog re-renders both bars
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
┌──────────────────────────────────────────┐
│ Add Files — archive.7z                   │
├──────────────────────────────────────────┤
│ Format:  [7z ▾]                          │ ← pre-filled to current archive
│                                          │
│ Source                                   │
│  [+ Add files] [+ Add folder]           │
│ ┌──────────────────────────────────────┐ │
│ │ C:\Users\...\documents\              │ │
│ │   filter: [*.pdf;*.txt      ]        │ │
│ │   [✓] Recurse subdirectories         │ │
│ │   policy: [Include ▾]                │ │
│ └──────────────────────────────────────┘ │
│                                          │
│ Archive path prefix: [docs/      ]      │ ← optional custom path prefix
│                                          │
│ Update mode                              │
│  [Add & replace files ▾]                │ ← Append / Update
│                                          │
│ Compression                              │
│  Level:  [──●────────────] Normal       │ ← slider 0-5
│  Method: [LZMA2 ▾]                      │ ← format-dependent options
│                                          │
│ ── Advanced (collapsed) ──              │
│   Dictionary: [64 MB ▾]                 │ ← 7z: 64KB-1536MB
│   Word size: [273 ▾]                    │ ← LZMA/LZMA2
│   [ ] Solid archive                     │ ← 7z only: shown for 7z
│   Split to volumes: [         ] MB      │ ← 0 = no split
│   Threads:  [4              ]           │
│   [✓] Store modified timestamps         │
│   [ ] Store creation timestamps         │
│   [ ] Store access timestamps           │
│                                          │
│ Encryption                               │
│  Password:    [················]        │
│  Confirm:     [················]        │
│  [ ] Encrypt file names (7z only)       │ ← EncryptionScope: DataAndHeaders
│                                          │
│                    [Cancel]  [OK]       │
└──────────────────────────────────────────┘
```

**Format dropdown:** Pre-filled to the currently open archive's format. Options: 7z, Zip, Tar, Tar.gz, Tar.bz2, Tar.xz (writable formats only). When format changes, the compression/encryption controls below adapt (see format-dependent table). This allows re-compressing files into a different format than the source archive.

| Control | 7z | Zip | Tar | GZip/BZip2/Xz |
|---|---|---|---|---|
| Compression Level | All 6 levels | All 6 | N/A (hidden) | All 6 |
| Method dropdown | LZMA2, LZMA, PPMd, BZip2, Copy | Deflate, Deflate64, BZip2, LZMA, PPMd, Copy | Copy only (hidden) | Depends on format |
| Dictionary size | 64KB–1536MB (shown for LZMA/LZMA2) | Hidden | Hidden | Hidden |
| Word size | Shown for LZMA/LZMA2 | Hidden | Hidden | Hidden |
| Solid archive | Shown ✓ | Hidden | Hidden | Hidden |
| Split to volumes | Shown ✓ | Shown ✓ | Hidden | Hidden |
| Encrypt file names | Shown ✓ (DataAndHeaders) | Hidden (Zip cannot encrypt names) | Hidden | Hidden |

**Update mode:**
| Mode | Behavior |
|---|---|
| Add & replace (Append) | Add new files, leave existing files unchanged (no overwrite) |
| Add & update (Update) | Overwrite files with matching paths, append new ones |

**Source types:**
- `[+ Add files]`: multi-select file picker → adds individual file paths
- `[+ Add folder]`: folder picker → adds directory with wildcard filter (e.g., `*.pdf;*.txt`)
  - Default filter: `*` (all files)
  - Default policy: Include. Switching to Exclude inverts (add all EXCEPT matching)
  - Recurse checkbox: default on
- Archive path prefix: prepends a virtual directory to all added files (e.g., `docs/` → files appear under `docs/` inside archive)

**New C wrapper functions needed** (to expose bit7z settings not yet in demo.h):

| C wrapper | bit7z source |
|---|---|
| `bit7z_writer_set_compression_method(writer, method: int)` | `setCompressionMethod()` |
| `bit7z_writer_set_dictionary_size(writer, bytes: uint32_t)` | `setDictionarySize()` |
| `bit7z_writer_set_word_size(writer, bytes: uint32_t)` | `setWordSize()` |
| `bit7z_writer_set_solid_mode(writer, solid: bool)` | `setSolidMode()` |
| `bit7z_writer_set_volume_size(writer, bytes: uint64_t)` | `setVolumeSize()` |
| `bit7z_writer_set_encryption_scope(writer, scope: int)` | `setPassword(pwd, EncryptionScope)` — overload |
| `bit7z_writer_add_dir_filtered(writer, dir, filter, policy: int, recursive: bool)` | `addFiles(dir, filter, policy, recursive)` |
| `bit7z_writer_add_items(writer, paths, archive_paths, count)` | `addItems(vector<pair<fsPath, archivePath>>)` |
| `bit7z_writer_set_store_timestamps(writer, modified, created, accessed: bool)` | Three setters |

**Safe Rust wrapper changes:** New `Writer` methods matching the above. `WriterCompressionMethod` enum: `Copy=0, Deflate=1, Deflate64=2, BZip2=3, Lzma=4, Lzma2=5, Ppmd=6`. `EncryptionScope` enum: `DataOnly=0, DataAndHeaders=1`.

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

Properties are grouped into **categories**. Only categories that have at least one non-empty value are shown. This naturally handles format-specific differences (e.g., tar shows uid/gid, zip may not have creation time).

#### Single entry — full detail:

```
┌───────────────────────────────────────────┐
│ Properties — report.pdf                   │
├───────────────────────────────────────────┤
│ General                                   │
│   Name:      report.pdf                   │
│   Type:      PDF Document                 │
│   Path:      Documents\report.pdf         │
│   Size:      1.2 MB (1,257,472 bytes)     │
│   Packed:    850 KB (870,400 bytes)       │
│   Ratio:     70%                          │
│   CRC32:     A3F7C21B                     │
│                                           │
│ Time                                      │
│   Modified:  2026-06-15 09:30             │
│   Created:   2026-06-15 09:28            │ ← hidden on old Zip
│   Accessed:  2026-06-19 14:00            │ ← hidden on old Zip
│                                           │
│ Platform                                  │
│   Host OS:   Windows (0)                 │
│   Attributes:0x00000020 (Archive)         │ ← Windows only
│   POSIX:     0o644 (-rw-r--r--)          │ ← tar / 7z Unix only
│   Owner:     alice                        │ ← tar / 7z Unix only
│   Group:     users                        │ ← tar / 7z Unix only
│                                           │
│ Security                                  │
│   Encrypted: No                           │
│                                           │
│ Technical                                 │
│   Method:    LZMA2                        │
│   Symlink:   —                            │ ← shown if symlink
│   Comment:   —                            │ ← shown if has comment
│                                [Close]     │
└───────────────────────────────────────────┘
```

#### Category visibility rules:

| Category | Shown when... |
|---|---|
| **General** | Always |
| **Time** | Any of modified/created/accessed is non-None. Zip v2.0 only stores modified — created/accessed rows are hidden |
| **Platform** | At least one of host_os, attributes, posix_attrib, user, group is non-None. Tar and 7z-on-Unix show POSIX/user/group. Zip/7z-on-Windows show attributes. RAR shows host_os |
| **Security** | Always (shows encryption status) |
| **Technical** | At least one of method, symlink, comment is non-None |

#### Multiple entries selected:

Aggregate totals (sum of sizes) + collapsed entry list. The collapsed list shows each entry with its own per-format properties in a DataTable — entries that have a given property show it; entries that don't leave the cell blank. Columns: Name, Size, Packed, Ratio, CRC, Modified, Encryption. The table auto-hides columns where no selected entry has that property.

```
┌──────────────────────────────────────────┐
│ Properties — Selected (3 items)           │
├──────────────────────────────────────────┤
│ Size:    2.4 MB                           │
│ Packed:  1.8 MB                           │
│ Files:   3                                │
│ ▶ Entries                                 │
│                              [Close]      │
└──────────────────────────────────────────┘
```

Per-entry data already available on `ArchiveEntry` — uses the new fields from L1.1. Multi-select aggregates from existing entry data.

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
| Progress for all operations (dual bar) | Taskbar progress bar, CLI progress output |
| Menu bar with all menus | Menu bar customization |
| Properties (archive + entries) with format-specific categories | Comment editing |
| Add Files dialog with update mode | Include/exclude file masks |
| Create dialog with format + compression level | Multi-volume, solid/thread/dictionary config beyond basic toggle |
| Test entries (all + selected, single entry + directory) | Recover / repair archive |
| Context menu expanded | Send To / 7-zip submenu |
| Compress CLI | CLI progress, list archives to stdout |
| Linux tray (basic D-Bus) + file dialogs (rfd) | Two-panel mode, macOS support |
| Add/Delete/Rename/Test in repository | Archive conversion, batch ops, SFX, benchmarking |
| 12 entry property C wrappers (mtime, ctime, atime, attributes, host_os, method, comment, user, group, symlink, posix_attrib, extension) | All 96 BitProperty values |
| 9 writer-setting C wrappers (method, dictionary, word, solid, volume, encryption_scope, dir_filtered, add_items, store_timestamps) | RetainDirectories, retry, format-specific advanced |
| CRC, modified, created, accessed, symlink populated on ArchiveEntry | — |
| — | Flat view, thumbnails, file type icons, drag & drop, address bar, favorites panel, wizard mode |
