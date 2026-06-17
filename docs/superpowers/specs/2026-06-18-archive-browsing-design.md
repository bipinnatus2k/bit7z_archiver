# Archive Browsing Design

## Problem

Current archive browsing loads all entries upfront via `list_page()` which:

- Calls 6+ FFI functions per item (path, name, size, packed_size, is_dir, is_encrypted), making even small archives slow
- Stores all entries in a `VecDeque` with no lazy loading beyond the initial 200
- `load_page` is dead code — never triggered by any view
- Folder aggregation (`compute_level_entries`) iterates all cached entries
- `DataTable` renders all rows (no virtual scrolling used), causing UI freezes on large datasets

## Solution Overview

Three prongs:

1. **On-demand directory loading** — new `list_directory` method on `ArchiveRepository` backed by bit7z `itemsMatching(pattern)`, loading only items for the current directory path
2. **Batched C bridge** — single FFI call returns all properties for matching items, eliminating 6×N cross-language calls
3. **Directory cache** — `HashMap<String, Vec<ArchiveEntry>>` keyed by archive path, clearing on close

Virtual scrolling is already supported by gpui-component's `DataTable` (uses GPUI `uniform_list`), requiring no changes.

## Architecture

```
ArchiveFileList (DataTable, virtual scroll)
      |
      | displayed_entries()
      v
ArchiveViewModel
  - current_path: String         ("dir/subdir/")
  - directory_cache: HashMap<String, Vec<LevelEntry>>
  - level_entries: Vec<LevelEntry>  (current directory, sorted/filtered)
      |
      | list_directory(handle, path)
      v
ArchiveRepository (trait)
  - fn list_directory(&self, handle: &ArchiveHandle, path: &str) -> Result<Vec<ArchiveEntry>, AE>
      |
      | C bridge: bit7z_reader_list_directory(reader, pattern)
      v
C++ Bridge (demo.h)
  - calls reader.itemsMatching(pattern)
  - returns flat array of {path, size, packed_size, is_dir, is_encrypted}
```

## Components

### 1. `ArchiveRepository` trait extension

Add a new method:

```rust
fn list_directory(
    &self,
    archive: &ArchiveHandle,
    path: &str,    // "dir/subdir/" or "" for root
) -> Result<Vec<ArchiveEntry>, ArchiveError>;
```

Returns only the direct children of `path` — items whose relative path starts with `path` and has no further `/` after the prefix.

**Path format convention:** `path` parameter is WITHOUT leading slash and WITH trailing slash for non-root paths. Root is empty string `""`. Examples:
- Root → `""` → pattern `"*"`
- `dir/` → pattern `"dir/*"`
- `dir/subdir/` → pattern `"dir/subdir/*"`

### 2. C bridge (`demo.h`)

To maximize autocxx compatibility, use **opaque handle + accessor pattern** rather than struct arrays.

```cpp
// Opaque handle to a cached itemsMatching result.
// Created once, read via accessors, freed in one call.
// Thread-safety: not required (only one thread accesses at a time).

void* bit7z_reader_list_directory(void* reader, const char* path);
// Returns opaque handle, or nullptr on error.

uint32_t bit7z_item_list_count(void* list);
const char* bit7z_item_list_path(void* list, uint32_t index);
uint64_t bit7z_item_list_size(void* list, uint32_t index);
uint64_t bit7z_item_list_packed_size(void* list, uint32_t index);
int32_t bit7z_item_list_is_dir(void* list, uint32_t index);
int32_t bit7z_item_list_is_encrypted(void* list, uint32_t index);

void bit7z_item_list_free(void* list);
```

**Implementation detail:** `bit7z_reader_list_directory` calls `itemsMatching(pattern)` **once**, filters to direct children, and stores the filtered `vector<BitArchiveItemInfo>` in a heap-allocated wrapper. The accessors read from this cached vector — no repeated `items()` calls.

This reduces the current N × 6 FFI calls to **1 create + N × 5 accessors + 1 free** = 5N + 2 (saves the `bit7z_item_name` call since `name` is derived from `path` on the Rust side via `rsplit('/')`).

Autocxx compatibility: all parameters are `void*` or primitive types — autocxx can generate bindings for all of them via `generate!("bit7z_reader_list_directory")` etc.

### 3. `Bit7zRepository` implementation

