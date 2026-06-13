use std::path::PathBuf;

/// Find the 7-Zip shared library on the current platform.
pub fn find_7z_library() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        // Common install paths for 7-Zip
        let candidates = [
            r"C:\Program Files\7-Zip\7z.dll",
            r"C:\Program Files (x86)\7-Zip\7z.dll",
        ];
        for p in &candidates {
            if std::path::Path::new(p).exists() {
                return Some(PathBuf::from(p));
            }
        }
        // Also try PATH
        if let Ok(paths) = std::env::var("PATH") {
            for dir in std::env::split_paths(&paths) {
                let candidate = dir.join("7z.dll");
                if candidate.exists() {
                    return Some(candidate);
                }
            }
        }
    }
    #[cfg(target_os = "linux")]
    {
        // Common library names on Linux
        let lib_names = ["lib7zip.so", "libp7zip.so", "lib7z.so"];
        for name in &lib_names {
            // Check standard library paths
            for dir in ["/usr/lib", "/usr/local/lib", "/usr/lib/x86_64-linux-gnu"] {
                let candidate = PathBuf::from(dir).join(name);
                if candidate.exists() {
                    return Some(candidate);
                }
            }
        }
    }
    None
}
