//! Specs and ADRs must keep the sections of their template, so that agents and
//! reviewers find the same information in the same place.

use std::io;
use std::path::Path;

use super::{Violation, markdown_files, relative};

/// Must match the `## ` headings of `docs/templates/adr.md`.
const ADR_SECTIONS: &[&str] = &[
    "Status",
    "Context",
    "Decision",
    "Alternatives considered",
    "Consequences",
];

/// Must match the `## ` headings of `docs/templates/spec.md`.
const SPEC_SECTIONS: &[&str] = &[
    "Status",
    "Context",
    "References",
    "Simplest alternative",
    "Multiplayer impact",
    "Slices",
    "Proposed amendments",
];

pub fn check(root: &Path) -> io::Result<Vec<Violation>> {
    let mut violations = Vec::new();
    for (dir, required) in [("docs/adr", ADR_SECTIONS), ("docs/specs", SPEC_SECTIONS)] {
        for path in markdown_files(&root.join(dir))? {
            let content = std::fs::read_to_string(&path)?;
            for section in missing_sections(&content, required) {
                violations.push(Violation {
                    path: relative(root, &path),
                    message: format!("missing section `## {section}`"),
                });
            }
        }
    }
    Ok(violations)
}

fn missing_sections<'a>(content: &str, required: &[&'a str]) -> Vec<&'a str> {
    let mut in_fence = false;
    let mut headings = Vec::new();
    for line in content.lines() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
        } else if !in_fence && let Some(heading) = line.strip_prefix("## ") {
            headings.push(heading.trim());
        }
    }
    required
        .iter()
        .copied()
        .filter(|section| !headings.contains(section))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::test_repo::TestRepo;
    use super::*;

    const VALID_ADR: &str =
        "# 0001\n## Status\n## Context\n## Decision\n## Alternatives considered\n## Consequences\n";

    fn messages(repo: &TestRepo) -> Vec<String> {
        check(repo.root())
            .unwrap()
            .into_iter()
            .map(|violation| violation.to_string())
            .collect()
    }

    #[test]
    fn complete_adr_passes() {
        let repo = TestRepo::new();
        repo.write("docs/adr/0001-example.md", VALID_ADR);
        assert_eq!(messages(&repo), Vec::<String>::new());
    }

    #[test]
    fn adr_without_decision_fails() {
        let repo = TestRepo::new();
        repo.write(
            "docs/adr/0001-example.md",
            &VALID_ADR.replace("## Decision\n", ""),
        );
        assert_eq!(
            messages(&repo),
            ["docs/adr/0001-example.md: missing section `## Decision`"]
        );
    }

    #[test]
    fn heading_inside_code_fence_does_not_count() {
        let repo = TestRepo::new();
        let fenced = VALID_ADR.replace("## Decision\n", "```\n## Decision\n```\n");
        repo.write("docs/adr/0001-example.md", &fenced);
        assert_eq!(
            messages(&repo),
            ["docs/adr/0001-example.md: missing section `## Decision`"]
        );
    }

    #[test]
    fn spec_without_multiplayer_impact_fails() {
        let repo = TestRepo::new();
        let spec: String = SPEC_SECTIONS
            .iter()
            .filter(|section| **section != "Multiplayer impact")
            .map(|section| format!("## {section}\n"))
            .collect();
        repo.write("docs/specs/0001-example.md", &spec);
        assert_eq!(
            messages(&repo),
            ["docs/specs/0001-example.md: missing section `## Multiplayer impact`"]
        );
    }

    #[test]
    fn templates_satisfy_their_own_check() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        for (template, required) in [("adr.md", ADR_SECTIONS), ("spec.md", SPEC_SECTIONS)] {
            let content =
                std::fs::read_to_string(root.join("docs/templates").join(template)).unwrap();
            assert_eq!(
                missing_sections(&content, required),
                Vec::<&str>::new(),
                "{template}"
            );
        }
    }
}
