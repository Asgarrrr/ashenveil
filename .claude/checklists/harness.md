---
paths:
  - "xtask/**"
  - ".cargo/**"
  - ".claude/**"
  - ".github/**"
---

# Review checklist: tooling and agent harness

Each "no" is a finding.

- Does each new guardrail have a negative test, seen failing with the
  guardrail disabled?
- Is every command an `xtask` subcommand, with no shell script? Do git and
  Claude Code hooks call `cargo xtask hook <name>`?
- Does a change to a constitution path carry the human's explicit approval?
- Does a new constitution path appear in `.claude/protected-paths.txt`? The
  Claude Code hook and the CI check both read that list.
- Does `CLAUDE.md` still describe what `verify` runs?

## Manual check: constitution hook in a Claude Code session

Run it after any change to `.claude/settings.json` or to the
`hook constitution` code. Automated tests cover the hook logic; this proves
that Claude Code calls it.

1. Start `claude` at the root of the main checkout. Run `/hooks`: `PreToolUse`
   lists `Edit|Write|NotebookEdit` with `cargo xtask hook constitution`.
2. Ask: "Use the Write tool to create `docs/adr/0001-hook-test.md`
   containing `test`."
   Expected: the call is refused with a `PreToolUse:Write hook error` that
   names `docs/adr/0001-hook-test.md` as a constitution path.
   `git status` shows no new file.
3. Ask: "Use the Write tool to create `crates/ash-sim/src/hook_test.rs`
   containing `// test`." Expected: the file is created. Delete it.
4. Run `git worktree add .claude/worktrees/hook-test`. Repeat steps 2 and 3
   with paths under `.claude/worktrees/hook-test/`. Expected: same results.
   The worktree file is not refused for being under `.claude/`.
5. Run `git worktree remove --force .claude/worktrees/hook-test`, then
   `git branch -D hook-test`.
