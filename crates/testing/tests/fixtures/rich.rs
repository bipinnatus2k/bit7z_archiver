use bit7z_domain::archive::ArchiveFormat;
use bit7z_testing::fixture::{ArchiveFixture, FileSpec};

pub fn create_zip() -> Result<ArchiveFixture, String> {
    ArchiveFixture::build("rich deep directory tree", ArchiveFormat::Zip, None, &rich_entries())
}

pub fn create_7z() -> Result<ArchiveFixture, String> {
    ArchiveFixture::build("rich deep directory tree (7z)", ArchiveFormat::SevenZip, None, &rich_entries())
}

pub fn create_tar() -> Result<ArchiveFixture, String> {
    ArchiveFixture::build("rich deep directory tree (tar)", ArchiveFormat::Tar, None, &rich_entries())
}

fn rich_entries() -> Vec<FileSpec> {
    vec![
        FileSpec {
            path: "root_file.txt",
            content: b"Root file",
            is_directory: false,
            modified: None,
            attributes: None,
        },
        FileSpec {
            path: "level1/file_a.txt",
            content: b"Level 1 file A",
            is_directory: false,
            modified: None,
            attributes: None,
        },
        FileSpec {
            path: "level1/level2/file_b.txt",
            content: b"Level 2 file B",
            is_directory: false,
            modified: None,
            attributes: None,
        },
        FileSpec {
            path: "level1/level2/file_c.bin",
            content: &[0xDE, 0xAD, 0xBE, 0xEF],
            is_directory: false,
            modified: None,
            attributes: None,
        },
        FileSpec {
            path: "level1/level2/level3/deep.txt",
            content: b"Deeply nested file at level 3",
            is_directory: false,
            modified: None,
            attributes: None,
        },
        FileSpec {
            path: "branch1/note.md",
            content: b"# Branch 1 Note\n\nContent in branch 1.",
            is_directory: false,
            modified: None,
            attributes: None,
        },
        FileSpec {
            path: "branch2/sub1/leaf.txt",
            content: b"Leaf in branch 2 / sub 1",
            is_directory: false,
            modified: None,
            attributes: None,
        },
        FileSpec {
            path: "branch2/sub2/leaf2.txt",
            content: b"Leaf in branch 2 / sub 2",
            is_directory: false,
            modified: None,
            attributes: None,
        },
    ]
}
