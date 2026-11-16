//! Single entry point for every project command. Run as `cargo xtask <command>`.

use std::path::Path;
use std::process::ExitCode;

mod hook;
mod setup;
mod verify;

const USAGE: &str = "usage: cargo xtask verify [--fast]\n       cargo xtask setup\n       \
                     cargo xtask hook constitution | commit-msg <message-file> | pre-commit";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        ["verify"] => run_verify(),
        ["verify", "--fast"] | ["hook", "pre-commit"] => run_verify_fast(),
        ["setup"] => run_setup(),
        ["hook", "constitution"] => run_constitution_hook(),
        ["hook", "commit-msg", message_file] => run_commit_msg_hook(Path::new(message_file)),
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
    if !report(verify::lint_inheritance::check(root)) {
        failed.push("lint inheritance");
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

fn run_verify_fast() -> ExitCode {
    if hook::pre_commit::run(repo_root()) {
        println!("verify --fast: ok");
        ExitCode::SUCCESS
    } else {
        eprintln!("verify --fast: failed");
        ExitCode::FAILURE
    }
}

fn run_setup() -> ExitCode {
    match setup::install(repo_root()) {
        Ok(hooks) => {
            for hook in hooks {
                println!("setup: installed {}", hook.display());
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("setup: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Exit code 2 blocks the tool call and shows stderr to Claude. Any other
/// failure code is a non-blocking error: the call goes ahead.
fn run_constitution_hook() -> ExitCode {
    let verdict = std::io::read_to_string(std::io::stdin())
        .map_err(|error| error.to_string())
        .and_then(|input| hook::constitution::judge(&input));
    match verdict {
        Ok(hook::constitution::Verdict::Allow) => ExitCode::SUCCESS,
        Ok(hook::constitution::Verdict::Block(message)) => {
            eprintln!("{message}");
            ExitCode::from(2)
        }
        Err(error) => {
            eprintln!("hook constitution: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run_commit_msg_hook(message_file: &Path) -> ExitCode {
    let checked = std::fs::read_to_string(message_file)
        .map_err(|error| format!("{}: {error}", message_file.display()))
        .and_then(|message| hook::commit_msg::check(&message));
    match checked {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!(
                "commit-msg: {error}\ncommit-msg: {}",
                hook::commit_msg::FORMAT
            );
            ExitCode::FAILURE
        }
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
