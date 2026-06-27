use autocxx::prelude::*;

include_cpp! {
    #include "demo.h"
    safety!(unsafe_ffi)

    generate!("multiply")
    generate!("bit7z_create_library")
    generate!("bit7z_destroy_library")

    generate!("bit7z_reader_open")
    generate!("bit7z_reader_close")
    generate!("bit7z_reader_item_count")

    generate!("bit7z_item_path")
    generate!("bit7z_item_name")
    generate!("bit7z_item_size")
    generate!("bit7z_item_packed_size")
    generate!("bit7z_item_is_dir")
    generate!("bit7z_item_is_encrypted")
    generate!("bit7z_item_crc")
    generate!("bit7z_item_from_reader")

    generate!("bit7z_reader_extract_to")
    generate!("bit7z_reader_extract_item_size")
    generate!("bit7z_reader_extract_item_data")
    generate!("bit7z_reader_free_buffer")

    // Directory listing (opaque handle)
    generate!("bit7z_reader_list_directory")
    generate!("bit7z_item_list_count")
    generate!("bit7z_item_list_index")
    generate!("bit7z_item_list_path")
    generate!("bit7z_item_list_size")
    generate!("bit7z_item_list_packed_size")
    generate!("bit7z_item_list_is_dir")
    generate!("bit7z_item_list_is_encrypted")
    generate!("bit7z_item_list_free")

    // Batch items (all items in one call)
    generate!("bit7z_reader_items")
    generate!("bit7z_item_list_crc")
    generate!("bit7z_item_list_item")

    // Test archive integrity
    generate!("bit7z_reader_test")
    generate!("bit7z_test_result_total")
    generate!("bit7z_test_result_failed_count")
    generate!("bit7z_test_result_all_ok")
    generate!("bit7z_test_result_error")
    generate!("bit7z_test_result_free")

    // Encryption detection
    generate!("bit7z_is_header_encrypted")
    generate!("bit7z_is_encrypted")
    generate!("bit7z_reader_has_encrypted_items")

    // Item property accessors (direct BitArchiveItem pointer)
    generate!("bit7z_item_mtime")
    generate!("bit7z_item_ctime")
    generate!("bit7z_item_atime")
    generate!("bit7z_item_attributes")
    generate!("bit7z_item_host_os")
    generate!("bit7z_item_compression_method")
    generate!("bit7z_item_comment")
    generate!("bit7z_item_user")
    generate!("bit7z_item_group")
    generate!("bit7z_item_is_symlink")
    generate!("bit7z_item_posix_attrib")
    generate!("bit7z_item_extension")

    // Writer advanced settings
    generate!("bit7z_writer_set_compression_method")
    generate!("bit7z_writer_set_dictionary_size")
    generate!("bit7z_writer_set_word_size")
    generate!("bit7z_writer_set_solid_mode")
    generate!("bit7z_writer_set_volume_size")
    generate!("bit7z_writer_set_password_ex")
    generate!("bit7z_writer_set_store_timestamps")
    generate!("bit7z_writer_add_dir_filtered")

    // bit7z_writer_add_items is NOT generated here because its signature
    // uses `const char**` (pointer-to-pointer), which autocxx cannot bind.
    // It is declared manually as an `extern "C"` in src/adapters/bit7z/mod.rs.
}

pub use ffi::*;
