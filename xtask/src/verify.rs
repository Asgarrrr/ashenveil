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

/// Markdown files directly inside `dir`, sorted for stable output.
/// A missing directory yields no files: it is not created until needed.
fn markdown_files(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut files = Vec::new();
    for entry in entries {
        let path = entry?.path();
        if path.is_file() && path.extension().is_some_and(|ext| ext == "md") {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

fn relative(root: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(root).unwrap_or(path).to_path_buf()
}

#[cfg(test)]
pub(crate) mod test_repo {
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU32, Ordering};

    /// Throwaway repository tree, deleted on drop.
    pub struct TestRepo {
        root: PathBuf,
    }

    impl TestRepo {
        pub fn new() -> Self {
            static NEXT: AtomicU32 = AtomicU32::new(0);
            let root = std::env::temp_dir().join(format!(
                "xtask-test-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
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
