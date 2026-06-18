#pragma once

#include "bit7z/bitformat.hpp"
#include "bit7z/bit7zlibrary.hpp"
#include "bit7z/bitinputarchive.hpp"
#include "bit7z/bitarchivereader.hpp"
#include "bit7z/bitarchivewriter.hpp"
#include "bit7z/bitarchiveeditor.hpp"
#include "bit7z/bitarchiveiteminfo.hpp"
#include "bit7z/biterror.hpp"
#include "bit7z/bittypes.hpp"
#include <bit7z/bitwindows.hpp>
#include <sys/stat.h>
#include <memory>

// Type aliases for ergonomic Rust naming
using ArchiveFormatFeatures = bit7z::FormatFeatures;
using ArchiveInFormat = bit7z::BitInFormat;
using ArchiveInOutFormat = bit7z::BitInOutFormat;
using ArchiveItem = bit7z::BitArchiveItem;
using ArchiveItemInfo = bit7z::BitArchiveItemInfo;
using ArchiveProperties = bit7z::BitProperty;
using ArchivePropVariant = bit7z::BitPropVariant;
using ArchiveItemOffset = bit7z::BitArchiveItemOffset;
using ArchiveLibrary = bit7z::Bit7zLibrary;
using ArchiveStartOffset = bit7z::ArchiveStartOffset;
using InputArchive = bit7z::BitInputArchive;
using ArchiveReader = bit7z::BitArchiveReader;
using ArchiveWriter = bit7z::BitArchiveWriter;
using buffer_t = bit7z::buffer_t;
using time_type = bit7z::time_type;
using ArchiveError = bit7z::BitError;

// ===== Test function =====
inline int multiply(int a, int b) { return a * b; }

// ===== Library wrappers =====
inline void* bit7z_create_library(const char* dll_path) {
    try {
        auto* lib = new bit7z::Bit7zLibrary(dll_path ? std::string(dll_path) : "");
        return static_cast<void*>(lib);
    } catch (...) { return nullptr; }
}
inline void bit7z_destroy_library(void* lib) {
    delete static_cast<bit7z::Bit7zLibrary*>(lib);
}

// ===== Reader wrappers =====

inline void* bit7z_reader_open(void* lib_ptr, const char* path, const char* password) {
    try {
        auto& lib = *static_cast<bit7z::Bit7zLibrary*>(lib_ptr);
        std::string p(path ? path : "");
        auto dot = p.find_last_of('.');
        std::string ext;
        if (dot != std::string::npos) {
            ext = p.substr(dot);
            for (auto& c : ext) c = (char)tolower(c);
        }
        const bit7z::BitInFormat& fmt =
            (ext == ".zip")  ? static_cast<const bit7z::BitInFormat&>(bit7z::BitFormat::Zip) :
            (ext == ".tar")  ? static_cast<const bit7z::BitInFormat&>(bit7z::BitFormat::Tar) :
            (ext == ".gz" || ext == ".tgz")  ? static_cast<const bit7z::BitInFormat&>(bit7z::BitFormat::GZip) :
            (ext == ".bz2" || ext == ".tbz") ? static_cast<const bit7z::BitInFormat&>(bit7z::BitFormat::BZip2) :
            (ext == ".xz"  || ext == ".txz") ? static_cast<const bit7z::BitInFormat&>(bit7z::BitFormat::Xz) :
            (ext == ".wim") ? static_cast<const bit7z::BitInFormat&>(bit7z::BitFormat::Wim) :
            (ext == ".rar") ? static_cast<const bit7z::BitInFormat&>(bit7z::BitFormat::Rar) :
            static_cast<const bit7z::BitInFormat&>(bit7z::BitFormat::SevenZip);
        auto* reader = new bit7z::BitArchiveReader(lib,
            bit7z::tstring(path ? path : ""), fmt,
            bit7z::tstring(password ? password : ""));
        return static_cast<void*>(reader);
    } catch (...) { return nullptr; }
}
inline void bit7z_reader_close(void* reader_ptr) {
    delete static_cast<bit7z::BitArchiveReader*>(reader_ptr);
}
inline uint32_t bit7z_reader_item_count(void* reader_ptr) {
    try {
        return static_cast<bit7z::BitArchiveReader*>(reader_ptr)->itemsCount();
    } catch (...) { return 0; }
}

