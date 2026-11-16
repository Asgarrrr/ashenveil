# Ashenveil

A turn-based roguelike inspired by Dwarf Fortress. An authoritative Rust server
simulates the whole world at several levels of detail. A web client displays the
player's area. Agents write almost all the code; one human designs and approves.

The full methodology and its rationale: `docs/plans/0000-methodology.md`.

## Sources of truth

| Path | Content | Owner |
|---|---|---|
| `CONTEXT.md` | Domain glossary. One term = one concept. | Human approves |
| `docs/vision/pillars.md` | 3 to 5 pillars. Decide between two valid choices. | Human |
| `docs/design/<system>.md` | Game design: the WHAT and WHY, player side. | Human |
| `docs/adr/NNNN-<title>.md` | Technical decisions. Immutable: superseded, never edited. | Human |
| `docs/specs/NNNN-<name>.md` | Technical spec and slice list. Living until delivered. | Agent proposes |
| `docs/plans/` | Design documents from brainstorms. | — |
| `docs/templates/` | Spec, ADR, design, slice report, stop report. | Human |

- Authority order: pillars > ADR > design > spec > code. A contradiction
  between sources is a stop trigger. Never resolve it alone.
- A spec cannot invent a mechanic. A behavior with no design source is a stop trigger.
- Use the glossary terms verbatim in code, docs and commits.

## Commands

- `cargo xtask verify` — the definition of done. Today it checks instruction-file
  line caps and the mandatory sections of every spec and ADR.
- `cargo test -p xtask` — tests of the project commands.

Every project command is an `xtask` subcommand, written in Rust. No shell scripts.

## Decision rights

- Constitution (human only): pillars, design, ADRs, lint config, dependency
  matrix, budgets, CI, hooks, `CLAUDE.md`, `.claude/`. Never edit these without
  the human's explicit approval.
- Contract (agent proposes, human approves): specs and slicing, new
  dependencies, amendments that change an acceptance criterion.
- Execution (agents): the whole scope of an approved slice.

## Work types

| Type | Flow |
|---|---|
| Feature | design → spec → slices → implementation → review → milestone |
| Bug | repro → failing test → fix → review |
| Decision | `researcher` / `advisor` → throwaway prototype if needed → ADR |
| Chore | direct change → `cargo xtask verify` |

## Slices

- One branch, one worktree, one `implementer` in a fresh context. The brief
  holds the spec path, the slice id and the linked ADRs. Nothing else.
- Rust code slice: the first commit lays down the signatures; the acceptance
  test compiles and fails by assertion on that commit.
- Tooling slice: each guardrail has a negative test proving it fires.
- A slice that overflows during implementation stops and triggers a re-slicing.
- Review by an independent `reviewer` against the spec. Squash-merge.
- The slice report follows `docs/templates/slice-report.md`, including
  "What the spec did not foresee" and the cost.

## Discoveries during a slice

| Scope | Action |
|---|---|
| Inside the slice, contract unchanged | Fix, mention in the report. |
| The spec is wrong for this slice | Add it under `Proposed amendments` in the spec. |
| Outside the slice | Do not fix. Open a GitHub issue. |
| The constitution seems wrong | Argued proposal; superseding ADR if accepted. |

## Stop triggers

Contradiction between sources; slice overflow; a need that touches the
constitution; two failed attempts; an unreachable perf budget. Report with
`docs/templates/stop-report.md`: finding, options, recommendation, what is blocked.

## Repository layout

- Monorepo: Cargo workspace (`crates/`, `xtask/`) and one web package (`web/`).
- Crates are named `ash-<name>`, imported as `ash_<name>`. Dependencies point
  down toward `ash-core`. Only `ash-server` assembles.
- Modules by domain, not by layer. One module = one glossary concept.
- `foo.rs` + `foo/`, never `foo/mod.rs`.
- No catch-all files: no `utils.rs`, `helpers.rs`, `common.rs`, `misc.rs`.
- Unit tests inline; integration tests in `crates/<crate>/tests/`; test data in
  `tests/fixtures/`.
- Generated files are not committed.

## Instruction file caps

Checked by `cargo xtask verify`: this file ≤ 150 lines, each `.claude/rules/`
file ≤ 100, each crate `CLAUDE.md` ≤ 80. A fact worth not rediscovering goes
into module docs or a test, not an instruction file.
