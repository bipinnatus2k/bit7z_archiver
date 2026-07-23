use bit7z_app_archive::validator::FormatValidator;
use bit7z_app_archive::validators::zip::ZipValidator;

#[tokio::test]
async fn test_zip_validator_empty_data_fails() {
    let v = ZipValidator;
    let result = v.validate(std::path::Path::new(""), &[]).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_zip_validator_minimal_zip_passes() {
    // A minimal valid Zip with one "hello.txt" entry
    let mut data = Vec::new();

    // Local file header for "hello.txt" containing "Hello"
    data.extend_from_slice(&[0x50, 0x4B, 0x03, 0x04]); // LFH sig
    data.extend_from_slice(&[0x0A, 0x00]); // version needed
    data.extend_from_slice(&[0x00, 0x00]); // flags
    data.extend_from_slice(&[0x00, 0x00]); // compression: store
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // mtime
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // crc32 (placeholder)
    data.extend_from_slice(&[5, 0x00, 0x00, 0x00]); // compressed size
    data.extend_from_slice(&[5, 0x00, 0x00, 0x00]); // uncompressed size
    data.extend_from_slice(&[10, 0x00]); // filename length
    data.extend_from_slice(&[0x00, 0x00]); // extra field length
    data.extend_from_slice(b"hello.txt"); // filename
    data.extend_from_slice(b"Hello"); // file data

    let lfh_offset: u32 = 0;
    let cd_offset = data.len() as u32;

    // Central directory entry
    data.extend_from_slice(&[0x50, 0x4B, 0x01, 0x02]); // CD sig
    data.extend_from_slice(&[0x0A, 0x00]); // version made by
    data.extend_from_slice(&[0x0A, 0x00]); // version needed
    data.extend_from_slice(&[0x00, 0x00]); // flags
    data.extend_from_slice(&[0x00, 0x00]); // compression
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // mtime
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // crc32
    data.extend_from_slice(&[5, 0x00, 0x00, 0x00]); // compressed
    data.extend_from_slice(&[5, 0x00, 0x00, 0x00]); // uncompressed
    data.extend_from_slice(&[10, 0x00]); // filename length
    data.extend_from_slice(&[0x00, 0x00]); // extra
    data.extend_from_slice(&[0x00, 0x00]); // comment
    data.extend_from_slice(&[0x00, 0x00]); // disk start
    data.extend_from_slice(&[0x00, 0x00]); // internal attrs
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // external attrs
    data.extend_from_slice(&lfh_offset.to_le_bytes()); // local header offset (0)
    data.extend_from_slice(b"hello.txt"); // filename

    let cd_size = (data.len() - cd_offset as usize) as u32;

    // EOCD
    data.extend_from_slice(&[0x50, 0x4B, 0x05, 0x06]); // EOCD sig
    data.extend_from_slice(&[0x00, 0x00]); // disk
    data.extend_from_slice(&[0x00, 0x00]); // cd disk
    data.extend_from_slice(&[1, 0x00]); // entries on disk
    data.extend_from_slice(&[1, 0x00]); // total entries
    data.extend_from_slice(&cd_size.to_le_bytes()); // cd size
    data.extend_from_slice(&cd_offset.to_le_bytes()); // cd offset
    data.extend_from_slice(&[0x00, 0x00]); // comment length

    let v = ZipValidator;
    let result = v.validate(std::path::Path::new(""), &data).await;
    assert!(result.is_ok(), "expected OK, got: {:?}", result);
}
