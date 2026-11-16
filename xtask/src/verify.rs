//! The checks behind `cargo xtask verify`. `verify` holds the file checks
//! that need no compiler; `dependency_matrix` and `toolchain` call cargo;
//! `fast` is the commit-time subset.

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

pub mod dependency_matrix;
pub mod fast;
mod file_names;
mod line_caps;
pub mod lint_inheritance;
mod sections;
pub mod toolchain;

/// Build output and installed dependencies, never written by us.
const SKIPPED_DIRS: &[&str] = &[".git", "target", "node_modules"];

#[derive(Debug, PartialEq, Eq)]
pub struct Violation {
    /// Path relative to the repository root.
    pub path: PathBuf,
    pub message: String,
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path.display(), self.message)
    }
}

pub fn verify(root: &Path) -> io::Result<Vec<Violation>> {
    let mut violations = line_caps::check(root)?;
    violations.extend(sections::check(root)?);
    violations.extend(file_names::check(root)?);
    Ok(violations)
}

/// Files below `dir` whose name passes `keep`, sorted. A subdirectory holding
/// its own `.git` is another checkout (an agent worktree under
/// `.claude/worktrees/`): it is skipped, like build output.
fn repository_files(dir: &Path, keep: &dyn Fn(&str) -> bool) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for path in read_dir_or_empty(dir)? {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        if path.is_dir() {
            if !SKIPPED_DIRS.contains(&name) && !path.join(".git").exists() {
                files.extend(repository_files(&path, keep)?);
            }
        } else if keep(name) {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

/// Markdown files anywhere below `dir`, sorted for stable output.
fn markdown_files(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for path in read_dir_or_empty(dir)? {
        if path.is_dir() {
            files.extend(markdown_files(&path)?);
        } else if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
        {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

/// Entries of `dir`. A missing directory yields none: it is not created until needed.
fn read_dir_or_empty(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let name_dir =
        |error: io::Error| io::Error::new(error.kind(), format!("{}: {error}", dir.display()));
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(name_dir(error)),
    };
    let mut paths = entries
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<io::Result<Vec<_>>>()
        .map_err(name_dir)?;
    paths.sort();
    Ok(paths)
}

/// Reads a file; the error names it, since a bare I/O error does not.
fn read_file(root: &Path, path: &Path) -> io::Result<String> {
    std::fs::read_to_string(path).map_err(|error| {
        io::Error::new(
            error.kind(),
            format!("{}: {error}", relative(root, path).display()),
        )
    })
}

fn relative(root: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(root).unwrap_or(path).to_path_buf()
}

#[cfg(test)]
pub(crate) mod test_repo {
    use std::path::{Path, PathBuf};
    use std::process::{Command, Output};
    use std::sync::atomic::{AtomicU32, Ordering};

    static NEXT: AtomicU32 = AtomicU32::new(0);

    /// Throwaway repository tree, deleted on drop.
    pub struct TestRepo {
        root: PathBuf,
    }

    impl TestRepo {
        pub fn new() -> Self {
            Self::at(std::env::temp_dir().join(format!(
                "xtask-test-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            )))
        }

        /// Clears `root` first: a killed run leaves its tree behind, and
        /// process ids get reused.
        pub fn at(root: PathBuf) -> Self {
            let _ = std::fs::remove_dir_all(&root);
            std::fs::create_dir_all(&root).unwrap();
            Self { root }
        }

        pub fn root(&self) -> &Path {
            &self.root
        }

        pub fn write(&self, relative: &str, content: &str) {
            let path = self.root.join(relative);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, content).unwrap();
        }

        pub fn append(&self, relative: &str, content: &str) {
            let path = self.root.join(relative);
            let existing = std::fs::read_to_string(&path).unwrap();
            std::fs::write(path, existing + content).unwrap();
        }

        /// Replaces `from`, which must be present, in the file at `relative`.
        pub fn replace(&self, relative: &str, from: &str, to: &str) {
            let path = self.root.join(relative);
            let existing = std::fs::read_to_string(&path).unwrap();
            assert!(existing.contains(from), "{relative} lacks {from:?}");
            std::fs::write(path, existing.replace(from, to)).unwrap();
        }

        /// Copies `relative`, a file or a directory tree, from the real repository.
        pub fn copy_from_repository(&self, relative: &str) {
            copy_tree(&repository_root().join(relative), &self.root.join(relative));
        }
    }

    pub fn repository_root() -> &'static Path {
        Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
    }

    const XTASK_STUB_MANIFEST: &str = "[package]\nname = \"xtask\"\nversion = \"0.0.0\"\n\
        edition.workspace = true\npublish.workspace = true\n\n[lints]\nworkspace = true\n";

    /// A copy of the real workspace manifest, toolchain file and crates, with
    /// a dependency-free `xtask` stub so the copy builds offline.
    pub fn scratch_workspace() -> TestRepo {
        let repo = TestRepo::new();
        for path in ["Cargo.toml", "rust-toolchain.toml", "crates"] {
            repo.copy_from_repository(path);
        }
        repo.write("xtask/Cargo.toml", XTASK_STUB_MANIFEST);
        repo.write("xtask/src/main.rs", "fn main() {}\n");
        repo
    }

    fn copy_tree(from: &Path, to: &Path) {
        if from.is_dir() {
            for entry in std::fs::read_dir(from).unwrap() {
                let entry = entry.unwrap();
                copy_tree(&entry.path(), &to.join(entry.file_name()));
            }
        } else {
            std::fs::create_dir_all(to.parent().unwrap()).unwrap();
            std::fs::copy(from, to).unwrap();
        }
    }

    /// Isolates a command from the user's git config, which may set
    /// `core.hooksPath` or signing, and from the git variables of a calling hook.
    pub fn isolate_git(command: &mut Command) -> &mut Command {
        command
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_AUTHOR_NAME", "xtask test")
            .env("GIT_AUTHOR_EMAIL", "xtask@test.invalid")
            .env("GIT_COMMITTER_NAME", "xtask test")
            .env("GIT_COMMITTER_EMAIL", "xtask@test.invalid")
            .env_remove("GIT_DIR")
            .env_remove("GIT_INDEX_FILE")
            .env_remove("GIT_WORK_TREE")
    }

    /// Runs git in `repo`, isolated.
    pub fn git(repo: &TestRepo, args: &[&str]) -> Output {
        isolate_git(&mut Command::new("git"))
            .args(args)
            .current_dir(repo.root())
            .output()
            .unwrap()
    }

    /// Runs git in `repo` and fails the test if git fails.
    pub fn git_ok(repo: &TestRepo, args: &[&str]) {
        let output = git(repo, args);
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    impl Drop for TestRepo {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    pub fn lines(count: usize) -> String {
        "line\n".repeat(count)
    }
}

#[cfg(test)]
mod tests {
    use super::test_repo::{TestRepo, lines, repository_root};
    use super::*;

    #[test]
    fn test_repo_starts_empty_even_if_a_stale_directory_exists() {
        let stale = std::env::temp_dir().join(format!("xtask-test-stale-{}", std::process::id()));
        std::fs::create_dir_all(&stale).unwrap();
        std::fs::write(stale.join("CLAUDE.md"), "stale").unwrap();
        let repo = TestRepo::at(stale);
        assert!(!repo.root().join("CLAUDE.md").exists());
    }

    #[test]
    fn real_repository_passes() {
        assert_eq!(verify(repository_root()).unwrap(), Vec::new());
    }

    #[test]
    fn claude_md_over_cap_and_adr_without_decision_both_fail() {
        let repo = TestRepo::new();
        repo.write("CLAUDE.md", &lines(151));
        repo.write(
            "docs/adr/0001-example.md",
            "# 0001\n## Status\n## Context\n## Alternatives considered\n## Consequences\n",
        );
        let paths: Vec<PathBuf> = verify(repo.root())
            .unwrap()
            .into_iter()
            .map(|violation| violation.path)
            .collect();
        assert_eq!(
            paths,
            [
                PathBuf::from("CLAUDE.md"),
                PathBuf::from("docs/adr/0001-example.md")
            ]
        );
    }
}
