//! The Claude Code `PreToolUse` hook on `Edit|Write|NotebookEdit`: refuses an
//! edit to a constitution path (plan section 8). Bash gets around it, so it
//! is a reminder; the CI protected-path check is the authority.
//!
//! A path is judged relative to the root of the checkout that contains it,
//! against that checkout's list. Agents work in worktrees under
//! `.claude/worktrees/`: judged from the main checkout, every file in a
//! worktree would fall under `.claude/`.

use std::io;
use std::path::{Component, Path, PathBuf};

use serde_json::Value;

/// The protected-path list, relative to the checkout root.
pub const LIST: &str = ".claude/protected-paths.txt";

#[derive(Debug, PartialEq, Eq)]
pub enum Verdict {
    Allow,
    /// Refused; the message tells Claude why.
    Block(String),
}

/// Judges the hook input JSON that Claude Code sends on stdin.
pub fn judge(input: &str) -> Result<Verdict, String> {
    let input: Value =
        serde_json::from_str(input).map_err(|error| format!("hook input is not JSON: {error}"))?;
    let tool_input = &input["tool_input"];
    let path = tool_input["file_path"]
        .as_str()
        .or_else(|| tool_input["notebook_path"].as_str())
        .ok_or("hook input holds neither `tool_input.file_path` nor `tool_input.notebook_path`")?;
    // Write and Edit paths arrive absolute; the join keeps a relative
    // notebook path working.
    let path = normalize(&Path::new(input["cwd"].as_str().unwrap_or("")).join(path));
    // The file may not exist yet: only its ancestors are looked at.
    let Some(root) = path
        .ancestors()
        .skip(1)
        .find(|dir| dir.join(".git").exists())
    else {
        return Ok(Verdict::Allow);
    };
    let list = match std::fs::read_to_string(root.join(LIST)) {
        Ok(list) => list,
        // Another project, or a checkout from before the list existed.
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Verdict::Allow),
        Err(error) => return Err(format!("{}: {error}", root.join(LIST).display())),
    };
    let relative = slash_separated(path.strip_prefix(root).unwrap_or(&path));
    Ok(match protecting_entry(&list, &relative) {
        Some(entry) => Verdict::Block(format!(
            "`{relative}` is a constitution path (entry `{entry}` of {LIST}): only the \
             human edits it. Stop and report the need instead (plan 0000, section 8)."
        )),
        None => Verdict::Allow,
    })
}

/// Resolves `.` and `..` without touching the disk.
fn normalize(path: &Path) -> PathBuf {
    let mut normal = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normal.pop();
            }
            other => normal.push(other),
        }
    }
    normal
}

/// The list uses `/` on every platform.
fn slash_separated(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn protecting_entry<'a>(list: &'a str, path: &str) -> Option<&'a str> {
    list.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .find(|entry| covers(entry, path))
}