// ===== Item accessor wrappers (static buffers, single-threaded) =====
inline const char* bit7z_item_path(void* reader_ptr, uint32_t index) {
    try {
        static std::string s;
        s = static_cast<bit7z::BitArchiveReader*>(reader_ptr)->items()[index].path();
        return s.c_str();
    } catch (...) { return ""; }
}
inline const char* bit7z_item_name(void* reader_ptr, uint32_t index) {
    try {
        static std::string s;
        s = static_cast<bit7z::BitArchiveReader*>(reader_ptr)->items()[index].name();
        return s.c_str();
    } catch (...) { return ""; }
}
inline uint64_t bit7z_item_size(void* reader_ptr, uint32_t index) {
    try {
        return static_cast<bit7z::BitArchiveReader*>(reader_ptr)->items()[index].size();
    } catch (...) { return 0; }
}
inline uint64_t bit7z_item_packed_size(void* reader_ptr, uint32_t index) {
    try {
        return static_cast<bit7z::BitArchiveReader*>(reader_ptr)->items()[index].packSize();
    } catch (...) { return 0; }
}
inline int32_t bit7z_item_is_dir(void* reader_ptr, uint32_t index) {
    try {
        return static_cast<bit7z::BitArchiveReader*>(reader_ptr)->items()[index].isDir() ? 1 : 0;
    } catch (...) { return 0; }
}
inline int32_t bit7z_item_is_encrypted(void* reader_ptr, uint32_t index) {
    try {
        return static_cast<bit7z::BitArchiveReader*>(reader_ptr)->items()[index].isEncrypted() ? 1 : 0;
    } catch (...) { return 0; }
}
inline uint32_t bit7z_item_crc(void* reader_ptr, uint32_t index) {
    try {
        return static_cast<bit7z::BitArchiveReader*>(reader_ptr)->items()[index].crc();
    } catch (...) { return 0; }
}

// ===== Extract wrappers =====
inline int32_t bit7z_reader_extract_to(void* reader_ptr, const uint32_t* indices, uint32_t count, const char* dest_path) {
    try {
        auto& reader = *static_cast<bit7z::BitArchiveReader*>(reader_ptr);
        std::vector<uint32_t> idxs(indices, indices + count);
        reader.extractTo(bit7z::tstring(dest_path ? dest_path : ""), idxs);
        return 0;
    } catch (...) { return -1; }
}

inline int32_t bit7z_reader_extract_item_to_buffer(void* reader_ptr, uint32_t index, void** out_data, int64_t* out_size) {
    try {
        auto& reader = *static_cast<bit7z::BitArchiveReader*>(reader_ptr);
        bit7z::buffer_t buf;
        reader.extractTo(buf, index);
        *out_size = static_cast<int64_t>(buf.size());
        if (buf.empty()) { *out_data = nullptr; return -1; }
        auto* data = new unsigned char[buf.size()];
        std::copy(buf.begin(), buf.end(), data);
        *out_data = data;
        return 0;
    } catch (...) { *out_data = nullptr; *out_size = 0; return -1; }
}
inline void bit7z_reader_free_buffer(void* data) {
    delete[] static_cast<unsigned char*>(data);
}

inline int64_t bit7z_reader_extract_item_size(void* reader_ptr, uint32_t index) {
    try {
        auto& reader = *static_cast<bit7z::BitArchiveReader*>(reader_ptr);
        bit7z::buffer_t buf;
        reader.extractTo(buf, index);
        return static_cast<int64_t>(buf.size());
    } catch (...) { return -1; }
}
inline void* bit7z_reader_extract_item_data(void* reader_ptr, uint32_t index) {
    try {
        auto& reader = *static_cast<bit7z::BitArchiveReader*>(reader_ptr);
        bit7z::buffer_t buf;
        reader.extractTo(buf, index);
        if (buf.empty()) return nullptr;
        auto* data = new unsigned char[buf.size()];
        std::copy(buf.begin(), buf.end(), data);
        return data;
    } catch (...) { return nullptr; }
}

// ===== Test archive integrity =====

struct TestResult {
    bool all_ok;
    uint32_t total;
    uint32_t failed_count;
    std::vector<std::string> failed_paths;
    std::vector<std::string> failed_errors;
};

inline void* bit7z_reader_test(void* reader_ptr) {
    try {
        auto& reader = *static_cast<bit7z::BitArchiveReader*>(reader_ptr);
        reader.test();  // throws BitException if any item fails
        // If no exception, all items passed
        auto* result = new TestResult();
        result->all_ok = true;
        result->total = reader.itemsCount();
        result->failed_count = 0;
        return static_cast<void*>(result);
    } catch (const bit7z::BitException& e) {
        auto* result = new TestResult();
        result->all_ok = false;
        result->total = 0;
        result->failed_count = 1;
        result->failed_paths.push_back("");
        result->failed_errors.push_back(e.what());
        return static_cast<void*>(result);
    } catch (...) {
        return nullptr;
    }
}

