//! CI relies only on the exit code of `cargo xtask constitution`: 0 passes,
//! 1 means protected paths are touched without approval, 2 means the check
//! could not judge. Only the real binary proves which one it returns.
//!
//! The binary judges the repository that holds the xtask sources, so each
//! test points `GIT_DIR` at a throwaway repository instead.

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};

static NEXT: AtomicU32 = AtomicU32::new(0);

/// Three commits: the list, then a free path, then a protected path.
struct Repository {
    root: PathBuf,
    base: String,
    free: String,
    protected: String,
}

impl Repository {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "xtask-constitution-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join(".claude")).unwrap();
        let mut repository = Self {
            root,
            base: String::new(),
            free: String::new(),
            protected: String::new(),
        };
        repository.git(&["init", "--quiet"]);
        let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap();
        let list = ".claude/protected-paths.txt";
        std::fs::copy(source.join(list), repository.root.join(list)).unwrap();
        repository.base = repository.commit(list);
        repository.free = repository.commit("crates/ash-sim/src/world.rs");
        repository.protected = repository.commit("docs/adr/0001-data-model.md");
        repository
    }

    fn git(&self, args: &[&str]) -> String {
        let output = Command::new("git")
            .args(args)
            .current_dir(&self.root)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_AUTHOR_NAME", "xtask test")
            .env("GIT_AUTHOR_EMAIL", "xtask@test.invalid")
            .env("GIT_COMMITTER_NAME", "xtask test")
            .env("GIT_COMMITTER_EMAIL", "xtask@test.invalid")
            .env_remove("GIT_DIR")
            .env_remove("GIT_INDEX_FILE")
            .env_remove("GIT_WORK_TREE")
            .output()
            .unwrap();
        assert!(output.status.success(), "git {args:?} failed");
        String::from_utf8(output.stdout).unwrap().trim().to_owned()
    }

    /// Commits `relative` (written if absent) and returns the new commit.
    fn commit(&self, relative: &str) -> String {
        let path = self.root.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        if !path.exists() {
            std::fs::write(&path, "x\n").unwrap();
        }
        self.git(&["add", relative]);
        self.git(&["commit", "--quiet", "-m", relative]);
        self.git(&["rev-parse", "HEAD"])
    }

    fn exit_code(&self, args: &[&str]) -> Option<i32> {
        Command::new(env!("CARGO_BIN_EXE_xtask"))
            .arg("constitution")
            .args(args)
            .env("GIT_DIR", self.root.join(".git"))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap()
            .code()
    }
}

impl Drop for Repository {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[test]
fn protected_path_without_approval_exits_1() {
    let repository = Repository::new();
    assert_eq!(
        repository.exit_code(&["--base", &repository.free, "--head", &repository.protected]),
        Some(1)
    );
}

#[test]
fn protected_path_with_approval_exits_0() {
    let repository = Repository::new();
    assert_eq!(
        repository.exit_code(&[
            "--base",
            &repository.free,
            "--head",
            &repository.protected,
            "--approved"
        ]),
        Some(0)
    );
}

#[test]
fn free_path_exits_0() {
    let repository = Repository::new();
    assert_eq!(
        repository.exit_code(&["--base", &repository.base, "--head", &repository.free]),
        Some(0)
    );
}

#[test]
fn unknown_base_exits_2() {
    let repository = Repository::new();
    assert_eq!(
        repository.exit_code(&["--base", &"0".repeat(40), "--head", &repository.free]),
        Some(2)
    );
}

#[test]
fn missing_head_exits_2() {
    let repository = Repository::new();
    assert_eq!(repository.exit_code(&["--base", &repository.base]), Some(2));
}
