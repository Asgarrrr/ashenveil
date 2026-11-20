//! `cargo xtask accept <test>`: runs one named acceptance test across the
//! workspace (methodology, section 9: the proof of a slice).
//!
//! An acceptance test is an integration-test target, named after its file:
//! `walk` is `crates/<crate>/tests/walk.rs`. The name must match exactly one
//! such target in the workspace, and `cargo test` runs that target alone.

use std::path::Path;
use std::process::{Command, ExitCode, Stdio};

use serde_json::Value;

use crate::verify::toolchain::{cargo, unexpected_metadata, workspace_metadata};

pub fn run(root: &Path, test: &str) -> ExitCode {
    match command(root, test).and_then(|mut command| execute(&mut command)) {
        Ok(()) => {
            println!("accept: ok");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("accept: {message}");
            ExitCode::FAILURE
        }
    }
}

/// Prints and runs `command`; an error unless it succeeds and runs at least
/// one test. An empty or all-ignored target would otherwise prove nothing.
fn execute(command: &mut Command) -> Result<(), String> {
    let args: Vec<_> = command
        .get_args()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    let shown = format!("cargo {}", args.join(" "));
    println!("accept: {shown}");
    let output = command
        .stdout(Stdio::piped())
        .output()
        .map_err(|error| format!("cannot run cargo: {error}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    print!("{stdout}");
    if !output.status.success() {
        Err(format!("failed: {shown} ({})", output.status))
    } else if !ran_a_test(&stdout) {
        Err(format!("ran no test: {shown}"))
    } else {
        Ok(())
    }
}

/// True when a libtest summary line, such as
/// `test result: ok. 3 passed; 0 failed; ...`, counts a passed test.
fn ran_a_test(stdout: &str) -> bool {
    stdout
        .lines()
        .filter_map(|line| line.strip_prefix("test result: "))
        .any(|summary| {
            let words: Vec<&str> = summary.split_whitespace().collect();
            words
                .windows(2)
                .any(|pair| pair[1] == "passed;" && pair[0] != "0")
        })
}

/// The `cargo test` call that runs `test` alone, or why there is none.
fn command(root: &Path, test: &str) -> Result<Command, String> {
    let metadata = workspace_metadata(root).map_err(|error| error.to_string())?;
    let packages = metadata["packages"]
        .as_array()
        .ok_or_else(|| unexpected_metadata().to_string())?;
    let owners: Vec<&str> = packages
        .iter()
        .filter(|package| has_test_target(package, test))
        .filter_map(|package| package["name"].as_str())
        .collect();
    match owners.as_slice() {
        [] => Err(format!(
            "no integration test named `{test}`: expected `crates/<crate>/tests/{test}.rs`"
        )),
        [package] => {
            let mut command = Command::new(cargo());
            command
                .args(["test", "--package", package, "--test", test])
                .current_dir(root)
                .stderr(Stdio::inherit());
            Ok(command)
        }
        _ => Err(format!(
            "`{test}` names an integration test in several packages: {}",
            owners.join(", ")
        )),
    }
}

fn has_test_target(package: &Value, test: &str) -> bool {
    package["targets"].as_array().is_some_and(|targets| {
        targets.iter().any(|target| {
            target["name"] == test
                && target["kind"]
                    .as_array()
                    .is_some_and(|kinds| kinds.iter().any(|kind| kind == "test"))
        })
    })
}

#[cfg(test)]
mod tests {
    use super::super::verify::test_repo::TestRepo;
    use super::*;

    const PASS: &str = "#[test]\nfn passes() {}\n";
    const FAIL: &str = "#[test]\nfn fails() {\n    panic!(\"fails\");\n}\n";

    /// A workspace of `packages`, each with its integration tests as
    /// `(name, source)` pairs and a library whose unit test fails.
    fn workspace(packages: &[(&str, &[(&str, &str)])]) -> TestRepo {
        let repo = TestRepo::new();
        let members: Vec<String> = packages
            .iter()
            .map(|(package, _)| format!("\"{package}\""))
            .collect();
        repo.write(
            "Cargo.toml",
            &format!(
                "[workspace]\nresolver = \"3\"\nmembers = [{}]\n",
                members.join(", ")
            ),
        );
        for (package, tests) in packages {
            repo.write(
                &format!("{package}/Cargo.toml"),
                &format!(
                    "[package]\nname = \"{package}\"\nversion = \"0.0.0\"\nedition = \"2024\"\n"
                ),
            );
            repo.write(
                &format!("{package}/src/lib.rs"),
                &format!("#[cfg(test)]\nmod tests {{\n{FAIL}}}\n"),
            );
            for (test, source) in *tests {
                repo.write(&format!("{package}/tests/{test}.rs"), source);
            }
        }
        repo
    }

    /// Runs `accept` on `repo`, silenced, in its own target directory even
    /// when the environment sets a shared one.
    fn accept(repo: &TestRepo, test: &str) -> Result<(), String> {
        let mut command = command(repo.root(), test)?;
        command
            .env("CARGO_TARGET_DIR", repo.root().join("target"))
            .stderr(Stdio::null());
        execute(&mut command)
    }

    #[test]
    fn passing_test_is_accepted_without_running_its_neighbours() {
        let repo = workspace(&[
            ("alpha", &[("walk", PASS)]),
            (
                "beta",
                &[("walk_over_websocket", FAIL), ("time_passes", FAIL)],
            ),
        ]);
        accept(&repo, "walk").unwrap();
    }

    #[test]
    fn failing_test_is_refused() {
        let repo = workspace(&[("alpha", &[("walk", FAIL)])]);
        assert!(accept(&repo, "walk").is_err());
    }

    #[test]
    fn empty_test_is_refused() {
        let repo = workspace(&[("alpha", &[("walk", "")])]);
        assert!(accept(&repo, "walk").is_err());
    }

    #[test]
    fn test_with_only_ignored_tests_is_refused() {
        let repo = workspace(&[(
            "alpha",
            &[("walk", "#[test]\n#[ignore]\nfn ignored() {}\n")],
        )]);
        assert!(accept(&repo, "walk").is_err());
    }

    #[test]
    fn missing_test_is_refused() {
        let repo = workspace(&[("alpha", &[("walk", PASS)])]);
        assert!(accept(&repo, "time_passes").is_err());
    }

    #[test]
    fn name_shared_by_two_packages_is_refused() {
        let repo = workspace(&[("alpha", &[("walk", PASS)]), ("beta", &[("walk", PASS)])]);
        assert!(accept(&repo, "walk").is_err());
    }
}