inline uint32_t bit7z_test_result_total(void* result_ptr) {
    return static_cast<TestResult*>(result_ptr)->total;
}

inline uint32_t bit7z_test_result_failed_count(void* result_ptr) {
    return static_cast<TestResult*>(result_ptr)->failed_count;
}

inline int32_t bit7z_test_result_all_ok(void* result_ptr) {
    return static_cast<TestResult*>(result_ptr)->all_ok ? 1 : 0;
}

inline const char* bit7z_test_result_error(void* result_ptr) {
    auto* r = static_cast<TestResult*>(result_ptr);
    if (r->failed_errors.empty()) return "";
    return r->failed_errors[0].c_str();
}

inline void bit7z_test_result_free(void* result_ptr) {
    delete static_cast<TestResult*>(result_ptr);
}

// ===== Encryption detection =====

inline int32_t bit7z_is_header_encrypted(void* lib_ptr, const char* path) {
    try {
        auto& lib = *static_cast<bit7z::Bit7zLibrary*>(lib_ptr);
        return bit7z::BitArchiveReader::isHeaderEncrypted(lib, path ? path : "") ? 1 : 0;
    } catch (...) { return 0; }
}

inline int32_t bit7z_is_encrypted(void* lib_ptr, const char* path) {
    try {
        auto& lib = *static_cast<bit7z::Bit7zLibrary*>(lib_ptr);
        return bit7z::BitArchiveReader::isEncrypted(lib, path ? path : "") ? 1 : 0;
    } catch (...) { return 0; }
}

inline int32_t bit7z_reader_has_encrypted_items(void* reader_ptr) {
    try {
        auto& reader = *static_cast<bit7z::BitArchiveReader*>(reader_ptr);
        return reader.hasEncryptedItems() ? 1 : 0;
    } catch (...) { return 0; }
}

// ===== Directory listing (opaque handle + accessors) =====

struct ItemList {
    std::vector<bit7z::BitArchiveItemInfo> items;
    std::vector<std::string> paths;  // cached path strings (path() returns by value)
    std::string prefix;
};

inline void* bit7z_reader_list_directory(void* reader_ptr, const char* path) {
    try {
        auto& reader = *static_cast<bit7z::BitArchiveReader*>(reader_ptr);
        std::string prefix = path ? path : "";
        size_t plen = prefix.size();

        // Build wildcard pattern. On Windows, 7-Zip uses \ as separator
        // in returned paths, so the pattern must use \ too.
        std::string pattern = prefix + "*";
#ifdef _WIN32
        for (auto& c : pattern) if (c == '/') c = '\\';
#endif
        auto matched = reader.itemsMatching(pattern);

        auto* list = new ItemList();
        list->prefix = prefix;

        for (auto& item : matched) {
            std::string itemPath = item.path();
            // Normalise to / for consistent filtering
            for (auto& c : itemPath) if (c == '\\') c = '/';

            // Skip items whose relative tail still contains '/'
            // (shouldn't happen with itemsMatching but be safe)
            auto tail = itemPath.substr(plen);
            if (tail.empty()) continue;
            auto slashPos = tail.find('/');
            if (slashPos != std::string::npos && slashPos != tail.size() - 1) continue;

            list->paths.push_back(itemPath);
            list->items.push_back(std::move(item));
        }
        return static_cast<void*>(list);
    } catch (...) { return nullptr; }
}

inline uint32_t bit7z_item_list_count(void* list_ptr) {
    return static_cast<uint32_t>(static_cast<ItemList*>(list_ptr)->items.size());
}

inline uint32_t bit7z_item_list_index(void* list_ptr, uint32_t index) {
    return static_cast<ItemList*>(list_ptr)->items[index].index();
}

inline const char* bit7z_item_list_path(void* list_ptr, uint32_t index) {
    return static_cast<ItemList*>(list_ptr)->paths[index].c_str();
}

inline uint64_t bit7z_item_list_size(void* list_ptr, uint32_t index) {
    return static_cast<ItemList*>(list_ptr)->items[index].size();
}

inline uint64_t bit7z_item_list_packed_size(void* list_ptr, uint32_t index) {
    return static_cast<ItemList*>(list_ptr)->items[index].packSize();
}

