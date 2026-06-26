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
