//! `cargo xtask play`: builds the web client, serves it, and starts the server.

mod client;
mod files;

use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::thread;

use crate::verify::toolchain::{cargo, unexpected_metadata, workspace_metadata};

pub use client::build as build_client;

/// Where the browser loads the page.
const PAGE_ADDRESS: &str = "127.0.0.1:8000";
/// Where the game server listens. `web/src/main.ts` connects to it.
const SERVER_ADDRESS: &str = "127.0.0.1:8001";

pub fn run(root: &Path) -> ExitCode {
    match play(root) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("play: {message}");
            ExitCode::FAILURE
        }
    }
}

/// Runs until the game server stops.
fn play(root: &Path) -> Result<(), String> {
    build_client(root)?;
    run_step(
        Command::new(cargo())
            .args(["build", "--release", "--package", "ash-server"])
            .current_dir(root),
    )?;
    let server = target_dir(root)?
        .join("release")
        .join(format!("ash-server{}", std::env::consts::EXE_SUFFIX));
    let page = TcpListener::bind(PAGE_ADDRESS)
        .map_err(|error| format!("cannot serve the page on {PAGE_ADDRESS}: {error}"))?;
    let mut server = Command::new(&server)
        .arg(SERVER_ADDRESS)
        .spawn()
        .map_err(|error| format!("cannot start {}: {error}", server.display()))?;
    let web = root.join("web");
    thread::spawn(move || files::serve(page, web));
    println!("play: open http://{PAGE_ADDRESS}/ (Ctrl-C stops)");
    match server.wait() {
        Ok(status) => Err(format!("the server stopped: {status}")),
        Err(error) => {
            let _ = server.kill();
            Err(format!("lost the server: {error}"))
        }
    }
}

/// The directory cargo builds into, which `CARGO_TARGET_DIR` may move.
fn target_dir(root: &Path) -> Result<PathBuf, String> {
    let metadata = workspace_metadata(root).map_err(|error| error.to_string())?;
    metadata["target_directory"]
        .as_str()
        .map(PathBuf::from)
        .ok_or_else(|| unexpected_metadata().to_string())
}

/// Prints and runs `command`, output streamed; an error unless it succeeds.
fn run_step(command: &mut Command) -> Result<(), String> {
    let shown: Vec<_> = std::iter::once(command.get_program())
        .chain(command.get_args())
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    let shown = shown.join(" ");
    println!("play: {shown}");
    match command.status() {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => Err(format!("failed: {shown} ({status})")),
        Err(error) => Err(format!("cannot run {shown}: {error}")),
    }
}
