


/// Overwrite mode for extraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverwriteMode {
    Ask,
    Overwrite,
    Skip,
    RenameExtracted,
}

impl OverwriteMode {
    pub fn label(&self) -> &'static str {
        match self {
            OverwriteMode::Ask => "Ask",
            OverwriteMode::Overwrite => "Overwrite",
            OverwriteMode::Skip => "Skip",
            OverwriteMode::RenameExtracted => "Rename extracted",
        }
    }

    pub fn all() -> Vec<OverwriteMode> {
        vec![
            OverwriteMode::Ask,
            OverwriteMode::Overwrite,
            OverwriteMode::Skip,
            OverwriteMode::RenameExtracted,
        ]
    }

    pub fn index(self) -> i32 {
        match self {
            OverwriteMode::Ask => 0,
            OverwriteMode::Overwrite => 1,
            OverwriteMode::Skip => 2,
            OverwriteMode::RenameExtracted => 3,
        }
    }
}
