
/// Archive-level properties from bit7z.
#[derive(Debug, Clone)]
pub struct ArchiveProperties {
    pub items_count: u32,
    pub folders_count: u32,
    pub files_count: u32,
    pub total_size: u64,
    pub packed_size: u64,
    pub is_encrypted: bool,
    pub has_encrypted_items: bool,
    pub is_multi_volume: bool,
    pub is_solid: bool,
    pub encrypted_names: bool,
    pub has_comment: bool,
    pub comment_size: Option<usize>,
    pub has_recovery_record: bool,
    pub locked: bool,
    pub dictionary_size: Option<u64>,
    pub headers_size: u64,
    pub volumes_count: u32,
}

impl Default for ArchiveProperties {
    fn default() -> Self {
        Self {
            items_count: 0,
            folders_count: 0,
            files_count: 0,
            total_size: 0,
            packed_size: 0,
            is_encrypted: false,
            has_encrypted_items: false,
            is_multi_volume: false,
            is_solid: false,
            encrypted_names: false,
            has_comment: false,
            comment_size: None,
            has_recovery_record: false,
            locked: false,
            dictionary_size: None,
            headers_size: 0,
            volumes_count: 0,
        }
    }
}

#[cfg(test)]
mod archive_properties_tests {
    use super::*;

    #[test]
    fn test_default_properties_are_zero() {
        let props = ArchiveProperties::default();
        assert_eq!(props.items_count, 0);
        assert_eq!(props.folders_count, 0);
        assert_eq!(props.files_count, 0);
        assert_eq!(props.total_size, 0);
        assert_eq!(props.packed_size, 0);
        assert!(!props.is_encrypted);
        assert!(!props.has_encrypted_items);
        assert!(!props.is_multi_volume);
        assert!(!props.is_solid);
        assert!(!props.encrypted_names);
        assert!(!props.has_comment);
        assert_eq!(props.comment_size, None);
        assert!(!props.has_recovery_record);
        assert!(!props.locked);
        assert_eq!(props.dictionary_size, None);
    }

    #[test]
    fn test_custom_properties() {
        let props = ArchiveProperties {
            items_count: 42,
            folders_count: 5,
            files_count: 37,
            total_size: 10240,
            packed_size: 5120,
            is_encrypted: true,
            has_encrypted_items: true,
            is_multi_volume: false,
            is_solid: true,
            encrypted_names: true,
            has_comment: true,
            comment_size: Some(128),
            has_recovery_record: false,
            locked: true,
            dictionary_size: Some(65536),
            headers_size: 1200,
            volumes_count: 1,
        };
        assert_eq!(props.items_count, 42);
        assert!(props.is_encrypted);
        assert!(props.locked);
        assert_eq!(props.dictionary_size, Some(65536));
    }

    #[test]
    fn test_files_plus_folders_equals_items() {
        let props = ArchiveProperties {
            items_count: 100,
            folders_count: 10,
            files_count: 90,
            ..Default::default()
        };
        assert_eq!(props.files_count + props.folders_count, props.items_count);
    }
}