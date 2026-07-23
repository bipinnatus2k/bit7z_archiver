use std::sync::Arc;

use bit7z_app_archive::runtime_service::{build_bit7z_runtime, ArchiveService};
use bit7z_testing::assert::assert_session_state_eq;
use bit7z_testing::bit7z_harness::Bit7zHarness;
use bit7z_testing::cli_referee::CliReferee;
use bit7z_testing::harness::TestHarness;

mod fixtures;

fn library() -> Option<bit7z_infra_bit7z::Library> {
    let path = bit7z_infra_platform::find_7z_library()?;
    bit7z_infra_bit7z::Library::open(&path.to_string_lossy()).ok()
}

fn find_7z_exe() -> Option<std::path::PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let candidates = [
            r"C:\Program Files\7-Zip\7z.exe",
            r"C:\Program Files (x86)\7-Zip\7z.exe",
        ];
        for p in &candidates {
            let path = std::path::PathBuf::from(p);
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

#[test]
fn test_list_simple_zip() {
    let lib = match library() {
        Some(l) => l,
        None => return,
    };
    let (runtime, resolver) = build_bit7z_runtime(lib);
    let service = Arc::new(ArchiveService::new(runtime, resolver));
    let harness = Bit7zHarness::new(service);

    let fixture = fixtures::simple::create_zip().expect("fixture creation failed");
    let seven_zip = find_7z_exe().expect("7z executable not found");
    let cli = CliReferee::new(seven_zip);

    let bit7z_state = harness
        .open_archive(&fixture.archive_path, None)
        .expect("bit7z open failed");
    assert_session_state_eq(&fixture.expected, &bit7z_state);

    let cli_state = cli
        .open_archive(&fixture.archive_path, None)
        .expect("cli open failed");
    assert_session_state_eq(&fixture.expected, &cli_state);

    assert_session_state_eq(&bit7z_state, &cli_state);
}