inline int32_t bit7z_item_list_is_dir(void* list_ptr, uint32_t index) {
    return static_cast<ItemList*>(list_ptr)->items[index].isDir() ? 1 : 0;
}

inline int32_t bit7z_item_list_is_encrypted(void* list_ptr, uint32_t index) {
    return static_cast<ItemList*>(list_ptr)->items[index].isEncrypted() ? 1 : 0;
}

inline void bit7z_item_list_free(void* list_ptr) {
    delete static_cast<ItemList*>(list_ptr);
}

// ===== Writer wrappers =====

enum WriterFormat : int {
    BIT7Z_FORMAT_7Z = 0,
    BIT7Z_FORMAT_ZIP = 1,
    BIT7Z_FORMAT_TAR = 2,
    BIT7Z_FORMAT_GZIP = 3,
    BIT7Z_FORMAT_BZIP2 = 4,
    BIT7Z_FORMAT_XZ = 5,
    BIT7Z_FORMAT_WIM = 6,
};

enum WriterCompressionLevel : int {
    BIT7Z_COMPRESS_NONE = 0,
    BIT7Z_COMPRESS_FASTEST = 1,
    BIT7Z_COMPRESS_FAST = 2,
    BIT7Z_COMPRESS_NORMAL = 3,
    BIT7Z_COMPRESS_MAX = 4,
    BIT7Z_COMPRESS_ULTRA = 5,
};

extern "C" void* bit7z_writer_create(void* lib_ptr, int format) {
    try {
        auto& lib = *static_cast<bit7z::Bit7zLibrary*>(lib_ptr);
        const auto& fmt = [format]() -> const bit7z::BitInOutFormat& {
            switch (format) {
                case 1: return bit7z::BitFormat::Zip;
                case 2: return bit7z::BitFormat::Tar;
                case 3: return bit7z::BitFormat::GZip;
                case 4: return bit7z::BitFormat::BZip2;
                case 5: return bit7z::BitFormat::Xz;
                case 6: return bit7z::BitFormat::Wim;
                default: return bit7z::BitFormat::SevenZip;
            }
        }();
        auto* writer = new bit7z::BitArchiveWriter(lib, fmt);
        return static_cast<void*>(writer);
    } catch (...) { return nullptr; }
}

extern "C" void* bit7z_writer_open(void* lib_ptr, const char* path, int format, const char* password) {
    try {
        auto& lib = *static_cast<bit7z::Bit7zLibrary*>(lib_ptr);
        const auto& fmt = [format]() -> const bit7z::BitInOutFormat& {
            switch (format) {
                case 1: return bit7z::BitFormat::Zip;
                case 2: return bit7z::BitFormat::Tar;
                case 3: return bit7z::BitFormat::GZip;
                case 4: return bit7z::BitFormat::BZip2;
                case 5: return bit7z::BitFormat::Xz;
                case 6: return bit7z::BitFormat::Wim;
                default: return bit7z::BitFormat::SevenZip;
            }
        }();
        auto* writer = new bit7z::BitArchiveWriter(
            lib, bit7z::tstring(path ? path : ""), fmt,
            bit7z::tstring(password ? password : ""));
        return static_cast<void*>(writer);
    } catch (...) { return nullptr; }
}

extern "C" void bit7z_writer_close(void* writer_ptr) {
    delete static_cast<bit7z::BitArchiveWriter*>(writer_ptr);
}

extern "C" void bit7z_writer_set_threads(void* writer_ptr, uint32_t n) {
    static_cast<bit7z::BitArchiveWriter*>(writer_ptr)->setThreadsCount(n);
}

extern "C" void bit7z_writer_set_compression_level(void* writer_ptr, int level) {
    auto lvl = bit7z::BitCompressionLevel::Normal;
    switch (level) {
        case 0: lvl = bit7z::BitCompressionLevel::None; break;
        case 1: lvl = bit7z::BitCompressionLevel::Fastest; break;
        case 2: lvl = bit7z::BitCompressionLevel::Fast; break;
        case 3: lvl = bit7z::BitCompressionLevel::Normal; break;
        case 4: lvl = bit7z::BitCompressionLevel::Max; break;
        case 5: lvl = bit7z::BitCompressionLevel::Ultra; break;
    }
    static_cast<bit7z::BitArchiveWriter*>(writer_ptr)->setCompressionLevel(lvl);
}

extern "C" void bit7z_writer_set_password(void* writer_ptr, const char* password) {
    static_cast<bit7z::BitArchiveWriter*>(writer_ptr)->setPassword(
        bit7z::tstring(password ? password : ""));
}

