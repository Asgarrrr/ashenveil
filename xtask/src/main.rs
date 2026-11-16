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

fn run_verify() -> ExitCode {
    let root = repo_root();
    match verify::verify(root) {
        Ok(violations) if violations.is_empty() => {
            println!("verify: ok");
            ExitCode::SUCCESS
        }
        Ok(violations) => {
            for violation in &violations {
                eprintln!("{violation}");
            }
            eprintln!("verify: {} violation(s)", violations.len());
            ExitCode::FAILURE
        }
        Err(error) => {
            eprintln!("verify: {error}");
            ExitCode::FAILURE
        }
    }
}

fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask lives one level below the repository root")
}
