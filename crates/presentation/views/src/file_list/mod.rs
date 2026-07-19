pub mod archive_file_list;
pub(crate) mod archive_fm_state;
mod file_list_table_delegate;

#[derive(Debug, Clone, PartialEq)]
pub enum ViewStatus {
    Empty,
    Loading,
    Ready,
    Error(String),
}

#[derive(Clone, Debug)]
pub struct LevelEntry {
    pub display_name: String,
    pub is_directory: bool,
    pub original_index: u32,
    pub size: u64,
    pub compressed_size: u64,
    pub modified: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone)]
pub struct OpHandle {
    pub can_cancel: bool,
}

#[derive(Debug, Clone)]
pub struct ProgressSnapshot {
    pub current: u64,
    pub total: u64,
    pub message: String,
}
