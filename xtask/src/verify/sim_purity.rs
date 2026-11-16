//! The simulation crates stay pure only through attributes in their crate
//! roots. `lib.rs` cannot be a protected path, since every new module edits
//! it, so `verify` checks the attributes themselves. `#![no_std]` also does
//! not stop `extern crate std;`, which would bring the clock back.

use std::io;
use std::path::Path;

use super::{Violation, read_file, relative, repository_files};

const SIM_CRATES: &[&str] = &["crates/ash-core", "crates/ash-sim"];

/// Inner attributes every simulation crate root must hold, compared without
/// whitespace.
const REQUIRED_ATTRIBUTES: &[&str] = &[
    "#![cfg_attr(not(test), no_std)]",
    "#![deny(clippy::float_arithmetic, clippy::arithmetic_side_effects)]",
];

const BANNED_LINTS: &[&str] = &["float_arithmetic", "arithmetic_side_effects"];

pub fn check(root: &Path) -> io::Result<Vec<Violation>> {
    let mut violations = Vec::new();
    for crate_dir in SIM_CRATES {
        // Deleting a whole crate is a visible change; a lost `lib.rs` is not.
        if !root.join(crate_dir).is_dir() {
            continue;
        }
        let src = root.join(crate_dir).join("src");
        let lib = src.join("lib.rs");
        let lib_source = read_file(root, &lib)?;
        for message in crate_root_problems(&lib_source) {
            violations.push(Violation {
                path: relative(root, &lib),
                message,
            });
        }
        for file in repository_files(&src, &|name| name.ends_with(".rs"))? {
            if imports_std(&read_file(root, &file)?) {
                violations.push(Violation {
                    path: relative(root, &file),
                    message: "`extern crate std` in a simulation crate".into(),
                });
            }
        }
    }
    Ok(violations)
}

fn crate_root_problems(source: &str) -> Vec<String> {
    let attributes: Vec<String> = source
        .lines()
        .map(without_whitespace)
        .filter(|line| line.starts_with("#!["))
        .collect();
    let mut problems = Vec::new();
    for required in REQUIRED_ATTRIBUTES {
        if !attributes.contains(&without_whitespace(required)) {
            problems.push(format!("missing crate attribute `{required}`"));
        }
    }
    // Another crate-level attribute naming a banned lint can only weaken it.
    for attribute in &attributes {
        let required = REQUIRED_ATTRIBUTES
            .iter()
            .any(|r| without_whitespace(r) == *attribute);
        if !required && BANNED_LINTS.iter().any(|lint| attribute.contains(lint)) {
            problems.push(format!(
                "crate attribute `{attribute}` overrides a simulation ban"
            ));
        }
    }
    problems
}

/// Textual: it catches the easy mistake, not a macro built to hide it.
fn imports_std(source: &str) -> bool {
    let code: String = source
        .lines()
        .map(|line| line.split("//").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join(" ");
    let tokens: Vec<&str> = code
        .split(|c: char| c.is_whitespace() || c == ';')
        .filter(|token| !token.is_empty())
        .collect();
    tokens
        .windows(3)
        .any(|window| window == ["extern", "crate", "std"])
}

fn without_whitespace(text: &str) -> String {
    text.chars().filter(|c| !c.is_whitespace()).collect()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::super::test_repo::{TestRepo, repository_root};
    use super::*;

    fn sim_crates() -> TestRepo {
        let repo = TestRepo::new();
        for crate_dir in SIM_CRATES {
            repo.copy_from_repository(&format!("{crate_dir}/src"));
        }
        repo
    }

    fn messages(repo: &TestRepo) -> Vec<(PathBuf, String)> {
        check(repo.root())
            .unwrap()
            .into_iter()
            .map(|violation| (violation.path, violation.message))
            .collect()
    }

    #[test]
    fn real_simulation_crates_are_pure() {
        assert_eq!(check(repository_root()).unwrap(), Vec::new());
    }

    #[test]
    fn extern_crate_std_fails_in_any_module() {
        let repo = sim_crates();
        repo.write("crates/ash-sim/src/world.rs", "extern crate std as s;\n");
        repo.append("crates/ash-core/src/lib.rs", "extern  crate\n  std;\n");
        let paths: Vec<PathBuf> = messages(&repo).into_iter().map(|(path, _)| path).collect();
        assert_eq!(
            paths,
            [
                PathBuf::from("crates/ash-core/src/lib.rs"),
                PathBuf::from("crates/ash-sim/src/world.rs"),
            ]
        );
    }

    #[test]
    fn extern_crate_std_in_a_comment_passes() {
        let repo = sim_crates();
        repo.append(
            "crates/ash-sim/src/lib.rs",
            "// never write extern crate std;\n",
        );
        assert_eq!(messages(&repo), Vec::new());
    }

    #[test]
    fn missing_no_std_fails() {
        let repo = sim_crates();
        repo.replace(
            "crates/ash-sim/src/lib.rs",
            "#![cfg_attr(not(test), no_std)]\n",
            "",
        );
        assert_eq!(
            messages(&repo),
            [(
                PathBuf::from("crates/ash-sim/src/lib.rs"),
                "missing crate attribute `#![cfg_attr(not(test), no_std)]`".into()
            )]
        );
    }

    #[test]
    fn lint_dropped_from_deny_fails() {
        let repo = sim_crates();
        repo.replace(
            "crates/ash-core/src/lib.rs",
            "clippy::float_arithmetic, ",
            "",
        );
        let messages: Vec<String> = messages(&repo).into_iter().map(|(_, m)| m).collect();
        assert_eq!(messages.len(), 2, "{messages:?}");
        assert!(messages[0].starts_with("missing crate attribute `#![deny("));
        assert!(messages[1].ends_with("overrides a simulation ban"));
    }

    #[test]
    fn crate_level_allow_of_a_banned_lint_fails() {
        let repo = sim_crates();
        repo.append(
            "crates/ash-sim/src/lib.rs",
            "#![allow(clippy::float_arithmetic, reason = \"x\")]\n",
        );
        let messages = messages(&repo);
        assert_eq!(messages.len(), 1);
        assert!(messages[0].1.ends_with("overrides a simulation ban"));
    }
}
