//! `cargo xtask setup`: installs the git hooks. Each hook file is a generated
//! launcher that calls `cargo xtask hook <name>`: the logic stays in Rust,
//! and the launcher is never committed.

use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

const HOOKS: &[&str] = &["commit-msg", "pre-commit"];

/// Writes the launchers into the hooks directory of the checkout at `root`
/// and returns their paths. Each run rewrites the same content, so setup
/// can run again. A hook that is not the launcher is never replaced, and a
/// hooks directory outside the git directory is refused: a global
/// `core.hooksPath` is shared by every repository of the user.
pub fn install(root: &Path) -> io::Result<Vec<PathBuf>> {
    // Unlike `.git/hooks`, this also works from a worktree, where `.git` is a file.
    let output = Command::new("git")
        .args(["rev-parse", "--git-common-dir", "--git-path", "hooks"])
        .current_dir(root)
        .output()?;
    if !output.status.success() {
        return Err(io::Error::other(format!(
            "git rev-parse failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    let printed = String::from_utf8(output.stdout).map_err(io::Error::other)?;
    let mut lines = printed.lines();
    let (Some(common_dir), Some(hooks_dir)) = (lines.next(), lines.next()) else {
        return Err(io::Error::other("unexpected `git rev-parse` output"));
    };
    // Relative to `root` in a main checkout, absolute in a worktree.
    let (common_dir, dir) = (root.join(common_dir), root.join(hooks_dir));
    if !dir.starts_with(&common_dir) {
        return Err(io::Error::other(format!(
            "the hooks directory {} is outside the git directory {}: core.hooksPath \
             is set. Unset it, or call `cargo xtask hook <name>` from your own hooks.",
            dir.display(),
            common_dir.display()
        )));
    }
    for hook in HOOKS {
        let path = dir.join(hook);
        match std::fs::read_to_string(&path) {
            Ok(existing) if existing != launcher(hook) => {
                return Err(io::Error::other(format!(
                    "{} exists and is not the xtask launcher: move it away, then run \
                     setup again.",
                    path.display()
                )));
            }
            Err(error) if error.kind() != io::ErrorKind::NotFound => return Err(error),
            _ => {}
        }
    }
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

    fn git_repo() -> TestRepo {
        let repo = TestRepo::new();
        git_ok(&repo, &["init", "--quiet"]);
        repo
    }

    #[test]
    fn a_hook_that_is_not_the_launcher_is_kept() {
        let repo = git_repo();
        let foreign = "#!/bin/sh\nmake lint\n";
        repo.write(".git/hooks/pre-commit", foreign);
        let error = install(repo.root()).unwrap_err();
        assert!(error.to_string().contains("pre-commit"), "{error}");
        let hooks = repo.root().join(".git/hooks");
        assert_eq!(
            std::fs::read_to_string(hooks.join("pre-commit")).unwrap(),
            foreign
        );
        assert!(!hooks.join("commit-msg").exists());
    }

    #[test]
    fn a_hooks_directory_outside_the_repository_is_refused() {
        let repo = git_repo();
        let elsewhere = TestRepo::new();
        git_ok(
            &repo,
            &[
                "config",
                "core.hooksPath",
                elsewhere.root().to_str().unwrap(),
            ],
        );
        let error = install(repo.root()).unwrap_err();
        assert!(error.to_string().contains("core.hooksPath"), "{error}");
        assert_eq!(std::fs::read_dir(elsewhere.root()).unwrap().count(), 0);
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

        repo.append("crates/ash-core/src/lib.rs", "pub fn answer() -> u8 {42}\n");
        git_ok(&repo, &["add", "crates/ash-core/src/lib.rs"]);
        let (accepted, stderr) = commit(&repo, "Add the answer");
        assert!(!accepted && stderr.contains("Diff in"), "{stderr}");
        git_ok(&repo, &["reset", "--hard", "--quiet"]);

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
