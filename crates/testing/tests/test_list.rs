use std::sync::Arc;

use bit7z_app_archive::runtime_service::{build_bit7z_runtime, ArchiveService};
use bit7z_testing::assert::assert_session_state_eq;
use bit7z_testing::bit7z_harness::Bit7zHarness;
use bit7z_testing::cli_referee::CliReferee;
use bit7z_testing::harness::TestHarness;

mod common;
mod fixtures;

#[test]
fn test_list_simple_zip() {
    let lib = match common::library() {
        Some(l) => l,
        None => return,
    };
    let (runtime, resolver) = build_bit7z_runtime(lib);
    let service = Arc::new(ArchiveService::new(runtime, resolver));
    let harness = Bit7zHarness::new(service);

    let fixture = fixtures::simple::create_zip().expect("fixture creation failed");
    let seven_zip = common::find_7z_exe().expect("7z executable not found");
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
