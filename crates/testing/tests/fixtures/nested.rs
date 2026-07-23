use bit7z_domain::archive::ArchiveFormat;
use bit7z_testing::fixture::{ArchiveFixture, FileSpec};

pub fn create_zip() -> Result<ArchiveFixture, String> {
    let entries = vec![
        FileSpec {
            path: "root.txt",
            content: b"root",
            is_directory: false,
            modified: None,
            attributes: None,
        },
        FileSpec {
            path: "sub/dir/a.txt",
            content: b"alpha",
            is_directory: false,
            modified: None,
            attributes: None,
        },
        FileSpec {
            path: "sub/dir/b.bin",
            content: &[0x00, 0x01, 0x02, 0xFF],
            is_directory: false,
            modified: None,
            attributes: None,
        },
    ];
    ArchiveFixture::build("nested directories", ArchiveFormat::Zip, None, &entries)
}
