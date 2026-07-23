//! Shared FFI helpers used by the bit7z adapters.

use std::path::Path;

use bit7z_domain::archive::{ArchiveEntry, ArchiveFormat};
use chrono::DateTime;

use bit7z_infra_bit7z as bit7z;

/// Detect the writer format from a file path extension.
pub fn detect_writer_format(path: &Path) -> bit7z::WriterFormat {
    path.extension()
        .and_then(|ext| {
            let ext = ext.to_string_lossy().to_lowercase();
            match ext.as_str() {
                "7z" => Some(bit7z::WriterFormat::SevenZip),
                "zip" => Some(bit7z::WriterFormat::Zip),
                "tar" => Some(bit7z::WriterFormat::Tar),
                "gz" | "tgz" => Some(bit7z::WriterFormat::GZip),
                "bz2" | "tbz" | "tbz2" => Some(bit7z::WriterFormat::BZip2),
                "xz" | "txz" => Some(bit7z::WriterFormat::Xz),
                "wim" => Some(bit7z::WriterFormat::Wim),
                _ => None,
            }
        })
        .unwrap_or(bit7z::WriterFormat::SevenZip)
}

/// Map a bit7z writer format to the domain ArchiveFormat.
pub fn writer_format_to_archive_format(wf: bit7z::WriterFormat) -> Option<ArchiveFormat> {
    match wf {
        bit7z::WriterFormat::SevenZip => Some(ArchiveFormat::SevenZip),
        bit7z::WriterFormat::Zip => Some(ArchiveFormat::Zip),
        bit7z::WriterFormat::Tar => Some(ArchiveFormat::Tar),
        bit7z::WriterFormat::GZip => Some(ArchiveFormat::GZip),
        bit7z::WriterFormat::BZip2 => Some(ArchiveFormat::BZip2),
        bit7z::WriterFormat::Xz => Some(ArchiveFormat::Xz),
        bit7z::WriterFormat::Wim => Some(ArchiveFormat::Wim),
        _ => None,
    }
}

/// Read all entries from a raw reader handle.
pub fn read_all_entries(raw: *mut std::ffi::c_void) -> Vec<ArchiveEntry> {
    let list = unsafe { bit7z_ffi::bit7z_reader_items(raw as *mut _) };
    if list.is_null() {
        return Vec::new();
    }
    let count = unsafe { bit7z_ffi::bit7z_item_list_count(list as *mut _) };
    let mut entries = Vec::with_capacity(count as usize);

    for i in 0..count {
        use std::ffi::CStr;
        let p = unsafe { bit7z_ffi::bit7z_item_list_path(list as *mut _, i) };
        let path_s = if p.is_null() {
            String::new()
        } else {
            let raw = unsafe { CStr::from_ptr(p).to_string_lossy().into_owned() };
            raw.replace('\\', "/")
        };
        let name_s = path_s
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or(&path_s)
            .to_string();
        let size = unsafe { bit7z_ffi::bit7z_item_list_size(list as *mut _, i) };
        let csize = unsafe { bit7z_ffi::bit7z_item_list_packed_size(list as *mut _, i) };
        let is_dir = unsafe { bit7z_ffi::bit7z_item_list_is_dir(list as *mut _, i) != 0 };
        let is_enc = unsafe { bit7z_ffi::bit7z_item_list_is_encrypted(list as *mut _, i) != 0 };

        let mut entry = ArchiveEntry {
            name: name_s,
            path: path_s,
            size,
            compressed_size: csize,
            is_directory: is_dir,
            is_encrypted: is_enc,
            original_index: i,
            ..Default::default()
        };
        populate_item_details(&mut entry, list as *mut std::ffi::c_void, i);
        entries.push(entry);
    }

    unsafe { bit7z_ffi::bit7z_item_list_free(list as *mut _) };
    entries
}

/// Populate extended fields on an ArchiveEntry using the FFI item accessors.
pub fn populate_item_details(entry: &mut ArchiveEntry, list: *mut std::ffi::c_void, index: u32) {
    entry.crc = Some(unsafe { bit7z_ffi::bit7z_item_list_crc(list as *mut _, index) });

    let item_ptr = unsafe { bit7z_ffi::bit7z_item_list_item(list as *mut _, index) };
    if item_ptr.is_null() {
        return;
    } // synthetic directory — no real item

    let mtime = unsafe { bit7z_ffi::bit7z_item_mtime(item_ptr) };
    if mtime > 0 {
        entry.modified = DateTime::from_timestamp(mtime as i64, 0);
    }

    let ctime = unsafe { bit7z_ffi::bit7z_item_ctime(item_ptr) };
    if ctime > 0 {
        entry.created = DateTime::from_timestamp(ctime as i64, 0);
    }

    let atime = unsafe { bit7z_ffi::bit7z_item_atime(item_ptr) };
    if atime > 0 {
        entry.accessed = DateTime::from_timestamp(atime as i64, 0);
    }

    entry.attributes = Some(unsafe { bit7z_ffi::bit7z_item_attributes(item_ptr) });
    entry.host_os = Some(unsafe { bit7z_ffi::bit7z_item_host_os(item_ptr) });
    entry.posix_attrib = Some(unsafe { bit7z_ffi::bit7z_item_posix_attrib(item_ptr) });
    entry.is_symlink = unsafe { bit7z_ffi::bit7z_item_is_symlink(item_ptr) != 0 };

    // Use a single stack-allocated buffer for all C string reads
    let mut buf = [0u8; 256];

    macro_rules! read_field {
        ($ffi_fn:ident, $field:ident) => {
            let ret = unsafe { bit7z_ffi::$ffi_fn(item_ptr, buf.as_mut_ptr() as *mut _, 256) };
            if ret >= 0 {
                let s = unsafe { std::ffi::CStr::from_ptr(buf.as_ptr() as *const _) };
                let s = s.to_string_lossy().into_owned();
                if !s.is_empty() {
                    entry.$field = Some(s);
                }
            }
        };
    }

    read_field!(bit7z_item_compression_method, compression_method);
    read_field!(bit7z_item_comment, comment);
    read_field!(bit7z_item_user, user);
    read_field!(bit7z_item_group, group);
    read_field!(bit7z_item_extension, extension);
    read_field!(bit7z_item_hardlink, hardlink);
}
