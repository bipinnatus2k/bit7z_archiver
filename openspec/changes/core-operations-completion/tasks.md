# Tasks — Core Operations Completion

## 1. FFI: C Wrapper Functions

- [ ] 1.1 Add 12 entry property wrappers to `src/demo.h`: `bit7z_item_mtime`, `bit7z_item_ctime`, `bit7z_item_atime`, `bit7z_item_attributes`, `bit7z_item_host_os`, `bit7z_item_compression_method`, `bit7z_item_comment`, `bit7z_item_user`, `bit7z_item_group`, `bit7z_item_is_symlink`, `bit7z_item_posix_attrib`, `bit7z_item_extension`
- [ ] 1.2 Add 9 writer-setting wrappers to `src/demo.h`: `bit7z_writer_set_compression_method`, `bit7z_writer_set_dictionary_size`, `bit7z_writer_set_word_size`, `bit7z_writer_set_solid_mode`, `bit7z_writer_set_volume_size`, `bit7z_writer_set_encryption_scope`, `bit7z_writer_add_dir_filtered`, `bit7z_writer_add_items`, `bit7z_writer_set_store_timestamps` (modified/created/accessed bool params)
- [ ] 1.3 Regenerate autocxx bindings in `src/ffi.rs` — declare all 21 new functions for code generation. Verify with `cargo build`
- [ ] 1.4 Add safe Rust `Item` accessors in `src/adapters/bit7z/mod.rs` for all 12 new item properties
- [ ] 1.5 Add safe Rust `Writer` methods in `src/adapters/bit7z/mod.rs` for all 9 new writer settings. Add `WriterCompressionMethod` enum, `EncryptionScope` enum
- [ ] 1.6 Expand `ArchiveEntry` in `src/domain/archive.rs`: populate dead fields (crc, modified, is_symlink from FFI), add new Option fields (created, accessed, attributes, posix_attrib, host_os, compression_method, comment, user, group)

## 2. Bit7zRepository Wiring

- [ ] 2.1 Implement `add()` in `src/adapters/repository.rs`: open Writer in update mode, call add_files/add_dir, compress with progress callbacks
- [ ] 2.2 Implement `delete()` in `src/adapters/repository.rs`: open Editor, call delete for each index, apply
- [ ] 2.3 Implement `rename()` in `src/adapters/repository.rs`: open Editor, call rename, apply
- [ ] 2.4 Implement `test()` in `src/adapters/repository.rs`: call `bit7z_reader_test()` for archive-wide; for per-entry, extract-to-memory + CRC compare with recursive directory expansion
- [ ] 2.5 Implement `get_archive_properties()` returning `ArchiveProperties` struct. Add `ArchiveProperties` to `src/domain/archive.rs`
- [ ] 2.6 Implement `set_progress_sender()` — store `Sender<ProgressUpdate>` for use by add/delete/test/extract methods
- [ ] 2.7 Populate all new `ArchiveEntry` fields in `list_page()` and `list_directory()` — call new item accessors, map to domain struct

## 3. Application Layer

- [ ] 3.1 Define `ProgressUpdate` struct in `src/application/mod.rs` or new `src/application/progress.rs`
- [ ] 3.2 Add `Sender<ProgressUpdate>` parameter to `ExtractEntriesUseCase::execute()`, pass to repo
- [ ] 3.3 Add `Sender<ProgressUpdate>` parameter to `CreateArchiveUseCase::execute()`, wire through compress callback
- [ ] 3.4 Add `Sender<ProgressUpdate>` parameter to `AddToArchiveUseCase::execute()`, validate paths, delegate to repo
- [ ] 3.5 Add `Sender<ProgressUpdate>` parameter to `DeleteEntriesUseCase::execute()`
- [ ] 3.6 Implement `TestEntriesUseCase` — accepts `Option<Vec<usize>>` (None = entire archive), returns `TestResult { passed, failed: Vec<TestFailure> }`. Add `TestResult`, `TestFailure`, `TestFailureReason` types
- [ ] 3.7 Implement `CalculateChecksumUseCase` — temp-extract selected non-dir entries, compute hash per algorithm, return `Vec<(path, hex_string)>`
- [ ] 3.8 Implement `OpenEntryUseCase` — temp-extract single entry, shell-execute with OS association
- [ ] 3.9 Implement `NewFolderUseCase` — create empty dir entry via Editor
- [ ] 3.10 Implement `NewFileUseCase` — create empty temp file, open editor, on save add to archive
- [ ] 3.11 Complete Compress CLI in `src/cli.rs`: enumerate input paths (recursive for dirs), create archive, add files, print summary

## 4. ViewModels

- [ ] 4.1 Register `ProgressState` as global in `src/gui.rs` (`cx.set_global(ProgressState::default())`)
- [ ] 4.2 Add `is_paused: bool` field to `ProgressState` in `src/adapters/view_models/progress_vm.rs`
- [ ] 4.3 Implement `ProgressState::start(rx)` — spawn background task reading channel, updating fields, detecting close/error
- [ ] 4.4 Implement `ProgressState::pause()` / `resume()` — toggle AtomicBool, signal worker thread (reuse existing mechanism from `src/adapters/bit7z/worker.rs`)
- [ ] 4.5 Implement `ArchiveVM::delete_selected()` in `src/adapters/view_models/archive_vm.rs`
- [ ] 4.6 Implement `ArchiveVM::add_files()` — open file/folder picker, call AddToArchiveUseCase, refresh
- [ ] 4.7 Implement `ArchiveVM::rename_entry(index)` — open rename input, call RenameEntryUseCase, refresh
- [ ] 4.8 Implement `ArchiveVM::test_selected()` — call TestEntriesUseCase, open results window on failures

