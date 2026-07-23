use std::sync::Arc;

use bit7z_app_archive::runtime_service::{build_bit7z_runtime, ArchiveService};
use bit7z_testing::assert::assert_extraction_matches;
use bit7z_testing::bit7z_harness::Bit7zHarness;
use bit7z_testing::cli_referee::CliReferee;
use bit7z_testing::harness::TestHarness;

mod common;
mod fixtures;

#[test]
fn test_extract_simple_zip() {
    let lib = match common::library() {
        Some(l) => l,
        None => return,
    };
    let (runtime, resolver) = build_bit7z_runtime(lib);
    let service = Arc::new(ArchiveService::new(runtime, resolver));
    let harness = Bit7zHarness::new(service);

    let fixture = fixtures::simple::create_zip().expect("fixture");
    let cli = CliReferee::new(common::find_7z_exe().expect("7z not found"));

    let dest = tempfile::TempDir::new().unwrap();
    harness
        .extract_all(&fixture.archive_path, dest.path(), None)
        .unwrap();
    assert_extraction_matches(&fixture.expected, dest.path());

    let dest2 = tempfile::TempDir::new().unwrap();
    cli.extract_all(&fixture.archive_path, dest2.path(), None)
        .unwrap();
    assert_extraction_matches(&fixture.expected, dest2.path());
}

#[test]
fn test_extract_nested_zip() {
    let lib = match common::library() {
        Some(l) => l,
        None => return,
    };
    let (runtime, resolver) = build_bit7z_runtime(lib);
    let service = Arc::new(ArchiveService::new(runtime, resolver));
    let harness = Bit7zHarness::new(service);

    let fixture = fixtures::nested::create_zip().expect("fixture");
    let cli = CliReferee::new(common::find_7z_exe().expect("7z not found"));

    let dest = tempfile::TempDir::new().unwrap();
    harness
        .extract_all(&fixture.archive_path, dest.path(), None)
        .unwrap();
    assert_extraction_matches(&fixture.expected, dest.path());

    let dest2 = tempfile::TempDir::new().unwrap();
    cli.extract_all(&fixture.archive_path, dest2.path(), None)
        .unwrap();
    assert_extraction_matches(&fixture.expected, dest2.path());
}
