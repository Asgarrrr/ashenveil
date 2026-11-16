//! Single entry point for every project command. Run as `cargo xtask <command>`.

use std::path::Path;
use std::process::ExitCode;

mod verify;

const USAGE: &str = "usage: cargo xtask verify";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [command] if command == "verify" => run_verify(),
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

/// Runs every check, even after a failure, so one run reports all of them.
fn run_verify() -> ExitCode {
    let root = repo_root();
    let mut failed = Vec::new();
    if !report(verify::verify(root)) {
        failed.push("files");
    }
    if !report(verify::dependency_matrix::check(root)) {
        failed.push("dependency matrix");
    }
    for step in verify::toolchain::STEPS {
        if !step.run(root) {
            failed.push(step.name);
        }
    }
    if failed.is_empty() {
        println!("verify: ok");
        ExitCode::SUCCESS
    } else {
        eprintln!("verify: failed: {}", failed.join(", "));
        ExitCode::FAILURE
    }
}

/// Prints the violations; true when there are none.
fn report(result: std::io::Result<Vec<verify::Violation>>) -> bool {
    match result {
        Ok(violations) => {
            for violation in &violations {
                eprintln!("{violation}");
            }
            violations.is_empty()
        }
        Err(error) => {
            eprintln!("verify: {error}");
            false
        }
    }
}

fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask lives one level below the repository root")
}