## 5. Views — Menu & Toolbar

- [ ] 5.1 Create `src/adapters/views/menu.rs` — render menu bar with File/Edit/View/Tools/Favorites/Help. Each item dispatches same events as toolbar/context menu
- [ ] 5.2 Integrate menu bar into `RootView` render in `src/adapters/views/root.rs` — above toolbar
- [ ] 5.3 Add keyboard shortcut dispatch in `RootView` for all menu items (Ctrl+O, Ctrl+N, Ctrl+T, Enter, Ctrl+V, F4, Ctrl+A, Del, F2, F5, Alt+Enter, Ctrl+Shift+N)
- [ ] 5.4 Update `Toolbar` in `src/adapters/views/toolbar.rs`: add `Add` button, add `Settings` gear button (right-aligned)

## 6. Views — Windows

- [ ] 6.1 Wire Settings window: add trigger from `Tools → Settings` menu + toolbar gear → `cx.open_window(SettingsDialog)`. Remove unused `settings_dialog` field from `RootView`. File: `src/adapters/views/dialogs/settings.rs` + `root.rs`
- [ ] 6.2 Wire Progress window: dual-bar layout (per-file + overall), Pause/Resume, Hide-to-tray, Cancel. Read `ProgressState::global(cx)` each frame. File: `src/adapters/views/dialogs/progress.rs`
- [ ] 6.3 Polish Create dialog: replace static text with real GPUI controls — format `PickList`, compression level slider, advanced section (collapsed: method, dictionary, word, solid, volume, threads). File: `src/adapters/views/dialogs/create.rs`
- [ ] 6.4 Polish Extract dialog: browse button → `pick_folder()`, overwrite mode dropdown (Ask/Overwrite/Skip/Rename), keep-broken-files checkbox. File: `src/adapters/views/dialogs/extract.rs`
- [ ] 6.5 Create Add Files dialog: full window with format dropdown, file/folder picker with wildcard filter + include/exclude, compression section, advanced section, encryption section. File: new `src/adapters/views/dialogs/add_files.rs`
- [ ] 6.6 Create Properties — Archive window: two-column layout (General + Advanced), close button. File: new `src/adapters/views/dialogs/properties_archive.rs`
- [ ] 6.7 Create Properties — Entries window: single-entry full detail (categorized: General/Time/Platform/Security/Technical), multi-entry aggregate totals + collapsible DataTable. File: new `src/adapters/views/dialogs/properties_entries.rs`
- [ ] 6.8 Create Test Results window: "X passed, Y failed" header, collapsed failed list (expandable), close button. File: new `src/adapters/views/dialogs/test_results.rs`
- [ ] 6.9 Create Checksum result popup: small window showing CRC32/MD5/SHA1/SHA256 hex strings for selected entries. File: new `src/adapters/views/dialogs/checksum_result.rs`

## 7. Views — Context Menu

- [ ] 7.1 Expand context menu in `src/adapters/views/archive_file_list.rs`: Open, View, Edit, Extract, Add, Test Selected/All, Checksum submenu, Rename, Delete, New Folder, New File, Properties, Select All, Clear, Refresh
- [ ] 7.2 Add empty-space context menu: New Folder, New File, Select All, Refresh
- [ ] 7.3 Wire all context menu actions to corresponding use cases / ViewModel methods

## 8. Linux Platform

- [ ] 8.1 Implement real tray icon in `src/adapters/tray/linux.rs` using `zbus` crate: D-Bus StatusNotifierItem with icon, Activate (restore window), context menu (Open/Exit)
- [ ] 8.2 Add tray tooltip updates during progress operations: "bit7z — Extracting 42%"
- [ ] 8.3 Replace file dialog stubs in `src/adapters/platform.rs` with `rfd` crate: `pick_archive_file()` (multi-select), `pick_folder()`

## 9. Tests

- [ ] 9.1 Create test fixtures in `tests/fixtures/`: `basic.7z` (3 text files + 1 subdir), `basic.zip` (same content), `corrupted.7z` (1 good + 1 CRC-mismatched entry), `encrypted.7z` (password-protected), `empty.7z`
- [ ] 9.2 Integration tests: `Bit7zRepository` add/delete/rename against real test archives
- [ ] 9.3 Integration tests: `TestEntriesUseCase` — known-good (all pass), corrupted (some fail), directory recursion
- [ ] 9.4 Integration tests: Compress CLI round-trip (compress dir → extract → compare)
- [ ] 9.5 Unit tests: `ProgressState` — send updates, verify fields, verify completion on sender drop, verify error capture
- [ ] 9.6 Unit tests: `TestResult` aggregation logic
- [ ] 9.7 Unit tests: `ArchiveProperties` parsing from test archives
