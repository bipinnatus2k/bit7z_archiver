use bit7z_app_archive::auto_format::AutoFormat;
use bit7z_domain::archive::ArchiveFormat;

fn gzip_header() -> Vec<u8> {
    // Minimum valid GZip header (10 bytes)
    vec![
        0x1F,       // ID1
        0x8B,       // ID2
        0x08,       // CM = deflate
        0x00,       // FLG
        0x00, 0x00, 0x00, 0x00, // MTIME
        0x00,       // XFL
        0x03,       // OS = Unix
    ]
}

#[test]
fn test_detect_zip() {
    let dir = tempfile::tempdir().unwrap();
    let zip_path = dir.path().join("test.zip");
    let data = [0x50, 0x4B, 0x03, 0x04, 0x0A, 0x00, 0x00, 0x00, 0x00, 0x00];
    std::fs::write(&zip_path, &data).unwrap();
    let detector = AutoFormat::new();
    let result = detector.detect(&zip_path).unwrap();
    assert_eq!(result.format, ArchiveFormat::Zip);
}

#[test]
fn test_detect_7z() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.7z");
    let data = [0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C];
    std::fs::write(&path, &data).unwrap();
    let detector = AutoFormat::new();
    let result = detector.detect(&path).unwrap();
    assert_eq!(result.format, ArchiveFormat::SevenZip);
}

#[test]
fn test_detect_gzip_tar() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("archive.tar.gz");
    let data = gzip_header();
    std::fs::write(&path, &data).unwrap();
    let detector = AutoFormat::new();
    let result = detector.detect(&path).unwrap();
    assert_eq!(result.format, ArchiveFormat::GZip);
    assert_eq!(result.logical, Some(ArchiveFormat::TarGz));
    assert_eq!(result.inner, Some(ArchiveFormat::Tar));
}

#[test]
fn test_format_mismatch_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("document.zip");
    // GZip magic bytes but .zip extension
    let data = gzip_header();
    std::fs::write(&path, &data).unwrap();
    let detector = AutoFormat::new();
    assert!(detector.detect(&path).is_err());
}
