mod extract;
mod create;
mod add_files;

pub use extract::ExtractArchive;
pub use create::CreateArchive;
pub use add_files::AddFilesToArchive;

#[cfg(test)]
mod tests {
    use super::*;
    use bit7z_domain::archive::{ArchiveFormat, ArchiveHandle, OverwriteMode};
    use std::path::PathBuf;

    #[test]
    fn test_extract_command_fields() {
        let cmd = ExtractArchive {
            handle: ArchiveHandle::new_reader().with_path(PathBuf::from("a.zip")),
            entry_indices: vec![0, 1],
            destination: PathBuf::from("/out"),
            overwrite_mode: OverwriteMode::Overwrite,
        };
        assert_eq!(cmd.entry_indices.len(), 2);
    }

    #[test]
    fn test_create_command_fields() {
        let cmd = CreateArchive {
            path: PathBuf::from("new.7z"),
            format: ArchiveFormat::SevenZip,
            encryption: None,
        };
        assert_eq!(cmd.format, ArchiveFormat::SevenZip);
    }

    #[test]
    fn test_add_files_command_fields() {
        let cmd = AddFilesToArchive {
            handle: ArchiveHandle::new_reader().with_path(PathBuf::from("a.zip")),
            files: vec![PathBuf::from("f.txt")],
        };
        assert_eq!(cmd.files.len(), 1);
    }
}
