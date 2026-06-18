## Why

bit7z_archiver has ~9 subsystems that are partially built but orphaned — FFI functions exist without repository wiring, UI windows are fully rendered but never instantiated, CLI commands are stubs. This change completes all of them, bringing the application to a functional baseline comparable to WinRAR/7-Zip for core operations.

## What Changes

- **Wire add/delete/rename/test**: `Bit7zRepository` currently returns `UnsupportedOperation` for all four. The C++ bridge and safe Rust wrappers already exist — only wiring needed. Test now supports both entire archive and selected entries (files and directories, recursively).
- **Progress for all operations**: Long-running ops (extract, create, add, test) now report progress through a dual-bar progress window (per-file + overall) with pause/resume and hide-to-tray. `ProgressState` registered as a GPUI Global.
- **Menu bar**: Full File/Edit/View/Tools/Favorites/Help menu with keyboard shortcuts, replacing the standalone toolbar.
- **Toolbar expansion**: Add button, Settings (gear) button.
- **Context menu expansion**: Open, View, Edit, Checksum (CRC32/MD5/SHA1/SHA256), Test (Selected/All), New Folder, New File, Properties.
- **Settings window**: Wired to toolbar and menu — was fully built but never instantiated.
- **Progress window**: Dual progress bars (per-file bytes + overall items/bytes), pause/resume, hide-to-tray with tray tooltip showing progress percentage.
- **Create dialog polish**: Real format dropdown and compression level slider replacing static text stubs. Advanced section with method, dictionary, solid, volume, threads.
- **Extract dialog polish**: Browse button for destination, overwrite mode dropdown, keep-broken-files checkbox.
- **Add Files dialog**: Full bit7z capabilities — format dropdown, file/folder picker with wildcard filter and include/exclude policy, compression method selector, dictionary size, word size, solid mode, volume splitting, timestamps, encryption scope.
- **Properties windows**: Archive properties (format, size, ratio, encryption, solid, multi-volume, comment, etc.) and entry properties (categorized: General/Time/Platform/Security/Technical) with format-specific auto-hide.
- **Test Results window**: Per-entry pass/fail with collapsed failed list.
- **Compress CLI**: Replaced placeholder with full file enumeration, archive creation, and add-files flow. Also list, checksum, and new-folder subcommands.
- **21 new C wrapper functions**: 12 entry property wrappers (mtime, ctime, atime, attrib, host_os, method, comment, user, group, symlink, posix_attrib, extension) + 9 writer-setting wrappers (method, dictionary, word, solid, volume, encryption_scope, dir_filtered, add_items, store_timestamps).
- **ArchiveEntry expansion**: Populate dead fields (crc, modified, is_symlink) + new Option fields (created, accessed, attributes, posix_attrib, host_os, compression_method, comment, user, group).
- **New use cases**: CalculateChecksum, OpenEntry, NewFolder, NewFile.
- **Linux platform**: Tray icon via zbus D-Bus StatusNotifierItem (replaces stub). File dialogs via rfd crate.

## Capabilities

### New Capabilities
- `core-operations`: Add files to existing archives, delete entries, rename entries, test archive integrity — all with progress reporting
- `progress-system`: Unified progress channel across all operations. Dual-bar progress window with pause/resume and hide-to-tray
- `menu-system`: Full menu bar with keyboard shortcuts driving all operations
- `properties-windows`: Per-archive and per-entry properties with format-specific categories
- `checksum-calculator`: CRC32, MD5, SHA1, SHA256 computation on selected entries
- `entry-edit`: Open entries with associated program, edit and update in archive, new folder/file creation
- `linux-refinement`: Real tray icon via D-Bus, native file dialogs via rfd

### Modified Capabilities
- `archive-modification`: Now fully functional (was UnsupportedOperation stubs)
- `archive-testing`: Now supports per-entry and per-directory testing with aggregated results
- `create-dialog`: Real interactive controls replacing static text
- `extract-dialog`: Browse destination, overwrite mode, keep-broken-files
- `cli-compress`: Fully functional (was placeholder)
- `archive-entry-properties`: Expanded from 7 fields to 19, all populated from bit7z
