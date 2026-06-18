## ADDED Requirements

### Requirement: Entry property C wrappers
The system SHALL add 12 new C wrapper functions in `demo.h` to expose bit7z item properties.

| Function | bit7z source | Return type |
|---|---|---|
| `bit7z_item_mtime(ptr)` | `item.lastWriteTime()` | `uint64_t` (FILETIME) |
| `bit7z_item_ctime(ptr)` | `item.creationTime()` | `uint64_t` (FILETIME) |
| `bit7z_item_atime(ptr)` | `item.lastAccessTime()` | `uint64_t` (FILETIME) |
| `bit7z_item_attributes(ptr)` | `item.attributes()` | `uint32_t` |
| `bit7z_item_host_os(ptr)` | `item.itemProperty(HostOS)` | `uint8_t` |
| `bit7z_item_compression_method(ptr, buf, size)` | `item.itemProperty(Method)` | string into buf |
| `bit7z_item_comment(ptr, buf, size)` | `item.itemProperty(Comment)` | string into buf |
| `bit7z_item_user(ptr, buf, size)` | `item.itemProperty(User)` | string into buf |
| `bit7z_item_group(ptr, buf, size)` | `item.itemProperty(Group)` | string into buf |
| `bit7z_item_is_symlink(ptr)` | `item.isSymLink()` | `int32_t` (bool) |
| `bit7z_item_posix_attrib(ptr)` | `item.itemProperty(PosixAttrib)` | `uint32_t` |
| `bit7z_item_extension(ptr, buf, size)` | `item.extension()` | string into buf |

### Requirement: Writer setting C wrappers
The system SHALL add 9 new C wrapper functions in `demo.h` to expose bit7z writer settings.

| Function | bit7z source |
|---|---|
| `bit7z_writer_set_compression_method(w, method)` | `setCompressionMethod()` |
| `bit7z_writer_set_dictionary_size(w, bytes)` | `setDictionarySize()` |
| `bit7z_writer_set_word_size(w, bytes)` | `setWordSize()` |
| `bit7z_writer_set_solid_mode(w, solid)` | `setSolidMode()` |
| `bit7z_writer_set_volume_size(w, bytes)` | `setVolumeSize()` |
| `bit7z_writer_set_encryption_scope(w, scope)` | `setPassword(pwd, EncryptionScope)` |
| `bit7z_writer_add_dir_filtered(w, dir, filter, policy, recursive)` | `addFiles(dir, filter, policy, recursive)` |
| `bit7z_writer_add_items(w, paths, archive_paths, count)` | `addItems(vector<pair<fsPath, archivePath>>)` |
| `bit7z_writer_set_store_timestamps(w, modified, created, accessed)` | `setStoreLastWriteTime/ setStoreCreationTime/ setStoreLastAccessTime` |

### Requirement: autocxx bindings regenerated
The system SHALL declare all 21 new functions in `src/ffi.rs` and verify `cargo build` succeeds.

### Requirement: Safe Rust wrappers
The system SHALL add corresponding method on safe `Item` and `Writer` structs in `src/adapters/bit7z/mod.rs` for all new C wrappers.

#### Scenario: Safe method for compression method
- **WHEN** calling `item.compression_method()`
- **THEN** returns `Option<String>` from the C++ `itemProperty(Method)` call
