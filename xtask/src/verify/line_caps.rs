//! Instruction files are loaded into every agent context, so their size is capped.

use std::io;
use std::path::{Path, PathBuf};

use super::{Violation, markdown_files, read_dir_or_empty, read_file, relative};

const ROOT_CLAUDE_MD_CAP: usize = 150;
const RULE_FILE_CAP: usize = 100;
const CRATE_CLAUDE_MD_CAP: usize = 80;

pub fn check(root: &Path) -> io::Result<Vec<Violation>> {
    let mut violations = Vec::new();

    let root_claude_md = root.join("CLAUDE.md");
    if root_claude_md.is_file() {
        check_file(root, &root_claude_md, ROOT_CLAUDE_MD_CAP, &mut violations)?;
    } else {
        violations.push(Violation {
            path: "CLAUDE.md".into(),
            message: "missing".into(),
        });
    }

    for rule in markdown_files(&root.join(".claude/rules"))? {
        check_file(root, &rule, RULE_FILE_CAP, &mut violations)?;
    }

    for crate_claude_md in crate_claude_mds(&root.join("crates"))? {
        check_file(root, &crate_claude_md, CRATE_CLAUDE_MD_CAP, &mut violations)?;
    }

    Ok(violations)
}

fn check_file(
    root: &Path,
    path: &Path,
    cap: usize,
    violations: &mut Vec<Violation>,
) -> io::Result<()> {
    let count = read_file(root, path)?.lines().count();
    if count > cap {
        violations.push(Violation {
            path: relative(root, path),
            message: format!("{count} lines, cap is {cap}"),
        });
    }
    Ok(())
}

fn crate_claude_mds(crates: &Path) -> io::Result<Vec<PathBuf>> {
    Ok(read_dir_or_empty(crates)?
        .into_iter()
        .map(|crate_dir| crate_dir.join("CLAUDE.md"))
        .filter(|candidate| candidate.is_file())
        .collect())
}

#[cfg(test)]
mod tests {
    use super::super::test_repo::{TestRepo, lines};
    use super::*;

    fn messages(repo: &TestRepo) -> Vec<String> {
        check(repo.root())
            .unwrap()
            .into_iter()
            .map(|violation| violation.to_string())
            .collect()
    }

    #[test]
    fn files_at_their_cap_pass() {
        let repo = TestRepo::new();
        repo.write("CLAUDE.md", &lines(150));
        repo.write(".claude/rules/sim.md", &lines(100));
        repo.write("crates/ash-core/CLAUDE.md", &lines(80));
        assert_eq!(messages(&repo), Vec::<String>::new());
    }

    #[test]
    fn files_one_line_over_their_cap_fail() {
        let repo = TestRepo::new();
        repo.write("CLAUDE.md", &lines(151));
        repo.write(".claude/rules/sim.md", &lines(101));
        repo.write("crates/ash-core/CLAUDE.md", &lines(81));
        assert_eq!(
            messages(&repo),
            [
                "CLAUDE.md: 151 lines, cap is 150",
                ".claude/rules/sim.md: 101 lines, cap is 100",
                "crates/ash-core/CLAUDE.md: 81 lines, cap is 80",
            ]
        );
    }

    #[test]
    fn nested_rule_file_over_cap_fails() {
        let repo = TestRepo::new();
        repo.write("CLAUDE.md", &lines(1));
        repo.write(".claude/rules/sim/scheduler.md", &lines(101));
        assert_eq!(
            messages(&repo),
            [".claude/rules/sim/scheduler.md: 101 lines, cap is 100"]
        );
    }

    #[test]
    fn unreadable_file_error_names_the_file() {
        let repo = TestRepo::new();
        repo.write("CLAUDE.md", &lines(1));
        let path = repo.root().join(".claude/rules/bad.md");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, [0xff, 0xfe]).unwrap();
        let error = check(repo.root()).unwrap_err();
        assert!(
            error.to_string().contains(".claude/rules/bad.md"),
            "{error}"
        );
    }

    // Unix permissions are the only portable way to make a directory unreadable.
    #[cfg(unix)]
    #[test]
    fn unreadable_directory_error_names_the_directory() {
        use std::os::unix::fs::PermissionsExt;
        let repo = TestRepo::new();
        repo.write("CLAUDE.md", &lines(1));
        let locked = repo.root().join(".claude/rules/locked");
        std::fs::create_dir_all(&locked).unwrap();
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
        let result = check(repo.root());
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();
        let error = result.unwrap_err();
        assert!(
            error.to_string().contains(".claude/rules/locked"),
            "{error}"
        );
    }

    #[test]
    fn missing_root_claude_md_fails() {
        let repo = TestRepo::new();
        assert_eq!(messages(&repo), ["CLAUDE.md: missing"]);
    }
}
