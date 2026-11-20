//! `cargo xtask accept <test>`: runs one named acceptance test (methodology,
//! section 9: the proof of a slice).
//!
//! An acceptance test is either an integration-test target, named after its
//! file (`walk` is `crates/<crate>/tests/walk.rs`), or a Node test of the web
//! client (`web_model` is `web/tests/web_model.test.js`). The name must match
//! exactly one of them across the repository, and only that test runs.

use std::path::Path;
use std::process::{Command, ExitCode, Stdio};

use serde_json::Value;

use crate::play;
use crate::verify::toolchain::{cargo, unexpected_metadata, workspace_metadata};

pub fn run(root: &Path, test: &str) -> ExitCode {
    let accepted = resolve(root, test).and_then(|acceptance| {
        if let Acceptance::Web { .. } = acceptance {
            play::build_client(root)?;
        }
        execute(&mut acceptance.command(root), &acceptance)
    });
    match accepted {
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

/// Where the acceptance test of a name lives.
enum Acceptance {
    /// The integration-test target `test` of a workspace package.
    Cargo { package: String, test: String },
    /// A Node test of the web client, relative to the repository root. It
    /// imports the built client.
    Web { file: String },
}

impl Acceptance {
    fn command(&self, root: &Path) -> Command {
        let mut command = match self {
            Self::Cargo { package, test } => {
                let mut command = Command::new(cargo());
                command.args(["test", "--package", package, "--test", test]);
                command
            }
            Self::Web { file } => {
                let mut command = Command::new("node");
                command.args(["--test", "--test-reporter=tap", file]);
                command
            }
        };
        command.current_dir(root).stderr(Stdio::inherit());
        command
    }

    /// True when `stdout` counts at least one passed test.
    fn ran_a_test(&self, stdout: &str) -> bool {
        match self {
            Self::Cargo { .. } => ran_a_cargo_test(stdout),
            Self::Web { file } => ran_a_node_test(stdout, file),
        }
    }
}

/// Prints and runs `command`; an error unless it succeeds and runs at least
/// one test. An empty or all-ignored target would otherwise prove nothing.
fn execute(command: &mut Command, acceptance: &Acceptance) -> Result<(), String> {
    let shown: Vec<_> = std::iter::once(command.get_program())
        .chain(command.get_args())
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    let shown = shown.join(" ");
    println!("accept: {shown}");
    let output = command
        .stdout(Stdio::piped())
        .output()
        .map_err(|error| format!("cannot run {shown}: {error}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    print!("{stdout}");
    if !output.status.success() {
        Err(format!("failed: {shown} ({})", output.status))
    } else if !acceptance.ran_a_test(&stdout) {
        Err(format!("ran no test: {shown}"))
    } else {
        Ok(())
    }
}

/// True when a libtest summary line, such as
/// `test result: ok. 3 passed; 0 failed; ...`, counts a passed test.
fn ran_a_cargo_test(stdout: &str) -> bool {
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

/// True when the TAP output of `node --test <file>` has a passed test that
/// was not skipped. A suite does not count, and neither does `file` itself:
/// Node reports a file that declares no test as one passed test of its name.
fn ran_a_node_test(stdout: &str, file: &str) -> bool {
    let mut passed = None;
    for line in stdout.lines().map(str::trim) {
        if let Some(result) = line.strip_prefix("ok ") {
            passed = result
                .split_once(" - ")
                .map(|(_, name)| name)
                .filter(|name| !name.ends_with(" # SKIP") && !name.ends_with(" # TODO"));
        } else if line.starts_with("not ok ") {
            passed = None;
        } else if line == "type: 'test'" && passed.is_some_and(|name| name != file) {
            return true;
        }
    }
    false
}

/// The acceptance test `test` names, or why there is none.
fn resolve(root: &Path, test: &str) -> Result<Acceptance, String> {
    let metadata = workspace_metadata(root).map_err(|error| error.to_string())?;
    let packages = metadata["packages"]
        .as_array()
        .ok_or_else(|| unexpected_metadata().to_string())?;
    let mut owners: Vec<Acceptance> = packages
        .iter()
        .filter(|package| has_test_target(package, test))
        .filter_map(|package| package["name"].as_str())
        .map(|package| Acceptance::Cargo {
            package: package.to_owned(),
            test: test.to_owned(),
        })
        .collect();
    let web = format!("web/tests/{test}.test.js");
    if root.join(&web).is_file() {
        owners.push(Acceptance::Web { file: web });
    }
    match owners.len() {
        0 => Err(format!(
            "no acceptance test named `{test}`: expected `crates/<crate>/tests/{test}.rs` \
             or `web/tests/{test}.test.js`"
        )),
        1 => Ok(owners.remove(0)),
        _ => {
            let places: Vec<&str> = owners
                .iter()
                .map(|owner| match owner {
                    Acceptance::Cargo { package, .. } => package.as_str(),
                    Acceptance::Web { file } => file.as_str(),
                })
                .collect();
            Err(format!(
                "`{test}` names an acceptance test in several places: {}",
                places.join(", ")
            ))
        }
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

    const NODE_PASS: &str = "import test from \"node:test\";\ntest(\"passes\", () => {});\n";
    const NODE_FAIL: &str = "import test from \"node:test\";\ntest(\"fails\", () => {\n  throw new Error(\"fails\");\n});\n";

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

    /// A workspace whose one package passes `walk`, plus the web test
    /// `test` with `source`.
    fn web_workspace(test: &str, source: &str) -> TestRepo {
        let repo = workspace(&[("alpha", &[("walk", PASS)])]);
        repo.write("web/package.json", "{ \"type\": \"module\" }\n");
        repo.write(&format!("web/tests/{test}.test.js"), source);
        repo
    }

    /// Runs `accept` on `repo`, silenced, in its own target directory even
    /// when the environment sets a shared one. The web client is not built:
    /// these web tests import nothing from it.
    fn accept(repo: &TestRepo, test: &str) -> Result<(), String> {
        let acceptance = resolve(repo.root(), test)?;
        let mut command = acceptance.command(repo.root());
        command
            .env("CARGO_TARGET_DIR", repo.root().join("target"))
            .stderr(Stdio::null());
        execute(&mut command, &acceptance)
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

    #[test]
    fn binary_named_like_a_test_does_not_make_it_ambiguous() {
        let repo = workspace(&[("alpha", &[("walk", PASS)]), ("beta", &[])]);
        repo.write("beta/src/bin/walk.rs", "fn main() {}\n");
        accept(&repo, "walk").unwrap();
    }

    #[test]
    fn passing_web_test_is_accepted() {
        let repo = web_workspace("web_model", NODE_PASS);
        accept(&repo, "web_model").unwrap();
    }

    #[test]
    fn failing_web_test_is_refused() {
        let repo = web_workspace("web_model", NODE_FAIL);
        assert!(accept(&repo, "web_model").is_err());
    }

    #[test]
    fn web_test_file_without_tests_is_refused() {
        let repo = web_workspace("web_model", "");
        assert!(accept(&repo, "web_model").is_err());
    }

    #[test]
    fn web_test_with_only_skipped_tests_is_refused() {
        let repo = web_workspace(
            "web_model",
            "import { describe, it } from \"node:test\";\n\
             describe(\"suite\", () => {\n  it.skip(\"skipped\", () => {});\n  \
             it.todo(\"todo\");\n});\n",
        );
        assert!(accept(&repo, "web_model").is_err());
    }

    #[test]
    fn name_of_a_web_test_and_a_cargo_test_is_refused() {
        let repo = web_workspace("walk", NODE_PASS);
        assert!(accept(&repo, "walk").is_err());
    }
}
