//! The `pre-commit` git hook, also run as `cargo xtask verify --fast`: fmt
//! and clippy on the crates that the staged files touch, and nothing else,
//! so that a commit stays cheap (plan section 2, corollary 1). The full
//! `verify` runs later.
//!
//! The staged files select the crates, but fmt and clippy check the working
//! tree, not the index: a change left unstaged in a touched crate counts, and
//! a staged version that differs from the working tree is not what is checked.

use std::io;
use std::path::Path;
use std::process::Command;

use crate::verify::toolchain::{run_cargo, unexpected_metadata as unexpected, workspace_metadata};

/// Workspace packages that hold a staged file, sorted. A staged file outside
/// every package, such as a doc, touches none.
pub fn touched_packages(root: &Path) -> io::Result<Vec<String>> {
    let output = Command::new("git")
        .args(["diff", "--cached", "--name-only", "-z"])
        .current_dir(root)
        .output()?;
    if !output.status.success() {
        return Err(io::Error::other(format!(
            "git diff --cached failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    let staged = String::from_utf8(output.stdout).map_err(io::Error::other)?;
    let metadata = workspace_metadata(root)?;
    let mut packages = Vec::new();
    for package in metadata["packages"].as_array().ok_or_else(unexpected)? {
        let name = package["name"].as_str().ok_or_else(unexpected)?;
        let manifest = Path::new(package["manifest_path"].as_str().ok_or_else(unexpected)?);
        let dir = manifest.parent().ok_or_else(unexpected)?;
        packages.push((dir.strip_prefix(root).unwrap_or(dir), name));
    }
    let mut touched = Vec::new();
    for file in staged.split('\0').filter(|file| !file.is_empty()) {
        // Workspace packages do not nest: at most one directory holds the file.
        let owner = packages
            .iter()
            .find(|(dir, _)| Path::new(file).starts_with(dir));
        if let Some((_, name)) = owner
            && !touched.iter().any(|touched| touched == name)
        {
            touched.push((*name).to_owned());
        }
    }
    touched.sort();
    Ok(touched)
}

/// Runs fmt and clippy on the touched packages; true when both pass.
pub fn run(root: &Path) -> bool {
    let packages = match touched_packages(root) {
        Ok(packages) => packages,
        Err(error) => {
            eprintln!("verify --fast: {error}");
            return false;
        }
    };
    if packages.is_empty() {
        println!("verify --fast: no crate touched");
        return true;
    }
    let mut selection = Vec::new();
    for package in &packages {
        selection.extend(["--package", package.as_str()]);
    }
    let fmt = run_cargo(root, &[&["fmt", "--check"], selection.as_slice()].concat());
    let clippy = run_cargo(
        root,
        &[
            &["clippy", "--all-targets"],
            selection.as_slice(),
            &["--", "-D", "warnings"],
        ]
        .concat(),
    );
    fmt && clippy
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verify::test_repo::{git_ok, scratch_workspace};

    #[test]
    fn only_crates_holding_a_staged_file_are_touched() {
        let repo = scratch_workspace();
        git_ok(&repo, &["init", "--quiet"]);
        repo.write("crates/ash-sim/src/world.rs", "");
        repo.write("docs/notes.md", "");
        // Changed but not staged: not part of the commit.
        repo.write("crates/ash-core/src/time.rs", "");
        git_ok(
            &repo,
            &["add", "crates/ash-sim/src/world.rs", "docs/notes.md"],
        );
        assert_eq!(touched_packages(repo.root()).unwrap(), ["ash-sim"]);
    }
}
