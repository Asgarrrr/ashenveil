//! Repository checks that need no compiler: instruction-file line caps and
//! mandatory document sections.

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

mod line_caps;
mod sections;

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
    Ok(violations)
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
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut paths = entries
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<io::Result<Vec<_>>>()?;
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
    use super::test_repo::{TestRepo, lines};
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
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        assert_eq!(verify(root).unwrap(), Vec::new());
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
