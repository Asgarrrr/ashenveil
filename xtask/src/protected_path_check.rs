//! The protected-path check, `cargo xtask constitution`: the CI authority
//! that the Claude Code hook only reminds of (plan 0000, section 8 and the
//! T4 design amendment). It judges every file that a pull request changes,
//! whatever tool changed it.
//!
//! The list comes from the base, never from the head or the working tree:
//! a pull request cannot weaken the list that judges it.

use std::fmt;
use std::path::Path;
use std::process::Command;

use crate::hook::constitution::{LIST, protecting_entry};

/// A changed file that an entry of the list protects.
#[derive(Debug, PartialEq, Eq)]
pub struct Touched {
    pub path: String,
    pub entry: String,
}

impl fmt::Display for Touched {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "`{}` (entry `{}` of {LIST})", self.path, self.entry)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Verdict {
    /// No protected path is touched, or the touched ones are approved.
    Pass(Vec<Touched>),
    /// Protected paths are touched without approval.
    Fail(Vec<Touched>),
}

/// Judges the files that `head` changes since its merge base with `base`.
/// A rename counts as a deletion plus an addition.
pub fn check(root: &Path, base: &str, head: &str, approved: bool) -> Result<Verdict, String> {
    let list = git(root, &["show", &format!("{base}:{LIST}")])?;
    let changed = git(
        root,
        &[
            "diff",
            "--name-only",
            "--no-renames",
            "-z",
            &format!("{base}...{head}"),
            "--",
        ],
    )?;
    let touched: Vec<Touched> = changed
        .split('\0')
        .filter(|path| !path.is_empty())
        .filter_map(|path| {
            protecting_entry(&list, path).map(|entry| Touched {
                path: path.to_owned(),
                entry: entry.to_owned(),
            })
        })
        .collect();
    Ok(if touched.is_empty() || approved {
        Verdict::Pass(touched)
    } else {
        Verdict::Fail(touched)
    })
}

/// Runs git in `root`; its stdout, or an error that names the command.
fn git(root: &Path, args: &[&str]) -> Result<String, String> {
    let command = format!("git {}", args.join(" "));
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|error| format!("{command}: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "{command}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    String::from_utf8(output.stdout).map_err(|error| format!("{command}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verify::test_repo::{TestRepo, git, git_ok};

    /// A repository whose first commit holds a list protecting `docs/adr/`.
    /// The list does not protect itself, so that a test can change it.
    fn repository() -> TestRepo {
        let repo = TestRepo::new();
        git_ok(&repo, &["init", "--quiet"]);
        repo.write(LIST, "docs/adr/\n");
        repo.write("README.md", "readme\n");
        commit(&repo);
        repo
    }

    /// Commits every change; returns the commit id.
    fn commit(repo: &TestRepo) -> String {
        git_ok(repo, &["add", "--all"]);
        git_ok(repo, &["commit", "--quiet", "--message", "Change"]);
        commit_id(repo)
    }

    fn commit_id(repo: &TestRepo) -> String {
        let output = git(repo, &["rev-parse", "HEAD"]);
        String::from_utf8(output.stdout).unwrap().trim().to_owned()
    }

    fn touched(path: &str) -> Touched {
        Touched {
            path: path.to_owned(),
            entry: "docs/adr/".to_owned(),
        }
    }

    #[test]
    fn a_protected_path_fails_without_approval() {
        let repo = repository();
        let base = commit_id(&repo);
        repo.write("docs/adr/x.md", "decision\n");
        let head = commit(&repo);
        assert_eq!(
            check(repo.root(), &base, &head, false).unwrap(),
            Verdict::Fail(vec![touched("docs/adr/x.md")])
        );
    }

    #[test]
    fn approval_passes_and_still_lists_the_protected_paths() {
        let repo = repository();
        let base = commit_id(&repo);
        repo.write("docs/adr/x.md", "decision\n");
        let head = commit(&repo);
        assert_eq!(
            check(repo.root(), &base, &head, true).unwrap(),
            Verdict::Pass(vec![touched("docs/adr/x.md")])
        );
    }

    #[test]
    fn free_paths_pass() {
        let repo = repository();
        let base = commit_id(&repo);
        repo.write("docs/specs/0001-x.md", "spec\n");
        repo.append("README.md", "more\n");
        let head = commit(&repo);
        assert_eq!(
            check(repo.root(), &base, &head, false).unwrap(),
            Verdict::Pass(Vec::new())
        );
    }

    /// Rename detection would report only the free destination.
    #[test]
    fn a_rename_out_of_a_protected_directory_fails() {
        let repo = repository();
        repo.write("docs/adr/x.md", "decision\n");
        let base = commit(&repo);
        git_ok(&repo, &["mv", "docs/adr/x.md", "x.md"]);
        let head = commit(&repo);
        assert_eq!(
            check(repo.root(), &base, &head, false).unwrap(),
            Verdict::Fail(vec![touched("docs/adr/x.md")])
        );
    }

    #[test]
    fn a_head_that_unprotects_a_path_is_judged_by_the_base_list() {
        let repo = repository();
        let base = commit_id(&repo);
        repo.write(LIST, "# emptied\n");
        repo.write("docs/adr/x.md", "decision\n");
        let head = commit(&repo);
        assert_eq!(
            check(repo.root(), &base, &head, false).unwrap(),
            Verdict::Fail(vec![touched("docs/adr/x.md")])
        );
    }

    #[test]
    fn case_does_not_hide_a_protected_path() {
        let repo = repository();
        let base = commit_id(&repo);
        repo.write("DOCS/ADR/x.md", "decision\n");
        let head = commit(&repo);
        assert_eq!(
            check(repo.root(), &base, &head, false).unwrap(),
            Verdict::Fail(vec![touched("DOCS/ADR/x.md")])
        );
    }

    /// Changes that reached the base after the fork are not the head's.
    #[test]
    fn changes_are_counted_from_the_merge_base() {
        let repo = repository();
        git_ok(&repo, &["checkout", "--quiet", "-b", "pull-request"]);
        repo.write("notes/x.md", "note\n");
        let head = commit(&repo);
        git_ok(&repo, &["checkout", "--quiet", "-"]);
        repo.write("docs/adr/y.md", "decision\n");
        let base = commit(&repo);
        assert_eq!(
            check(repo.root(), &base, &head, false).unwrap(),
            Verdict::Pass(Vec::new())
        );
    }

    /// Unlike the hook, the check fails closed: no list is not an empty list.
    #[test]
    fn a_base_without_the_list_is_an_error() {
        let repo = TestRepo::new();
        git_ok(&repo, &["init", "--quiet"]);
        repo.write("README.md", "readme\n");
        let base = commit(&repo);
        repo.write("docs/adr/x.md", "decision\n");
        let head = commit(&repo);
        assert!(check(repo.root(), &base, &head, false).is_err());
    }
}
