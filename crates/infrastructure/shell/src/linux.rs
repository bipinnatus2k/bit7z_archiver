use super::{ShellError, ShellIntegration};

pub struct LinuxShellIntegration;

const DESKTOP_FILE: &str = r#"[Desktop Entry]
Name=bit7z Archiver
Type=Application
Exec=bit7z_archiver %F
MimeType=application/x-7z-compressed;application/zip;application/x-rar;application/x-tar;application/gzip;application/x-xz;application/x-bzip2;
Actions=extract-here;extract-to;test-archive;

[Desktop Action extract-here]
Name=Extract Here
Exec=bit7z_archiver --extract %f --to %W

[Desktop Action extract-to]
Name=Extract to...
Exec=bit7z_archiver --extract %f

[Desktop Action test-archive]
Name=Test Archive
Exec=bit7z_archiver --test %f
"#;

impl ShellIntegration for LinuxShellIntegration {
    fn register() -> Result<(), ShellError> {
        let dir = std::path::PathBuf::from(std::env::var("XDG_DATA_HOME").unwrap_or_else(|_| {
            format!("{}/.local/share", std::env::var("HOME").unwrap_or_default())
        }))
        .join("applications");
        std::fs::create_dir_all(&dir).map_err(ShellError::Io)?;
        std::fs::write(dir.join("bit7z-archiver.desktop"), DESKTOP_FILE).map_err(ShellError::Io)?;
        Ok(())
    }

    fn unregister() -> Result<(), ShellError> {
        let dir = std::path::PathBuf::from(std::env::var("XDG_DATA_HOME").unwrap_or_else(|_| {
            format!("{}/.local/share", std::env::var("HOME").unwrap_or_default())
        }))
        .join("applications");
        let path = dir.join("bit7z-archiver.desktop");
        if path.exists() {
            std::fs::remove_file(path).map_err(ShellError::Io)?;
        }
        Ok(())
    }

    fn is_registered() -> bool {
        let dir = std::path::PathBuf::from(std::env::var("XDG_DATA_HOME").unwrap_or_else(|_| {
            format!("{}/.local/share", std::env::var("HOME").unwrap_or_default())
        }))
        .join("applications");
        dir.join("bit7z-archiver.desktop").exists()
    }
}
