#pragma once

#include "bit7z/bitformat.hpp"
#include "bit7z/bit7zlibrary.hpp"
#include "bit7z/bitinputarchive.hpp"
#include "bit7z/bitarchivereader.hpp"
#include "bit7z/bitarchivewriter.hpp"
#include "bit7z/bitarchiveiteminfo.hpp"
#include "bit7z/biterror.hpp"
#include "bit7z/bittypes.hpp"
#include <bit7z/bitwindows.hpp>

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
        auto* reader = new bit7z::BitArchiveReader(lib,
            bit7z::tstring(path ? path : ""),
            bit7z::BitInFormat(0),
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

