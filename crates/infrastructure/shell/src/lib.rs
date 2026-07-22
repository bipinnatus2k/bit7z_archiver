//! Shell context menu integration trait.

pub mod linux;
pub mod windows;

pub trait ShellIntegration: Send + Sync {
    fn register() -> Result<(), ShellError>;
    fn unregister() -> Result<(), ShellError>;
    fn is_registered() -> bool;
}

#[derive(Debug, thiserror::Error)]
pub enum ShellError {
    #[error("Registry error: {0}")]
    Registry(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
