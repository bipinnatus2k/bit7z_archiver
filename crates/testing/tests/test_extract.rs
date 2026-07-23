use std::sync::Arc;

use bit7z_app_archive::runtime_service::{build_bit7z_runtime, ArchiveService};
use bit7z_testing::assert::assert_extraction_matches;
use bit7z_testing::bit7z_harness::Bit7zHarness;
use bit7z_testing::cli_referee::CliReferee;
use bit7z_testing::fixture::ArchiveFixture;
use bit7z_testing::harness::TestHarness;

mod common;
mod fixtures;

fn setup() -> (Bit7zHarness, CliReferee) {
    let lib = common::library().expect("7z library not found");
    let (runtime, resolver) = build_bit7z_runtime(lib);
    let service = Arc::new(ArchiveService::new(runtime, resolver));
    let harness = Bit7zHarness::new(service);
    let cli = CliReferee::new(common::find_7z_exe().expect("7z exe not found"));
    (harness, cli)
}

fn assert_extract(harness: &Bit7zHarness, cli: &CliReferee, fixture: &ArchiveFixture) {
    let dest = tempfile::TempDir::new().unwrap();
    harness
        .extract_all(&fixture.archive_path, dest.path(), fixture.password)
        .unwrap();
    assert_extraction_matches(&fixture.expected, dest.path());

    let dest2 = tempfile::TempDir::new().unwrap();
    cli.extract_all(&fixture.archive_path, dest2.path(), fixture.password)
        .unwrap();
    assert_extraction_matches(&fixture.expected, dest2.path());
}

#[test]
fn test_extract_simple_zip() {
    let (harness, cli) = setup();
    let fixture = fixtures::simple::create_zip().expect("fixture");
    assert_extract(&harness, &cli, &fixture);
}

#[test]
fn test_extract_nested_zip() {
    let (harness, cli) = setup();
    let fixture = fixtures::nested::create_zip().expect("fixture");
    assert_extract(&harness, &cli, &fixture);
}

#[test]
fn test_extract_rich_zip() {
    let (harness, cli) = setup();
    let fixture = fixtures::rich::create_zip().expect("fixture");
    assert_extract(&harness, &cli, &fixture);
}

#[test]
fn test_extract_rich_7z() {
    let (harness, cli) = setup();
    let fixture = fixtures::rich::create_7z().expect("fixture");
    assert_extract(&harness, &cli, &fixture);
}

#[test]
fn test_extract_rich_tar() {
    let (harness, cli) = setup();
    let fixture = fixtures::rich::create_tar().expect("fixture");
    assert_extract(&harness, &cli, &fixture);
}

#[test]
fn test_extract_encrypted_zip() {
    let (harness, cli) = setup();
    let fixture = fixtures::encrypted::create_encrypted_zip().expect("fixture");
    assert_extract(&harness, &cli, &fixture);
}

#[test]
fn test_extract_encrypted_7z() {
    let (harness, cli) = setup();
    let fixture = fixtures::encrypted::create_encrypted_7z().expect("fixture");
    assert_extract(&harness, &cli, &fixture);
}