extern "C" void bit7z_writer_set_update_mode(void* writer_ptr, int mode) {
    auto modeEnum = bit7z::UpdateMode::None;
    if (mode == 1) modeEnum = bit7z::UpdateMode::Append;
    else if (mode == 2) modeEnum = bit7z::UpdateMode::Update;
    static_cast<bit7z::BitArchiveWriter*>(writer_ptr)->setUpdateMode(modeEnum);
}

extern "C" int32_t bit7z_writer_add_file(void* writer_ptr, const char* path) {
    try {
        static_cast<bit7z::BitArchiveWriter*>(writer_ptr)->addFile(
            bit7z::tstring(path ? path : ""));
        return 0;
    } catch (...) { return -1; }
}

extern "C" int32_t bit7z_writer_add_files(void* writer_ptr, const char* const* paths, uint32_t count) {
    try {
        std::vector<bit7z::tstring> vec;
        vec.reserve(count);
        for (uint32_t i = 0; i < count; ++i) {
            if (paths[i]) vec.emplace_back(paths[i]);
        }
        static_cast<bit7z::BitArchiveWriter*>(writer_ptr)->addItems(vec);
        return 0;
    } catch (...) { return -1; }
}

extern "C" int32_t bit7z_writer_add_dir(void* writer_ptr, const char* dir) {
    try {
        static_cast<bit7z::BitArchiveWriter*>(writer_ptr)->addDirectory(
            bit7z::tstring(dir ? dir : ""));
        return 0;
    } catch (...) { return -1; }
}

extern "C" int32_t bit7z_writer_compress_to(void* writer_ptr, const char* out_path) {
    try {
        static_cast<bit7z::BitArchiveWriter*>(writer_ptr)->compressTo(
            bit7z::tstring(out_path ? out_path : ""));
        return 0;
    } catch (...) { return -1; }
}

extern "C" int32_t bit7z_writer_compress_to_cb(
    void* writer_ptr,
    const char* out_path,
    void* ctx,
    int32_t (*on_progress)(uint64_t processed, uint64_t total, void* ctx),
    void   (*on_file)(const char* path, void* ctx)
) {
    try {
        auto& writer = *static_cast<bit7z::BitArchiveWriter*>(writer_ptr);
        auto sharedTotal = std::make_shared<uint64_t>(0);

        if (on_progress) {
            writer.setTotalCallback([sharedTotal](uint64_t total) {
                *sharedTotal = total;
            });
            writer.setProgressCallback([ctx, on_progress, sharedTotal](uint64_t processed) -> bool {
                return on_progress(processed, *sharedTotal, ctx) != 0;
            });
        }

        if (on_file) {
            writer.setFileCallback([ctx, on_file](const bit7z::tstring& path) {
                on_file(path.c_str(), ctx);
            });
        }

        writer.compressTo(bit7z::tstring(out_path ? out_path : ""));

        writer.setFileCallback(nullptr);
        writer.setProgressCallback(nullptr);
        writer.setTotalCallback(nullptr);

        return 0;
    } catch (...) { return -1; }
}

// ===== Editor wrappers =====

extern "C" void* bit7z_editor_open(void* lib_ptr, const char* path, int format, const char* password) {
    try {
        auto& lib = *static_cast<bit7z::Bit7zLibrary*>(lib_ptr);
        const auto& fmt = [format]() -> const bit7z::BitInOutFormat& {
            switch (format) {
                case 1: return bit7z::BitFormat::Zip;
                case 2: return bit7z::BitFormat::Tar;
                case 3: return bit7z::BitFormat::GZip;
                case 4: return bit7z::BitFormat::BZip2;
                case 5: return bit7z::BitFormat::Xz;
                case 6: return bit7z::BitFormat::Wim;
                default: return bit7z::BitFormat::SevenZip;
            }
        }();
        auto* editor = new bit7z::BitArchiveEditor(
            lib, bit7z::tstring(path ? path : ""), fmt,
            bit7z::tstring(password ? password : ""));
        return static_cast<void*>(editor);
    } catch (...) { return nullptr; }
}

extern "C" void bit7z_editor_close(void* editor_ptr) {
    delete static_cast<bit7z::BitArchiveEditor*>(editor_ptr);
}

extern "C" int32_t bit7z_editor_rename(void* editor_ptr, uint32_t index, const char* new_path) {
    try {
        static_cast<bit7z::BitArchiveEditor*>(editor_ptr)->renameItem(
            index, bit7z::tstring(new_path ? new_path : ""));
        return 0;
    } catch (...) { return -1; }
}

