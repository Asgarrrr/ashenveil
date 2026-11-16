//! The `commit-msg` git hook: refuses a message that breaks the format.
//!
//! Format: a subject that starts with a capital letter, holds at most 72
//! characters and does not end with a period; a blank line before the body.
//! Messages that git writes itself (`Merge `, `Revert "`) and `fixup!` or
//! `squash!` commits pass as they are. The imperative mood is a review
//! criterion: no mechanical check tells "Add" from "Adds" reliably.

pub const FORMAT: &str = "format: a capitalized, imperative subject of at most 72 \
                          characters, no trailing period; a blank line before the body";

const MAX_SUBJECT_CHARS: usize = 72;

/// Subject prefixes of messages git generates: their length and wording are git's.
const GIT_GENERATED: &[&str] = &["Merge ", "Revert \"", "fixup! ", "squash! "];

/// `git commit --verbose` appends the diff below this line; git drops it.
const SCISSORS: &str = "# ------------------------ >8 ------------------------";

/// Checks the message as git will record it: comment lines, the diff below
/// the scissors line and leading blank lines are dropped first.
pub fn check(message: &str) -> Result<(), String> {
    let lines: Vec<&str> = message
        .lines()
        .take_while(|line| *line != SCISSORS)
        .filter(|line| !line.starts_with('#'))
        .skip_while(|line| line.trim().is_empty())
        .collect();
    let Some(subject) = lines.first() else {
        return Err("empty message".into());
    };
    if GIT_GENERATED
        .iter()
        .any(|prefix| subject.starts_with(prefix))
    {
        return Ok(());
    }
    if !subject.starts_with(|c: char| c.is_ascii_uppercase()) {
        return Err(format!(
            "subject `{subject}` must start with a capital letter"
        ));
    }
    let length = subject.chars().count();
    if length > MAX_SUBJECT_CHARS {
        return Err(format!(
            "subject is {length} characters long, the cap is {MAX_SUBJECT_CHARS}"
        ));
    }
    if subject.ends_with('.') {
        return Err(format!("subject `{subject}` must not end with a period"));
    }
    if lines.get(1).is_some_and(|line| !line.trim().is_empty()) {
        return Err("a blank line must separate the subject from the body".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn well_formed_messages_pass() {
        let subject_at_cap = format!("A{}", "b".repeat(MAX_SUBJECT_CHARS - 1));
        for message in [
            "Add the agent harness",
            "Add the agent harness\n",
            "Add the agent harness\n\nThe body explains why.\nOn several lines.\n",
            subject_at_cap.as_str(),
            "\n\nAdd the agent harness\n# Please enter the commit message.\n",
            // The diff below the scissors line would read as a body with no blank line.
            "Add the agent harness\n# ------------------------ >8 ------------------------\ndiff --git a/x b/x\n",
            "Merge pull request #2 from Asgarrrr/t2-rust-guardrails",
            "Revert \"Add the agent harness\"\n\nThis reverts commit 7c5602d.",
            "fixup! add the agent harness.",
            "squash! Add the agent harness",
        ] {
            assert_eq!(check(message), Ok(()), "{message:?}");
        }
    }

    #[test]
    fn malformed_messages_fail() {
        let subject_over_cap = format!("A{}", "b".repeat(MAX_SUBJECT_CHARS));
        for message in [
            "",
            "\n# Please enter the commit message.\n",
            "add the agent harness",
            "- Add the agent harness",
            subject_over_cap.as_str(),
            "Add the agent harness.",
            "Add the agent harness\nThe body without a blank line.",
            "merge the harness branch",
            "fixup!add the agent harness",
        ] {
            assert!(check(message).is_err(), "{message:?}");
        }
    }
}
