use serde::{Deserialize, Serialize};
use crate::archive::archive_format::ArchiveFormat;
use crate::password::Password;

/// Encryption method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EncryptionMethod {
    #[serde(rename = "aes256")]
    Aes256,
    #[serde(rename = "zipcrypto")]
    ZipCrypto,
}

impl EncryptionMethod {
    pub fn display_name(&self) -> &str {
        match self {
            EncryptionMethod::Aes256 => "AES-256",
            EncryptionMethod::ZipCrypto => "ZipCrypto",
        }
    }

    pub fn available_for(format: ArchiveFormat) -> Vec<Self> {
        match format {
            ArchiveFormat::SevenZip => vec![EncryptionMethod::Aes256],
            ArchiveFormat::Zip => vec![EncryptionMethod::ZipCrypto, EncryptionMethod::Aes256],
            _ => vec![],
        }
    }
}


/// Configuration for creating encrypted archives.
#[must_use = "EncryptionConfig contains a password; unused config is likely a bug"]
#[derive(Debug, Clone)]
pub struct EncryptionConfig {
    pub password: Password,
    pub method: EncryptionMethod,
    pub encrypt_filenames: bool,
}

