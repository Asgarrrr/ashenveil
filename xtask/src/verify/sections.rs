//! Specs and ADRs must keep the sections of their template, so that agents and
//! reviewers find the same information in the same place.

use std::io;
use std::path::Path;

use super::{Violation, markdown_files, read_file, relative};

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
    "Glossary proposals",
    "Proposed amendments",
];

pub fn check(root: &Path) -> io::Result<Vec<Violation>> {
    let mut violations = Vec::new();
    for (dir, required) in [("docs/adr", ADR_SECTIONS), ("docs/specs", SPEC_SECTIONS)] {
        for path in markdown_files(&root.join(dir))? {
            let content = read_file(root, &path)?;
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
    let headings = headings(content);
    required
        .iter()
        .copied()
        .filter(|section| !headings.contains(section))
        .collect()
}

/// Level-2 ATX headings as rendered, outside code fences and HTML comments.
fn headings(content: &str) -> Vec<&str> {
    let mut fence: Option<(char, usize)> = None;
    let mut in_comment = false;
    let mut headings = Vec::new();
    for line in content.lines() {
        if in_comment {
            in_comment = !line.contains("-->");
            continue;
        }
        let Some(line) = strip_indent(line) else {
            continue;
        };
        if let Some((marker, len)) = fence {
            if fence_marker(line).is_some_and(|(m, l)| {
                m == marker && l >= len && line.trim_start_matches(m).trim().is_empty()
            }) {
                fence = None;
            }
        } else if let Some(marker) = fence_marker(line) {
            fence = Some(marker);
        } else if line.starts_with("<!--") {
            // `<!-->` and `<!--->` close themselves: the end marker may overlap the start.
            in_comment = !line.contains("-->");
        } else if let Some(heading) = level_two_heading(line) {
            headings.push(heading);
        }
    }
    headings
}

/// Up to three leading spaces are allowed; four make an indented code block.
fn strip_indent(line: &str) -> Option<&str> {
    let stripped = line.trim_start_matches(' ');
    (line.len() - stripped.len() <= 3).then_some(stripped)
}

fn fence_marker(line: &str) -> Option<(char, usize)> {
    let marker = line.chars().next().filter(|c| *c == '`' || *c == '~')?;
    let len = line.chars().take_while(|c| *c == marker).count();
    // A backtick in a backtick fence's info string makes the line inline code, not a fence.
    let info_ok = marker != '`' || !line[len..].contains('`');
    (len >= 3 && info_ok).then_some((marker, len))
}

fn level_two_heading(line: &str) -> Option<&str> {
    let rest = line.strip_prefix("##")?;
    if !(rest.is_empty() || rest.starts_with([' ', '\t'])) {
        return None;
    }
    let text = rest.trim();
    let without_closing = text.trim_end_matches('#');
    if without_closing.is_empty() || without_closing.ends_with([' ', '\t']) {
        return Some(without_closing.trim_end());
    }
    Some(text)
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

    fn assert_decision_missing(content: &str) {
        let repo = TestRepo::new();
        repo.write("docs/adr/0001-example.md", content);
        assert_eq!(
            messages(&repo),
            ["docs/adr/0001-example.md: missing section `## Decision`"]
        );
    }

    #[test]
    fn heading_inside_tilde_fence_does_not_count() {
        assert_decision_missing(&VALID_ADR.replace("## Decision\n", "~~~\n## Decision\n~~~\n"));
    }

    #[test]
    fn shorter_fence_inside_longer_fence_does_not_close_it() {
        assert_decision_missing(
            &VALID_ADR.replace("## Decision\n", "````\n```\n## Decision\n````\n"),
        );
    }

    #[test]
    fn heading_inside_html_comment_does_not_count() {
        assert_decision_missing(&VALID_ADR.replace("## Decision\n", "<!--\n## Decision\n-->\n"));
    }

    #[test]
    fn backticks_in_info_string_do_not_open_a_fence() {
        assert_decision_missing(&VALID_ADR.replace(
            "## Decision\n",
            "``` x ```\n## Note\n```\n## Decision\n```\n",
        ));
    }

    #[test]
    fn comment_closed_on_its_opening_line_by_short_form() {
        for opener in ["<!-->", "<!--->"] {
            assert_decision_missing(&VALID_ADR.replace(
                "## Decision\n",
                &format!("{opener}\n```\n-->\n## Decision\n```\n"),
            ));
        }
    }

    #[test]
    fn indented_comment_closer_ends_the_comment() {
        let repo = TestRepo::new();
        repo.write(
            "docs/adr/0001-example.md",
            &format!("<!-- note\n     end -->\n{VALID_ADR}"),
        );
        assert_eq!(messages(&repo), Vec::<String>::new());
    }

    #[test]
    fn indented_and_closed_headings_count() {
        let repo = TestRepo::new();
        let content = VALID_ADR
            .replace("## Decision\n", "   ## Decision ##\n")
            .replace("## Context\n", "## Context\t\n");
        repo.write("docs/adr/0001-example.md", &content);
        assert_eq!(messages(&repo), Vec::<String>::new());
    }

    #[test]
    fn four_space_indent_is_not_a_heading() {
        assert_decision_missing(&VALID_ADR.replace("## Decision\n", "    ## Decision\n"));
    }

    #[test]
    fn nested_and_uppercase_extension_files_are_checked() {
        let repo = TestRepo::new();
        repo.write("docs/adr/archive/0001-old.md", "# 0001\n");
        repo.write("docs/specs/0002-Foo.MD", "# 0002\n");
        let paths: Vec<String> = check(repo.root())
            .unwrap()
            .into_iter()
            .map(|violation| violation.path.display().to_string())
            .collect();
        assert!(paths.contains(&"docs/adr/archive/0001-old.md".to_owned()));
        assert!(paths.contains(&"docs/specs/0002-Foo.MD".to_owned()));
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
    fn section_lists_match_template_headings_exactly() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        for (template, required) in [("adr.md", ADR_SECTIONS), ("spec.md", SPEC_SECTIONS)] {
            let content =
                std::fs::read_to_string(root.join("docs/templates").join(template)).unwrap();
            assert_eq!(headings(&content), required, "{template}");
        }
    }
}
