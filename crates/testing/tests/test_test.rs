use std::sync::Arc;

use bit7z_app_archive::runtime_service::{build_bit7z_runtime, ArchiveService};
use bit7z_testing::bit7z_harness::Bit7zHarness;
use bit7z_testing::cli_referee::CliReferee;
use bit7z_testing::harness::TestHarness;

mod common;
mod fixtures;

#[test]
fn test_integrity_simple_zip() {
    let lib = match common::library() {
        Some(l) => l,
        None => return,
    };
    let (runtime, resolver) = build_bit7z_runtime(lib);
    let service = Arc::new(ArchiveService::new(runtime, resolver));
    let harness = Bit7zHarness::new(service);

    let fixture = fixtures::simple::create_zip().expect("fixture");
    let cli = CliReferee::new(common::find_7z_exe().expect("7z not found"));

    let bit7z_result = harness
        .test_archive(&fixture.archive_path, None)
        .unwrap();
    assert!(bit7z_result.passed, "bit7z integrity test failed: {:?}", bit7z_result.failures);

    let cli_result = cli
        .test_archive(&fixture.archive_path, None)
        .unwrap();
    assert!(cli_result.passed, "CLI integrity test failed: {:?}", cli_result.failures);
}

#[test]
fn test_integrity_nested_zip() {
    let lib = match common::library() {
        Some(l) => l,
        None => return,
    };
    let (runtime, resolver) = build_bit7z_runtime(lib);
    let service = Arc::new(ArchiveService::new(runtime, resolver));
    let harness = Bit7zHarness::new(service);

    let fixture = fixtures::nested::create_zip().expect("fixture");

    let bit7z_result = harness
        .test_archive(&fixture.archive_path, None)
        .unwrap();
    assert!(bit7z_result.passed, "bit7z integrity test failed: {:?}", bit7z_result.failures);
}
