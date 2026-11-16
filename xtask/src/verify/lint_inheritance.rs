//! A crate without `[lints] workspace = true` escapes every workspace lint,
//! and cargo only warns about it, so `verify` refuses it.

use std::io;
use std::path::Path;

use super::toolchain::{unexpected_metadata as unexpected, workspace_metadata};
use super::{Violation, read_file, relative};

pub fn check(root: &Path) -> io::Result<Vec<Violation>> {
    let mut violations = Vec::new();
    for package in workspace_metadata(root)?["packages"]
        .as_array()
        .ok_or_else(unexpected)?
    {
        let manifest = Path::new(package["manifest_path"].as_str().ok_or_else(unexpected)?);
        if !inherits_workspace_lints(&read_file(root, manifest)?) {
            violations.push(Violation {
                path: relative(root, manifest),
                message: "missing `[lints]` table with `workspace = true`".into(),
            });
        }
    }
    Ok(violations)
}

/// Only the `[lints]` table form is recognized, the one every crate uses; any
/// other spelling fails safe and gets reported.
fn inherits_workspace_lints(manifest: &str) -> bool {
    let mut in_lints = false;
    for line in manifest.lines() {
        let line: String = line
            .split('#')
            .next()
            .unwrap_or("")
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();
        if line.starts_with('[') {
            in_lints = line == "[lints]";
        } else if in_lints && line == "workspace=true" {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::super::test_repo::{repository_root, scratch_workspace};
    use super::*;

    #[test]
    fn real_workspace_inherits_workspace_lints() {
        assert_eq!(check(repository_root()).unwrap(), Vec::new());
    }

    #[test]
    fn crate_without_workspace_lints_fails() {
        let repo = scratch_workspace();
        repo.replace(
            "crates/ash-sim/Cargo.toml",
            "[lints]\nworkspace = true\n",
            "",
        );
        let paths: Vec<PathBuf> = check(repo.root())
            .unwrap()
            .into_iter()
            .map(|violation| violation.path)
            .collect();
        assert_eq!(paths, [PathBuf::from("crates/ash-sim/Cargo.toml")]);
    }
}
