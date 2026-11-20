//! `cargo xtask accept <test>`: runs one named acceptance test across the
//! workspace (methodology, section 9: the proof of a slice).

use std::path::Path;
use std::process::{Command, ExitCode};

pub fn run(root: &Path, test: &str) -> ExitCode {
    let mut command = match command(root, test) {
        Ok(command) => command,
        Err(message) => {
            eprintln!("accept: {message}");
            return ExitCode::FAILURE;
        }
    };
    let args: Vec<_> = command
        .get_args()
        .map(|arg| arg.to_string_lossy())
        .collect();
    println!("accept: cargo {}", args.join(" "));
    match command.status() {
        Ok(status) if status.success() => {
            println!("accept: ok");
            ExitCode::SUCCESS
        }
        Ok(_) => {
            eprintln!("accept: failed: {test}");
            ExitCode::FAILURE
        }
        Err(error) => {
            eprintln!("accept: cannot run cargo: {error}");
            ExitCode::FAILURE
        }
    }
}

/// The `cargo test` call that runs `test` alone, or why there is none.
fn command(root: &Path, test: &str) -> Result<Command, String> {
    let _ = (root, test);
    Err("spec 0001, slice S1".to_owned())
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

    /// Runs `accept` on `repo`; the error holds why it refused or failed.
    fn accept(repo: &TestRepo, test: &str) -> Result<(), String> {
        let output = command(repo.root(), test)?
            .env("CARGO_TARGET_DIR", repo.root().join("target"))
            .output()
            .unwrap();
        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stdout).into_owned())
        }
    }

    #[test]
    fn passing_test_is_accepted_without_running_its_neighbours() {
        let repo = workspace(&[
            ("alpha", &[("walk", PASS), ("walk_over_websocket", FAIL)]),
            ("beta", &[("time_passes", FAIL)]),
        ]);
        accept(&repo, "walk").unwrap();
    }

    #[test]
    fn failing_test_is_refused() {
        let repo = workspace(&[("alpha", &[("walk", FAIL)])]);
        assert!(accept(&repo, "walk").is_err());
    }

    #[test]
    fn missing_test_is_refused() {
        let repo = workspace(&[("alpha", &[("walk", PASS)])]);
        assert!(accept(&repo, "time_passes").is_err());
        assert!(accept(&repo, "wal").is_err());
    }

    #[test]
    fn name_shared_by_two_packages_is_refused() {
        let repo = workspace(&[("alpha", &[("walk", PASS)]), ("beta", &[("walk", PASS)])]);
        assert!(accept(&repo, "walk").is_err());
    }
}