/// Entry forms, as documented at the top of the list.
fn covers(entry: &str, path: &str) -> bool {
    if let Some(name) = entry.strip_prefix("**/") {
        path.rsplit('/').next() == Some(name)
    } else if entry.ends_with('/') {
        path.starts_with(entry)
    } else {
        path == entry
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::verify::test_repo::{TestRepo, repository_root};

    /// A checkout holding a copy of the real list.
    fn checkout(repo: &TestRepo, relative: &str) -> PathBuf {
        let root = repo.root().join(relative);
        std::fs::create_dir_all(root.join(".git")).unwrap();
        let list = std::fs::read_to_string(repository_root().join(LIST)).unwrap();
        repo.write(&format!("{relative}/{LIST}"), &list);
        root
    }

    fn input(tool: &str, field: &str, path: &Path, cwd: &Path) -> String {
        json!({
            "hook_event_name": "PreToolUse",
            "cwd": cwd,
            "tool_name": tool,
            "tool_input": { field: path, "content": "x" },
        })
        .to_string()
    }

    fn judge_write(root: &Path, relative: &str) -> Verdict {
        judge(&input("Write", "file_path", &root.join(relative), root)).unwrap()
    }

    fn assert_blocked(root: &Path, relative: &str) {
        let verdict = judge_write(root, relative);
        assert!(
            matches!(&verdict, Verdict::Block(message) if message.contains(relative)),
            "{relative}: {verdict:?}"
        );
    }

    /// A path the entry covers, for each entry form of the list.
    fn covered_path(entry: &str) -> String {
        let path = entry.replace("**/", "crates/ash-sim/");
        if path.ends_with('/') {
            path + "new/file.md"
        } else {
            path
        }
    }

    fn real_entries() -> Vec<String> {
        std::fs::read_to_string(repository_root().join(LIST))
            .unwrap()
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(str::to_owned)
            .collect()
    }

    #[test]
    fn every_entry_of_the_list_is_blocked() {
        let repo = TestRepo::new();
        let root = checkout(&repo, "main");
        let entries = real_entries();
        assert!(!entries.is_empty());
        for entry in entries {
            assert_blocked(&root, &covered_path(&entry));
        }
    }

    #[test]
    fn every_constitution_item_of_section_8_is_blocked() {
        let repo = TestRepo::new();
        let root = checkout(&repo, "main");
        for path in [
            "docs/vision/pillars.md",
            "docs/design/combat.md",
            "docs/adr/0001-data-model.md",
            "docs/perf/budgets.md",
            "Cargo.toml",
            "crates/ash-sim/clippy.toml",
            "xtask/src/verify/dependency_matrix/allowed.rs",
            "deny.toml",
            ".github/workflows/pr.yml",
            "xtask/src/hook/constitution.rs",
            "CLAUDE.md",
            "crates/ash-sim/CLAUDE.md",
            ".claude/settings.json",
            ".claude/rules/sim.md",
            "rust-toolchain.toml",
            "docs/templates/spec.md",
            LIST,
        ] {
            assert_blocked(&root, path);
        }
    }

    #[test]
    fn free_paths_are_allowed() {
        let repo = TestRepo::new();
        let root = checkout(&repo, "main");
        for path in [
            "crates/ash-sim/src/world.rs",
            "crates/ash-sim/Cargo.toml",
            "docs/specs/0001-walking-skeleton.md",
            "xtask/src/verify/fast.rs",
        ] {
            assert_eq!(judge_write(&root, path), Verdict::Allow, "{path}");
        }
    }

    #[test]
    fn worktree_paths_are_judged_from_the_worktree_root() {
        let repo = TestRepo::new();
        let main = checkout(&repo, "main");
        let worktree = main.join(".claude/worktrees/t3");
        std::fs::create_dir_all(&worktree).unwrap();
        std::fs::write(worktree.join(".git"), "gitdir: elsewhere\n").unwrap();
        std::fs::create_dir_all(worktree.join(".claude")).unwrap();
        std::fs::copy(main.join(LIST), worktree.join(LIST)).unwrap();
        // The session runs in the main checkout; the file lives in the worktree.
        let free = worktree.join("crates/ash-sim/src/lib.rs");
        let protected = worktree.join("docs/adr/0001-data-model.md");
        assert_eq!(
            judge(&input("Edit", "file_path", &free, &main)).unwrap(),
            Verdict::Allow
        );
        assert!(matches!(
            judge(&input("Write", "file_path", &protected, &main)).unwrap(),
            Verdict::Block(_)
        ));
    }

    #[test]
    fn notebook_edits_are_judged() {
        let repo = TestRepo::new();
        let root = checkout(&repo, "main");
        let notebook = root.join("docs/design/economy.ipynb");
        assert!(matches!(
            judge(&input("NotebookEdit", "notebook_path", &notebook, &root)).unwrap(),
            Verdict::Block(_)
        ));
    }

    #[test]
    fn parent_components_do_not_hide_a_protected_path() {
        let repo = TestRepo::new();
        let root = checkout(&repo, "main");
        assert!(matches!(
            judge_write(&root, "crates/../docs/adr/0001-data-model.md"),
            Verdict::Block(_)
        ));
    }
}