```rust
fn list_directory(&self, handle: &ArchiveHandle, path: &str) -> Result<Vec<ArchiveEntry>, ArchiveError> {
    let c_path = CString::new(path).map_err(|_| ArchiveError::InvalidPath)?;
    let list = unsafe { ffi::bit7z_reader_list_directory(handle.raw(), c_path.as_ptr()) };
    if list.is_null() { return Err(ArchiveError::Unknown); }

    let count = unsafe { ffi::bit7z_item_list_count(list) };
    let mut entries = Vec::with_capacity(count as usize);
    for i in 0..count {
        let path = unsafe { CStr::from_ptr(ffi::bit7z_item_list_path(list, i)) }
            .to_string_lossy().into_owned();
        let name = path.rsplit('/').next().unwrap_or(&path).to_string();
        entries.push(ArchiveEntry {
            path,
            name,
            size: unsafe { ffi::bit7z_item_list_size(list, i) },
            compressed_size: unsafe { ffi::bit7z_item_list_packed_size(list, i) },
            is_directory: unsafe { ffi::bit7z_item_list_is_dir(list, i) != 0 },
            is_encrypted: unsafe { ffi::bit7z_item_list_is_encrypted(list, i) != 0 },
            is_symlink: false,
            modified: None,
            crc: None,
        });
    }
    unsafe { ffi::bit7z_item_list_free(list); }
    Ok(entries)
}
```

### 4. `ArchiveViewModel` changes

**New/Changed fields:**

```rust
directory_cache: HashMap<String, Vec<ArchiveEntry>>,
directory_loaded: bool,  // whether current_path has been loaded
```

**`navigate_into(dir_name)`:**
  - Push `current_path` onto `path_history`
  - Append `dir_name/` to `current_path`
  - Run `load_current_directory()` in background
  - Set status to Loading while fetching

**`navigate_up()` / `navigate_root()`:**
  - Pop from `path_history` (or clear it for root)
  - `current_path = parent` (or `""`)
  - `level_entries` = cached entries for that path
  - No additional repo call

**`load_current_directory()`:**
  - Check `directory_cache` for `current_path`
  - Hit → use cached entries directly
  - Miss → call `repo.list_directory(handle, current_path)` → cache result
  - Apply current filter/sort → set `level_entries`
  - Set status to Ready

**`open_archive()` changes:**
  - Open archive, get properties (unchanged)
  - Call `load_current_directory()` instead of `list_page()`
  - Set `directory_cache = HashMap::new()`

**`close_archive()`:**
  - Clear `directory_cache`
  - Close handle (unchanged)

**`compute_level_entries()` removal:**
  - No longer needed — entries from `list_directory` are already at the right level
  - Keep `re_filter()` for text search and sort

### 5. Cache strategy

- `HashMap<String, Vec<ArchiveEntry>>` — key is `current_path`
- No LRU eviction — typical navigation depth is < 20 levels
- Cleared on `close_archive()`
- `navigate_up()` is guaranteed hit (just navigated through)
- `navigate_into()` may be miss on first visit

### 6. Sidebar directory tree (`ArchiveBrowser`)

- `cached_folders` now derived from the current directory's entries
- Shows subdirectories of the current path (not all archive folders)
- Matches 7-Zip/WinRAR behavior — sidebar shows current directory tree

### 7. Virtual scrolling

Already supported by `DataTable` → GPUI `uniform_list`. No code changes needed. The `FileTableDelegate.rows_count()` returns `vm.displayed_entries().len()` which may now be smaller (directory-level) or large (root with many entries). In either case, `uniform_list` only renders visible rows.

### 8. Error handling

- `list_directory` returns `ArchiveError::NotFound` if the path doesn't exist in the archive
- `navigate_into` shows error in status bar and stays on current directory
- Cache miss during navigation → background load → UI stays responsive

### 9. Testing

- Mock repository `list_directory` returns pre-configured entries filtered by path
- VM unit tests: navigate_into triggers load, navigate_up uses cache, cache clear on close
- Test with archives containing deep directory structures

## Future considerations

- **Pagination within directory** — if a directory has > 10,000 items, `itemsMatching` returns all at once. Future optimization: add offset/limit to `list_directory`.
- **FilterCallback** — bit7z v4.1 has `FilterCallback` for per-item filtering. Could be used to implement server-side filter.
- **Path autocomplete** — `findByName` can be used for quick file lookup.