extern "C" int32_t bit7z_editor_delete(void* editor_ptr, uint32_t index) {
    try {
        static_cast<bit7z::BitArchiveEditor*>(editor_ptr)->deleteItem(index);
        return 0;
    } catch (...) { return -1; }
}

extern "C" int32_t bit7z_editor_apply(void* editor_ptr) {
    try {
        static_cast<bit7z::BitArchiveEditor*>(editor_ptr)->applyChanges();
        return 0;
    } catch (...) { return -1; }
}

// ===== Callback-based extract (supports per-file overwrite, progress, cancel) =====

// C callback types — passed from Rust via function pointers.
// on_overwrite: return 0=Overwrite, 1=Skip
// on_progress:  return 0=cancel, non-zero=continue

inline int32_t bit7z_reader_extract_to_cb(
    void* reader_ptr,
    const uint32_t* indices,
    uint32_t count,
    const char* dest_path,
    void* ctx,
    int32_t (*on_overwrite)(const char* src, const char* dest, uint64_t existing_size, void* ctx),
    int32_t (*on_progress)(uint64_t processed, uint64_t total, void* ctx),
    void      (*on_file)(const char* path, void* ctx)
) {
    try {
        auto& reader = *static_cast<bit7z::BitArchiveReader*>(reader_ptr);
        std::string destDir(dest_path ? dest_path : "");
        if (!destDir.empty() && destDir.back() != '/' && destDir.back() != '\\') {
            destDir += '/';
        }

        // Shared state for total size (set by TotalCallback, read by ProgressCallback)
        auto sharedTotal = std::make_shared<uint64_t>(0);

        if (on_progress) {
            reader.setTotalCallback([sharedTotal](uint64_t total) {
                *sharedTotal = total;
            });
            reader.setProgressCallback([ctx, on_progress, sharedTotal](uint64_t processed) -> bool {
                return on_progress(processed, *sharedTotal, ctx) != 0;
            });
        }

        if (on_overwrite || on_file) {
            reader.setFileCallback([&reader, &destDir, ctx, on_overwrite, on_file](const bit7z::tstring& path) {
                if (on_file) {
                    on_file(path.c_str(), ctx);
                }

                if (!on_overwrite) return;

                std::string fullDest = destDir + path;

                struct stat st;
                if (stat(fullDest.c_str(), &st) == 0) {
                    uint64_t existingSize = static_cast<uint64_t>(st.st_size);
                    int32_t action = on_overwrite(path.c_str(), fullDest.c_str(), existingSize, ctx);
                    switch (action) {
                        case 0: reader.setOverwriteMode(bit7z::OverwriteMode::Overwrite); break;
                        default: reader.setOverwriteMode(bit7z::OverwriteMode::Skip); break;
                    }
                } else {
                    reader.setOverwriteMode(bit7z::OverwriteMode::Overwrite);
                }
            });
        }

        std::vector<uint32_t> idxs(indices, indices + count);
        reader.extractTo(bit7z::tstring(dest_path ? dest_path : ""), idxs);

        // Reset callbacks to avoid accidental reuse
        reader.setFileCallback(nullptr);
        reader.setProgressCallback(nullptr);
        reader.setTotalCallback(nullptr);

        return 0;
    } catch (...) { return -1; }
}
// C-linkage wrapper so Rust can call it via extern "C".
extern "C" int32_t bit7z_reader_extract_to_cb_c(
    void* reader_ptr,
    const uint32_t* indices,
    uint32_t count,
    const char* dest_path,
    void* ctx,
    int32_t (*on_overwrite)(const char* src, const char* dest, uint64_t existing_size, void* ctx),
    int32_t (*on_progress)(uint64_t processed, uint64_t total, void* ctx),
    void   (*on_file)(const char* path, void* ctx)
) {
    return bit7z_reader_extract_to_cb(reader_ptr, indices, count, dest_path, ctx, on_overwrite, on_progress, on_file);
}

// C-linkage wrapper for compress callback.
extern "C" int32_t bit7z_writer_compress_to_cb_c(
    void* writer_ptr,
    const char* out_path,
    void* ctx,
    int32_t (*on_progress)(uint64_t processed, uint64_t total, void* ctx),
    void   (*on_file)(const char* path, void* ctx)
) {
    return bit7z_writer_compress_to_cb(writer_ptr, out_path, ctx, on_progress, on_file);
}