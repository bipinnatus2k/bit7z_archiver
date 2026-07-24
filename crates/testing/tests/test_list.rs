use std::sync::Arc;

use bit7z_app_archive::runtime_service::{build_bit7z_runtime, ArchiveService};
use bit7z_testing::assert::assert_session_state_eq;
use bit7z_testing::bit7z_harness::Bit7zHarness;
use bit7z_testing::cli_referee::CliReferee;
use bit7z_testing::fixture::ArchiveFixture;
use bit7z_testing::harness::TestHarness;

mod common;
mod fixtures;

fn setup() -> (Bit7zHarness, CliReferee) {
    let lib = common::library().expect("7z library not found");
    let (runtime, resolver, detector) = build_bit7z_runtime(lib);
    let service = Arc::new(ArchiveService::new(runtime, resolver, Some(detector)));
    let harness = Bit7zHarness::new(service);
    let cli = CliReferee::new(common::find_7z_exe().expect("7z exe not found"));
    (harness, cli)
}

fn assert_list(harness: &Bit7zHarness, cli: &CliReferee, fixture: &ArchiveFixture) {
    let bit7z_state = harness
        .open_archive(&fixture.archive_path, fixture.password)
        .expect("bit7z open failed");
    assert_session_state_eq(&fixture.expected, &bit7z_state);

    let cli_state = cli
        .open_archive(&fixture.archive_path, fixture.password)
        .expect("cli open failed");
    assert_session_state_eq(&fixture.expected, &cli_state);

    assert_session_state_eq(&bit7z_state, &cli_state);
}

#[test]
fn test_list_simple_zip() {
    let (harness, cli) = setup();
    let fixture = fixtures::simple::create_zip().expect("fixture");
    assert_list(&harness, &cli, &fixture);
}

#[test]
fn test_list_nested_zip() {
    let (harness, cli) = setup();
    let fixture = fixtures::nested::create_zip().expect("fixture");
    assert_list(&harness, &cli, &fixture);
}

#[test]
fn test_list_rich_zip() {
    let (harness, cli) = setup();
    let fixture = fixtures::rich::create_zip().expect("fixture");
    assert_list(&harness, &cli, &fixture);
}

#[test]
fn test_list_rich_7z() {
    let (harness, cli) = setup();
    let fixture = fixtures::rich::create_7z().expect("fixture");
    assert_list(&harness, &cli, &fixture);
}

#[test]
fn test_list_rich_tar() {
    let (harness, cli) = setup();
    let fixture = fixtures::rich::create_tar().expect("fixture");
    assert_list(&harness, &cli, &fixture);
}

#[test]
fn test_list_encrypted_zip() {
    let (harness, cli) = setup();
    let fixture = fixtures::encrypted::create_encrypted_zip().expect("fixture");
    assert_list(&harness, &cli, &fixture);
}

#[test]
fn test_list_encrypted_7z() {
    let (harness, cli) = setup();
    let fixture = fixtures::encrypted::create_encrypted_7z().expect("fixture");
    assert_list(&harness, &cli, &fixture);
}
