use super::{ShellError, ShellIntegration};

pub struct WindowsShellIntegration;

impl ShellIntegration for WindowsShellIntegration {
    fn register() -> Result<(), ShellError> {
        let exe = std::env::current_exe().map_err(ShellError::Io)?;
        let exe_str = exe.to_string_lossy();
        let formats = &[".7z", ".zip", ".rar", ".tar", ".gz", ".xz", ".bz2"];
        for ext in formats {
            reg_set(
                ext,
                "shell\\\\ExtractHere",
                "&Extract Here",
                &format!("\"{}\" --extract \"%1\" --to \"%V\"", exe_str),
            )?;
            reg_set(
                ext,
                "shell\\\\ExtractTo",
                "E&xtract to...",
                &format!("\"{}\" --extract \"%1\"", exe_str),
            )?;
            reg_set(
                ext,
                "shell\\\\TestArchive",
                "&Test Archive",
                &format!("\"{}\" --test \"%1\"", exe_str),
            )?;
        }
        reg_set(
            "*",
            "shell\\\\AddToArchive",
            "&Add to archive...",
            &format!("\"{}\" --compress \"%1\"", exe_str),
        )?;
        reg_set(
            "Directory",
            "shell\\\\AddToArchive",
            "&Add to archive...",
            &format!("\"{}\" --compress \"%1\"", exe_str),
        )?;
        Ok(())
    }

    fn unregister() -> Result<(), ShellError> {
        let formats = &[".7z", ".zip", ".rar", ".tar", ".gz", ".xz", ".bz2"];
        for ext in formats {
            let _ = reg_delete(ext, "shell\\\\ExtractHere");
            let _ = reg_delete(ext, "shell\\\\ExtractTo");
            let _ = reg_delete(ext, "shell\\\\TestArchive");
        }
        let _ = reg_delete("*", "shell\\\\AddToArchive");
        let _ = reg_delete("Directory", "shell\\\\AddToArchive");
        Ok(())
    }

    fn is_registered() -> bool {
        reg_exists(".7z", "shell\\\\ExtractHere")
    }
}

fn reg_set(class: &str, verb: &str, display: &str, command: &str) -> Result<(), ShellError> {
    let key_path = format!("Software\\\\Classes\\\\{}\\\\{}", class, verb);
    let hkcu = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
    let key = hkcu
        .create_subkey(&key_path)
        .map_err(|e| ShellError::Registry(e.to_string()))?
        .0;
    key.set_value("", &display)
        .map_err(|e| ShellError::Registry(e.to_string()))?;
    let cmd_key = hkcu
        .create_subkey(&format!("{}\\\\command", key_path))
        .map_err(|e| ShellError::Registry(e.to_string()))?
        .0;
    cmd_key
        .set_value("", &command)
        .map_err(|e| ShellError::Registry(e.to_string()))?;
    Ok(())
}

fn reg_delete(class: &str, verb: &str) -> Result<(), ShellError> {
    let key_path = format!("Software\\\\Classes\\\\{}\\\\{}", class, verb);
    let hkcu = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
    let _ = hkcu.delete_subkey_all(&key_path);
    Ok(())
}

fn reg_exists(class: &str, verb: &str) -> bool {
    let key_path = format!("Software\\\\Classes\\\\{}\\\\{}", class, verb);
    let hkcu = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
    hkcu.open_subkey(&key_path).is_ok()
}
