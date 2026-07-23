use std::path::PathBuf;

pub fn library() -> Option<bit7z_infra_bit7z::Library> {
    let path = bit7z_infra_platform::find_7z_library()?;
    bit7z_infra_bit7z::Library::open(&path.to_string_lossy()).ok()
}

pub fn find_7z_exe() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let candidates = [
            r"C:\Program Files\7-Zip\7z.exe",
            r"C:\Program Files (x86)\7-Zip\7z.exe",
        ];
        for p in &candidates {
            let path = PathBuf::from(p);
            if path.exists() {
                return Some(path);
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(paths) = std::env::var("PATH") {
            for dir in std::env::split_paths(&paths) {
                let candidate = dir.join("7z");
                if candidate.exists() {
                    return Some(candidate);
                }
            }
        }
    }
    None
}
