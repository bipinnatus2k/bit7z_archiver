use std::path::PathBuf;

pub fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

pub fn fixture_path(name: &str) -> PathBuf {
    fixtures_dir().join(name)
}

pub fn fixture_exists(name: &str) -> bool {
    fixture_path(name).exists()
}

pub fn ensure_fixtures() -> Vec<String> {
    let needed = vec![
        "basic.7z",
        "basic.zip",
        "basic.tar",
        "corrupted.7z",
        "encrypted.7z",
        "empty.7z",
        "multi_file.7z",
    ];
    needed
        .into_iter()
        .filter(|n| !fixture_exists(n))
        .map(|n| n.to_string())
        .collect()
}

/// Create a temporary directory with the given files and their contents.
/// Returns the path to the directory.
pub fn temp_dir_with_files(files: &[(&str, &str)]) -> std::path::PathBuf {
    let dir = temp_dir();
    for (name, content) in files {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(&path, content).unwrap();
    }
    dir
}

/// Create a temporary directory that is cleaned up when the guard is dropped.
pub fn temp_dir() -> std::path::PathBuf {
    let dir = std::env::temp_dir()
        .join(format!("bit7z_test_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// Create a temporary file with content and return its path.
pub fn temp_file(name: &str, content: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir()
        .join(format!("bit7z_test_{}_{}", std::process::id(), name));
    std::fs::write(&path, content).unwrap();
    path
}

/// Locate the 7z CLI executable on the system.
pub fn find_7z_cli() -> Option<std::path::PathBuf> {
    // Check PATH first
    if let Ok(path) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path) {
            let candidate = dir.join("7z.exe");
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }
    // Common install locations
    for candidate in &[
        r"C:\Program Files\7-Zip\7z.exe",
        r"C:\Program Files (x86)\7-Zip\7z.exe",
    ] {
        let p = std::path::Path::new(candidate);
        if p.exists() {
            return Some(p.to_path_buf());
        }
    }
    // Check vcpkg_installed
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let vcpkg = manifest_dir
        .join("vcpkg_installed")
        .join("x64-windows")
        .join("tools")
        .join("7z.exe");
    if vcpkg.exists() {
        return Some(vcpkg);
    }
    None
}

/// Run a 7z CLI command and return stdout on success, or error message on failure.
pub fn run_7z(args: &[&str]) -> Result<String, String> {
    let exe = find_7z_cli().ok_or_else(|| "7z CLI not found".to_string())?;
    let output = std::process::Command::new(&exe)
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .map_err(|e| format!("Failed to run 7z: {}", e))?;
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(stdout)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Err(format!("7z failed (exit={:?}): {}", output.status.code(), stderr))
    }
}

/// Count total entries (files + dirs) in a 7z archive using CLI listing.
pub fn count_entries_7z(archive: &std::path::Path) -> Result<u64, String> {
    let stdout = run_7z(&["l", "-slt", archive.to_str().unwrap()])?;
    // Count "Path = " lines (one per entry)
    let count = stdout.lines().filter(|l| l.starts_with("Path = ")).count() as u64;
    Ok(count)
}

/// Check archive integrity using CLI.
pub fn verify_7z(archive: &std::path::Path) -> Result<bool, String> {
    let stdout = run_7z(&["t", archive.to_str().unwrap()])?;
    Ok(stdout.contains("Everything is Ok"))
}
