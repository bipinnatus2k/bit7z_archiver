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

    // Test archive integrity
    generate!("bit7z_reader_test")
    generate!("bit7z_test_result_total")
    generate!("bit7z_test_result_failed_count")
    generate!("bit7z_test_result_all_ok")
    generate!("bit7z_test_result_error")
    generate!("bit7z_test_result_free")
}

pub use ffi::*;
