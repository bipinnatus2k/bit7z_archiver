#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

mod theme;
mod domain;
mod ffi;
mod gui;
mod application;
mod adapters;
mod cli;
mod error;
mod ipc;
mod instance;
mod worker;

use std::sync::Arc;
use clap::Parser;
use cli::Cli;

fn main() {
    println!("bit7z Archiver v{}", env!("CARGO_PKG_VERSION"));

    let cli = Cli::parse();

    let lib_path = adapters::platform::find_7z_library()
        .expect("7-Zip library not found. Install 7-Zip or p7zip.");
    let lib_path_str = lib_path.to_string_lossy();
    let lib = adapters::bit7z::Library::open(&lib_path_str)
        .expect("Failed to load 7-Zip library");
    let repo: Arc<dyn domain::repository::ArchiveRepository> =
        Arc::new(adapters::repository::Bit7zRepository::new(lib));

    if cli.worker {
        worker::run_worker(repo);
    } else if let Some(_) = &cli.command {
        cli::run_cli(repo, &cli);
    } else {
        gui::run_gui();
    }
}


