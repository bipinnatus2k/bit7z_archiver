use bit7z_domain::archive::ArchiveFormat;
use bit7z_testing::fixture::{ArchiveFixture, FileSpec};

pub fn create_zip() -> Result<ArchiveFixture, String> {
    let entries = vec![
        FileSpec {
            path: "hello.txt",
            content: b"Hello, World!",
            is_directory: false,
            modified: None,
            attributes: None,
        },
        FileSpec {
            path: "empty.bin",
            content: b"",
            is_directory: false,
            modified: None,
            attributes: None,
        },
    ];
    ArchiveFixture::build("simple two-file archive", ArchiveFormat::Zip, None, &entries)
}
