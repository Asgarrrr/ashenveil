//! A helper belongs to the concept it serves, so catch-all module files are refused.

use std::io;
use std::path::Path;

use super::{Violation, relative, repository_files};

const CATCH_ALL_FILE_NAMES: &[&str] = &["utils.rs", "helpers.rs", "common.rs", "misc.rs"];

pub fn check(root: &Path) -> io::Result<Vec<Violation>> {
    let files = repository_files(root, &|name| CATCH_ALL_FILE_NAMES.contains(&name))?;
    Ok(files
        .into_iter()
        .map(|path| Violation {
            path: relative(root, &path),
            message: "catch-all file: name the module after the concept it serves".into(),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::super::test_repo::TestRepo;
    use super::*;

    #[test]
    fn catch_all_file_name_fails() {
        let repo = TestRepo::new();
        repo.write("crates/ash-sim/src/scheduler.rs", "");
        repo.write("crates/ash-sim/src/utils.rs", "");
        let paths: Vec<PathBuf> = check(repo.root())
            .unwrap()
            .into_iter()
            .map(|violation| violation.path)
            .collect();
        assert_eq!(paths, [PathBuf::from("crates/ash-sim/src/utils.rs")]);
    }
}
