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

fn main() {
    println!("bit7z Archiver v{}", env!("CARGO_PKG_VERSION"));
    gui::run_gui();
}

