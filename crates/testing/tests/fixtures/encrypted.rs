use bit7z_domain::archive::ArchiveFormat;
use bit7z_testing::fixture::{ArchiveFixture, FileSpec};

const TEST_PASSWORD: &str = "test_password_123";

pub fn create_encrypted_zip() -> Result<ArchiveFixture, String> {
    let entries = vec![
        FileSpec {
            path: "secret.txt",
            content: b"this is encrypted content",
            is_directory: false,
            modified: None,
            attributes: None,
        },
        FileSpec {
            path: "data.bin",
            content: &[0xAB, 0xCD, 0xEF, 0x01, 0x02],
            is_directory: false,
            modified: None,
            attributes: None,
        },
    ];
    ArchiveFixture::build(
        "encrypted zip archive",
        ArchiveFormat::Zip,
        Some(TEST_PASSWORD),
        &entries,
    )
}

pub fn create_encrypted_7z() -> Result<ArchiveFixture, String> {
    let entries = vec![
        FileSpec {
            path: "secret.txt",
            content: b"this is encrypted content (7z)",
            is_directory: false,
            modified: None,
            attributes: None,
        },
        FileSpec {
            path: "notes.md",
            content: b"# 7z Encrypted Archive\n\nTest content.",
            is_directory: false,
            modified: None,
            attributes: None,
        },
    ];
    ArchiveFixture::build(
        "encrypted 7z archive",
        ArchiveFormat::SevenZip,
        Some(TEST_PASSWORD),
        &entries,
    )
}
