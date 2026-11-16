//! `cargo xtask setup`: installs the git hooks. Each hook file is a generated
//! launcher that calls `cargo xtask hook <name>`: the logic stays in Rust,
//! and the launcher is never committed.

use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

const HOOKS: &[&str] = &["commit-msg", "pre-commit"];

/// Writes the launchers into the hooks directory of the checkout at `root`
/// and returns their paths. Each run rewrites the same content, so setup
/// can run again.
pub fn install(root: &Path) -> io::Result<Vec<PathBuf>> {
    // Unlike `.git/hooks`, this also works from a worktree, where `.git` is a file.
    let output = Command::new("git")
        .args(["rev-parse", "--git-path", "hooks"])
        .current_dir(root)
        .output()?;
    if !output.status.success() {
        return Err(io::Error::other(format!(
            "git rev-parse --git-path hooks failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    let printed = String::from_utf8(output.stdout).map_err(io::Error::other)?;
    // Relative to `root` in a main checkout, absolute in a worktree.
    let dir = root.join(printed.trim_end());
    std::fs::create_dir_all(&dir)?;
    HOOKS
        .iter()
        .map(|hook| {
            let path = dir.join(hook);
            std::fs::write(&path, launcher(hook))?;
            make_executable(&path)?;
            Ok(path)
        })
        .collect()
}

fn launcher(hook: &str) -> String {
    format!("#!/bin/sh\nexec cargo xtask hook {hook} \"$@\"\n")
}

/// Git ignores a hook without the executable bit.
#[cfg(unix)]
fn make_executable(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
}

#[cfg(not(unix))]
fn make_executable(_: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::process::Output;

    use super::super::verify::test_repo::{TestRepo, git, git_ok, isolate_git};
    use super::super::verify::toolchain::cargo;
    use super::*;

    /// A copy of the workspace with the real `xtask`, so the installed hooks
    /// run the code under test. The lockfile pins what is already downloaded.
    fn workspace_with_xtask() -> TestRepo {
        let repo = TestRepo::new();
        for path in [
            ".cargo",
            ".gitignore",
            "Cargo.lock",
            "Cargo.toml",
            "rust-toolchain.toml",
            "crates",
            "xtask",
        ] {
            repo.copy_from_repository(path);
        }
        repo
    }

    /// Runs `program` in `repo` with git isolated, offline, and with a build
    /// directory of its own; the git hooks inherit this environment.
    fn run(repo: &TestRepo, program: &str, args: &[&str]) -> Output {
        isolate_git(&mut Command::new(program))
            .args(args)
            .current_dir(repo.root())
            .env("CARGO_TARGET_DIR", repo.root().join("target"))
            .env("CARGO_NET_OFFLINE", "true")
            .output()
            .unwrap()
    }

    fn commit(repo: &TestRepo, message: &str) -> (bool, String) {
        let output = run(repo, "git", &["commit", "--quiet", "--message", message]);
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        (output.status.success(), stderr)
    }

    #[test]
    fn installed_hooks_refuse_bad_commits() {
        let repo = workspace_with_xtask();
        git_ok(&repo, &["init", "--quiet"]);
        git_ok(&repo, &["add", "--all"]);
        git_ok(&repo, &["commit", "--quiet", "--message", "Initial commit"]);
        let cargo = cargo();
        let cargo = cargo.to_str().unwrap();
        for _ in 0..2 {
            let output = run(&repo, cargo, &["xtask", "setup"]);
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        repo.write("docs/notes.md", "Notes.\n");
        git_ok(&repo, &["add", "docs/notes.md"]);
        let (accepted, stderr) = commit(&repo, "add notes.");
        assert!(!accepted && stderr.contains("commit-msg"), "{stderr}");
        let (accepted, stderr) = commit(&repo, "Add notes");
        assert!(accepted, "{stderr}");

        repo.append(
            "crates/ash-core/src/lib.rs",
            "pub fn widen(x: u8) -> u32 {\n    x as u32\n}\n",
        );
        git_ok(&repo, &["add", "crates/ash-core/src/lib.rs"]);
        let (accepted, stderr) = commit(&repo, "Widen bytes");
        assert!(!accepted && stderr.contains("as_conversions"), "{stderr}");
        let head = git(&repo, &["log", "--format=%s"]);
        assert_eq!(
            String::from_utf8_lossy(&head.stdout),
            "Add notes\nInitial commit\n"
        );
    }
}
