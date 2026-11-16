//! Compares the dependencies each workspace crate declares, as reported by
//! `cargo metadata`, to the allowed matrix in `dependency_matrix/allowed.rs`.

use std::io;
use std::path::Path;

use super::toolchain::{unexpected_metadata as unexpected, workspace_metadata};
use super::{Violation, relative};

mod allowed;

pub fn check(root: &Path) -> io::Result<Vec<Violation>> {
    let metadata = workspace_metadata(root)?;
    let mut violations = Vec::new();
    for package in metadata["packages"].as_array().ok_or_else(unexpected)? {
        let name = package["name"].as_str().ok_or_else(unexpected)?;
        let manifest = relative(
            root,
            Path::new(package["manifest_path"].as_str().ok_or_else(unexpected)?),
        );
        let Some((_, allowed)) = allowed::ALLOWED.iter().find(|(krate, _)| *krate == name) else {
            violations.push(Violation {
                path: manifest,
                message: format!("`{name}` is not in the dependency matrix"),
            });
            continue;
        };
        let mut forbidden = Vec::new();
        for dependency in package["dependencies"].as_array().ok_or_else(unexpected)? {
            let dependency = dependency["name"].as_str().ok_or_else(unexpected)?;
            if !allowed.contains(&dependency) && !forbidden.contains(&dependency) {
                forbidden.push(dependency);
            }
        }
        for dependency in forbidden {
            violations.push(Violation {
                path: manifest.clone(),
                message: format!("`{name}` -> `{dependency}` is not in the dependency matrix"),
            });
        }
    }
    Ok(violations)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::super::test_repo::{TestRepo, repository_root};
    use super::*;

    const WORKSPACE: &str = "[workspace]\nresolver = \"3\"\nmembers = [\"crates/*\"]\n";

    fn add_crate(repo: &TestRepo, name: &str, dependencies: &str) {
        repo.write(
            &format!("crates/{name}/Cargo.toml"),
            &format!(
                "[package]\nname = \"{name}\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n\
                 [dependencies]\n{dependencies}"
            ),
        );
        repo.write(&format!("crates/{name}/src/lib.rs"), "");
    }

    fn violations(repo: &TestRepo) -> Vec<(PathBuf, String)> {
        check(repo.root())
            .unwrap()
            .into_iter()
            .map(|violation| (violation.path, violation.message))
            .collect()
    }

    #[test]
    fn real_workspace_matches_the_matrix() {
        assert_eq!(check(repository_root()).unwrap(), Vec::new());
    }

    #[test]
    fn forbidden_edge_fails() {
        let repo = TestRepo::new();
        repo.write("Cargo.toml", WORKSPACE);
        add_crate(&repo, "ash-core", "ash-sim = { path = \"../ash-sim\" }\n");
        add_crate(&repo, "ash-sim", "ash-core = { path = \"../ash-core\" }\n");
        assert_eq!(
            violations(&repo),
            [(
                PathBuf::from("crates/ash-core/Cargo.toml"),
                "`ash-core` -> `ash-sim` is not in the dependency matrix".to_owned()
            )]
        );
    }

    #[test]
    fn crate_missing_from_the_matrix_fails() {
        let repo = TestRepo::new();
        repo.write("Cargo.toml", WORKSPACE);
        add_crate(&repo, "ash-core", "");
        add_crate(&repo, "ash-unlisted", "");
        assert_eq!(
            violations(&repo),
            [(
                PathBuf::from("crates/ash-unlisted/Cargo.toml"),
                "`ash-unlisted` is not in the dependency matrix".to_owned()
            )]
        );
    }
}
