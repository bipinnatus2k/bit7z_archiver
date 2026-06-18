# Core Operations Completion — Design

Full design document: `docs/superpowers/specs/2026-06-19-core-operations-completion-design.md`

## Architecture Layers (bottom-up)

```
L4: Views           (menu, toolbar, windows, context menu)
L3: ViewModels      (ArchiveVM, ProgressState, TestResult)
L2: Application     (use cases + progress channel)
L1: Adapters        (repository FFI, CLI, Linux platform)
```

## New C Wrapper Functions

### Entry Properties (12)
`bit7z_item_mtime`, `bit7z_item_ctime`, `bit7z_item_atime`, `bit7z_item_attributes`, `bit7z_item_host_os`, `bit7z_item_compression_method`, `bit7z_item_comment`, `bit7z_item_user`, `bit7z_item_group`, `bit7z_item_is_symlink`, `bit7z_item_posix_attrib`, `bit7z_item_extension`

### Writer Settings (9)
`bit7z_writer_set_compression_method`, `bit7z_writer_set_dictionary_size`, `bit7z_writer_set_word_size`, `bit7z_writer_set_solid_mode`, `bit7z_writer_set_volume_size`, `bit7z_writer_set_encryption_scope`, `bit7z_writer_add_dir_filtered`, `bit7z_writer_add_items`, `bit7z_writer_set_store_timestamps`

## Key Data Structures

- `ProgressUpdate`: dual-bar (file_current/total + items_done/total + bytes)
- `ProgressState`: Global, with is_active, is_paused, is_complete, channel receiver
- `TestResult`: passed count + Vec<TestFailure> (index, path, reason)
- `ArchiveProperties`: format, sizes, encryption, solid, volume, comment, etc.
- `ArchiveEntry`: expanded from 10 to 19 fields

## Key Design Decisions

- Progress via crossbeam channel: decouples use cases from GPUI
- Dialogs are independent GPUI windows via `cx.open_window()`
- Properties categorized (General/Time/Platform/Security/Technical) with format-specific auto-hide
- Add Files dialog controls are format-dependent (7z vs Zip vs Tar show different options)
- Test supports selected entries + entire archive; directory testing is recursive
