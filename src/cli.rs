//! CLI argument parser and entry point.
//! Supports: --open, --extract, --test, --compress

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "bit7z_archiver", version, about = "Cross-platform compressed file viewer and editor")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    Open {
        path: String,
        #[arg(long)]
        password: Option<String>,
    },
    Extract {
        path: String,
        #[arg(long)]
        to: Option<String>,
        #[arg(long)]
        password: Option<String>,
    },
    Test {
        path: String,
        #[arg(long)]
        password: Option<String>,
    },
    Compress {
        #[arg(num_args = 1..)]
        files: Vec<String>,
        #[arg(long)]
        to: Option<String>,
        #[arg(long, default_value = "7z")]
        format: String,
        #[arg(long)]
        password: Option<String>,
    },
}
