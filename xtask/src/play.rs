//! `cargo xtask play`: builds the web client, serves it, and starts the server.

use std::path::Path;
use std::process::ExitCode;

pub fn run(_root: &Path) -> ExitCode {
    eprintln!("play: not implemented yet");
    ExitCode::FAILURE
}
