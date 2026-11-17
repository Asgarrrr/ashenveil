# 0000 — Ashenveil working methodology

Status: approved by the human on 2026-11-16.
Date: 2026-11-16.
Scope: how agents build Ashenveil. No game mechanics here.

## 1. Context

Ashenveil is a roguelike inspired by Dwarf Fortress. The whole world is simulated
at several levels of detail. The game is pure turn-based. An authoritative Rust
server runs the simulation. A web client displays the player's area.

Agents write almost all the code. One human designs the game and approves.
This is the third attempt at this game. The previous ones stopped for two
opposite reasons: code that became messy, and foundation work that delayed any
playable result. This methodology addresses both risks.

### Product decisions already made

| Topic | Decision |
|---|---|
| World | Full multi-resolution simulation: level 0 (full fidelity around each player) to level 3 (history). |
| Time | Pure turn-based. Time-cost scheduler (an action costs N time units). |
| Server | Authoritative. Persistent per-player worlds, unloaded offline, lazy catch-up on return. |
| Multiplayer | Not now. Shared worlds kept possible: the player is an entity with a controller, never a global. |
| Determinism | Same build + same content + same seed + same commands = same state, bit for bit. No promise across versions. |
| Geometry | 3D tile grid split into chunks, sparse storage. Rendering is independent of the data. |
| Transport | WebSocket, binary messages. Message format independent of the transport. |
| Hosting | Private GitHub repository with GitHub Pro. GitHub Actions. Production on Linux (x86 or ARM). |
| Repository | Monorepo: Cargo workspace (`crates/`, `xtask/`) and a single web package (`web/`). A protocol change ships server and client in one PR. No JS monorepo tool. `xtask` orchestrates both toolchains. CI path filters skip web jobs when only internal Rust changes. |
| Language | English for every repository artifact: code, comments, docs, commits. |

## 2. Guiding principle

> Every rule a machine can check is checked by a machine.
> Prose is reserved for the *why* and for what cannot be checked.

Corollary 1: a blocking guardrail fires when fixing is cheap. Slicing blocks at
plan time. `verify --fast` (fmt + clippy on touched crates) blocks at commit, via
a `pre-commit` hook. The commit-message format blocks at commit.
A check that comes later only serves as a signal.

Corollary 2: each guardrail arrives with the first slice that needs it. The bar
never drops. It is reached at the right time (section 10).

## 3. Sources of truth

```
CLAUDE.md                  Entry map. Line cap checked by xtask.
CONTEXT.md                 Domain glossary. One term = one concept.
docs/
  vision/pillars.md        3 to 5 pillars. Used to decide between two valid choices.
  design/<system>.md       Game design. The WHAT and WHY from the player's side. Owned by the human.
  adr/NNNN-<title>.md      Technical decisions. Immutable: superseded, never edited.
  specs/NNNN-<name>.md     Technical spec and slice list. Living until delivered.
  perf/budgets.md          Perf budgets (created when the first system is measurable).
  plans/                   Design documents from brainstorms.
.claude/rules/<area>.md    Path-scoped rules, loaded by the files touched.
crates/*/CLAUDE.md         Crate-local invariants.
content/                   Game data (creatures, materials...) — see planned ADR 0005.
```

Rules:

1. **Authority order**: pillars > ADR > design > spec > code. An agent that
   detects a contradiction stops and reports it. It never resolves it alone.
2. **Design ≠ spec.** An agent cannot invent a mechanic by writing a spec.
3. **Progressive disclosure.** An agent's brief holds the spec, the slice id and
   the linked ADRs. Nothing else. Path-scoped rules load on their own.
4. **Mechanical caps.** `xtask` checks line counts: root `CLAUDE.md` ≤ 150,
   each `.claude/rules/` file ≤ 100, every other `CLAUDE.md` or `CLAUDE.local.md`
   ≤ 80 (amended 2026-11-16, approved by the human). A fact "worth
   not rediscovering" goes into module docs or a test, not an instruction file.

## 4. Enforced architecture

### Repository layout

```
ashenveil/
├── CLAUDE.md  CONTEXT.md  README.md
├── Cargo.toml  Cargo.lock  rust-toolchain.toml  deny.toml  .gitignore
├── crates/
│   └── ash-<name>/            CLAUDE.md, Cargo.toml, clippy.toml, src/, tests/, tests/fixtures/
├── web/                       Single TypeScript package (phase 1)
├── content/                   Game data (ADR 0005)
├── xtask/                     Every project command, in Rust
├── docs/                      vision/ design/ adr/ specs/ plans/ perf/ templates/
├── .claude/                   settings.json, rules/, checklists/
└── .github/                   workflows/ (pr.yml, nightly.yml), pull_request_template.md
```

Layout rules:

1. **Minimal root.** Only files that tools require at the root, plus the two
   entry points (`CLAUDE.md`, `CONTEXT.md`) and `README.md`.
2. **No shell scripts.** Every command (verify, hooks, setup, codegen) is an
   `xtask` subcommand: written in Rust, tested, cross-platform. Git and Claude
   hooks call `cargo xtask hook <name>`.
3. **Generated files are not committed.** `xtask` builds them (for example the
   WASM package of `ash-protocol`) into git-ignored directories.
4. **Crate naming.** Directory and package `ash-<name>`, imported as `ash_<name>`.
   The prefix avoids the clash with Rust's built-in `core` crate.

### Code layout conventions (checked by lints or `xtask`)

- **Modules by domain, not by layer.** `ash-sim/src/scheduler/`, `src/world/`,
  `src/combat/` — never `src/components/` + `src/systems/`. One module = one
  glossary concept, so a slice touches one module.
- **One module file style.** `foo.rs` + `foo/`; `foo/mod.rs` is denied by
  `clippy::mod_module_files`.
- **Tests.** Unit tests inline (`#[cfg(test)] mod tests`). Integration tests in
  `crates/<crate>/tests/`. Test data in `tests/fixtures/`.
- **No catch-all files.** `utils.rs`, `helpers.rs`, `common.rs`, `misc.rs` are
  refused by an `xtask` file-name check. A helper belongs to the concept it serves.

### Crates

| Crate | Role | Created |
|---|---|---|
| `ash-core` | Coordinates, game time, ids, deterministic RNG, fixed-point, integer helpers (sqrt, atan2, exp). | Phase 0 |
| `ash-sim` | World, scheduler, systems. Pure. | Phase 0 |
| `ash-protocol` | Messages and codec. Depends on `ash-core` only. Compiled to WASM for the client. | Phase 1 |
| `ash-server` | Network, sessions, world lifecycle. The only crate that assembles. | Phase 1 |
| `xtask` | Single entry point for commands. | Phase 0 |
| `ash-worldgen`, `ash-persistence`, `ash-tools` | Created by the first slice that needs them. | Later |
| `web/` | TypeScript client. | Phase 1 |

Game systems start as modules of `ash-sim`. A module becomes a crate only when a
criterion recorded in an ADR is met.

Dependency direction: always downward, toward `ash-core`. Only `ash-server` assembles.

### Guardrails

| Risk | Guardrail |
|---|---|
| Forbidden dependency | `xtask` test: `cargo metadata` compared to an allowed matrix. |
| I/O or clock in the simulation | `#![no_std]` + `alloc` for `ash-core` and `ash-sim`. Std allowed in tests only. |
| Non-deterministic `HashMap` | `hashbrown` with a deterministic hasher provided by `ash-core`. |
| Floats | `clippy::float_arithmetic` denied in `ash-core` and `ash-sim`. Fixed-point (`fixed`) through `ash-core` newtypes. A simplicity choice, not only a determinism one. |
| Integer overflow | `overflow-checks = true` in release for `ash-core` and `ash-sim`. Newtypes with declared overflow semantics, allowlisted in `arithmetic-side-effects-allowed`. |
| Silent conversion | `clippy::as_conversions`. |
| Masked error | A violated invariant crashes, through a single `invariant!` macro. The server catches the panic and quarantines **that world**. No `unwrap_or_default` to silence an error. Typed grid indices instead of an indexing ban. |
| `unsafe` | `unsafe_code = "forbid"`. Exceptions by ADR only. |
| Silenced lint | `#[expect(lint, reason = "...")]` only (`allow_attributes`, `allow_attributes_without_reason`). |
| Inconsistent lints | `[workspace.lints]` = common base. A crate cannot extend it while inheriting: `ash-core` / `ash-sim` bans are crate-level attributes. |
| Dubious dependency | `cargo-deny` (licenses, advisories, duplicates, sources). Allowlist for `ash-core` and `ash-sim`: a new dependency requires an ADR. |
| Toolchain drift | Pinned `rust-toolchain.toml`, committed `Cargo.lock`. Separate nightly toolchain for fuzzing only. |

### Decisions deferred to ADRs (prepared with `researcher` + `advisor`, measured prototype if needed)

These ADRs do **not** precede the phase 1 walking skeleton. Phase 1 uses
deliberately simple, replaceable implementations (`Vec` storage, single-threaded
scheduler, minimal codec) behind the crate boundaries. Phase 1 measurements then
feed ADRs 0001 to 0003.

- **0001** Simulation data model (existing ECS, custom ECS, per-component arrays).
- **0002** Deterministic parallelism (partitioning). Before the production scheduler: catch-up will need several cores.
- **0003** Wire format and codec. Lead: `ash-protocol` in WASM on the client, one codec, version handshake.
- **0004** Save format: versioned snapshots, migrations, a corpus of old saves in CI. The command log is **not** a save format: it is cut at each version change.
- **0005** Content as data (`content/`). The content hash is a determinism input.

## 5. Work types

| Type | Flow | Output |
|---|---|---|
| Feature | design → spec → slices → implementation → review → milestone | Code + archived spec |
| Bug | repro → failing test → fix → review | Code + regression test |
| Decision | `researcher` / `advisor` → throwaway prototype if needed → ADR | ADR, no merged code |
| Chore | direct change → `verify` | Code |

## 6. Feature flow

1. **Design** (`docs/design/`): the human, through a brainstorm.
2. **Spec** (`docs/specs/`): `opus` agent. References the design and the ADRs.
   Each slice lists its files, invariants, perf budget and acceptance command.
   It answers "the simplest alternative", proposed by the `advisor`. Challenge:
   `advisor`, `researcher`, `slice-check`.
   **The human approves the spec and its slicing, once.**
3. **Slice**: one branch, one worktree, one `implementer` in a fresh context.
   a. Rust code slice: the first commit lays down the signatures. The acceptance
      test compiles and **fails by assertion** on that commit.
      Tooling slice (hooks, CI, `xtask`): each guardrail has a negative test
      proving it fires.
   b. Implement until green. `cargo xtask verify`.
4. **Review**: independent `reviewer` that did not see the implementation. Reads
   the diff against the spec, with checklists loaded by path.
5. **Merge**: required CI and the `reviewer` close the slice. Squash-merge.
6. **Milestone**: the human validates **by playing**. The spec is closed;
   `CONTEXT.md` and `design/` are up to date.

Sequential by default. Parallel only for slices declared file-disjoint.

### Slice size

Criterion: one slice = one coherent behavior change, verifiable, reviewable in
one pass. Size is controlled at slicing time. CI flags a large diff without
blocking; the `reviewer` judges coherence. A slice that overflows during
implementation triggers a stop and a re-slicing of the spec.

## 7. Performance and elegance

- Budgets on reference hardware (`docs/perf/budgets.md`). Each spec inherits a
  share, which becomes a benchmark.
- Canonical reference worlds (fixed seeds) to measure realistic cases.
- Proportional rigor:

| Choice | Requirement |
|---|---|
| Hot path or hard to reverse | ≥ 2 candidates prototyped and measured, `advisor` challenge, ADR with numbers, hardware and command. |
| Hot path, reversible | The budget benchmark passes. |
| Cold code | Simplest and most readable. Measure only if profiling points at it. |

- Blocking measurements: instruction counts (`gungraun`, Linux x86 in CI) and
  allocation counts (custom counting allocator, ~30 lines). Wall time: tracked
  nightly, never blocking.
- Elegance: explicit review criterion (one concept per type, glossary vocabulary,
  no speculative abstraction, a deletion beats an addition). An optimization that
  hurts readability requires a benchmark and stays isolated.

## 8. Decision rights

| Zone | Who | Content |
|---|---|---|
| Constitution | Human only | Pillars, design, ADRs, lint config, dependency matrix, budgets, CI, hooks, `CLAUDE.md`, `.claude/` |
| Contract | Agent proposes, human approves | Specs and slicing, new dependencies, amendments that change an acceptance criterion |
| Execution | Agents | The whole scope of an approved slice |

Glossary entries are proposed in batches and approved at the milestone.

Constitution protection:
- **Authority**: a required CI check (branch protection) refuses any PR that
  touches a protected path without the human's explicit approval.
- **Convenience**: a `PreToolUse` hook on `Edit|Write` blocks those paths during
  an agent session. It can be bypassed through Bash: it is only a reminder.

Stop triggers: contradiction between sources; slice overflow; a need that touches
the constitution; two failed attempts; an unreachable perf budget.
Stop report: finding, options, recommendation, what is blocked.

### Implementer discoveries

| Scope | Action |
|---|---|
| Inside the slice, contract unchanged | Fix, mention in the report. |
| The spec is wrong for this slice | Write it under `Proposed amendments` in the spec. Blocks only if an acceptance criterion changes. |
| Outside the slice | Do not fix. Open a GitHub issue (inbox). |
| The constitution seems wrong | Argued proposal. Superseding ADR if accepted. |

The slice report must contain: "What the spec did not foresee" ("nothing" if
empty) and the cost (tokens, duration). The `reviewer` checks for undeclared
deviations. A recurring discovery feeds the spec template or a checklist.

## 9. Definition of done

1. **Mechanical**: `cargo xtask verify` exits 0 locally; `cargo xtask verify --ci`
   passes in CI. The content of `--ci` grows with the section 10 triggers
   (Linux x86 instruction gates, multi-architecture replay).
2. **Proof**: `cargo xtask accept <test>` shows failure by assertion on the
   signatures commit and success on the branch.
3. **Judgment**: `reviewer` with no blocking finding, complete report, docs in sync.

Determinism: two runs of the same build compared with each other, on Linux x86
and Linux ARM. This test detects non-determinism, not regressions: a deterministic
regression is the job of behavior tests. No stored reference hash, except for a
slice declared "behavior-preserving".

Red nightly: no new slice starts until it is fixed. Mutation testing is a nightly
**report** and an `--in-diff` run on PRs, never a nightly blocker (equivalent
mutants would freeze the project).

## 10. Progressive guardrail rollout

| Guardrail | Activation trigger |
|---|---|
| Docs, caps, lints, `no_std`, dependency matrix, `cargo-deny`, hooks, base CI | Phase 0 |
| Multi-architecture deterministic replay | First slice with a simulation that advances |
| Allocation counting | When a spec defines a tick's "steady state" |
| `gungraun` and budgets | First system with a budget in `budgets.md` |
| Protocol fuzzing | First message decoder |
| Save corpus | First save format (ADR 0004) |
| Mutation testing | When `ash-sim` holds a complete system |
| `xtask status` | When at least one spec is open and issues exist |
| Visual world inspector | Before the first level-1 system |

## 11. Project memory

Derived state, never hand-maintained.

| Information | Location |
|---|---|
| What is true | `docs/` |
| Progress | Checked slice boxes in specs |
| History | Squashed commits (body = why) and ADRs |
| Inbox | GitHub issues, via `gh` |
| Interruption mid-slice | `/handoff` document, not versioned, deleted on resume |

`cargo xtask status` (once triggered) computes open specs, next slice, nightly
state, issues and pending amendments.

## 12. Out of scope

Game mechanics, rendering, visual style, rendering engine choice. Each gets its
own brainstorm.

Game architecture questions raised during review, to be handled in their own specs:
- consistency contract for transitions between detail levels;
- offline catch-up: maximum horizon, latency, memory and save-size budgets;
- time advancement with two players in a shared world;
- saves: interrupted writes, corrupt files, resuming after an update.

The spec template has a mandatory "Multiplayer impact" section. It forces every
spec that touches the scheduler, the world or the player to confront the
shared-world assumption before it becomes expensive.

## 13. Phase 0 slices

Phase 0 = set up this methodology in the repository. No game code.
Phase 1 (walking skeleton: an `@` that moves over WebSocket, green replay) will
be a separate spec, with no prior ADR (see section 4).

T1 is done directly. T2 to T4 go through the full flow (`implementer`,
`reviewer`, CI): phase 0 thus proves that the workflow works.

### T1 — Repository and sources of truth
- Files: `git init`, `.gitignore`, `CLAUDE.md`, `CONTEXT.md`, `docs/` (vision,
  design, adr, specs, plans), templates (spec, ADR, design, slice report, stop
  report), workspace `Cargo.toml`, minimal `xtask`.
- Behavior: `cargo xtask verify` checks the line caps of `CLAUDE.md` and rule
  files, and the mandatory sections of each spec and ADR.
- Acceptance: `cargo xtask verify` passes; a `CLAUDE.md` over its cap and an ADR
  without a `Decision` section make it fail (`xtask` tests).

### T2 — Rust guardrails
- Files: `rust-toolchain.toml`, `[workspace.lints]`, per-crate `clippy.toml`,
  `deny.toml`, empty `no_std` crates `ash-core` and `ash-sim`, dependency matrix.
- Behavior: `verify` runs fmt, clippy, tests, `cargo-deny` and the matrix test.
- Acceptance: each guardrail has a negative test proving it fires (float in
  `ash-sim`, `as`, `#[allow]` without reason, forbidden edge, std usage,
  `foo/mod.rs`, a `utils.rs` file).

### T3 — Agent harness
- Files: `.claude/settings.json` (constitution hook), `.claude/rules/sim.md`,
  `.claude/rules/protocol.md`, `commit-msg` and `pre-commit` hooks installed by
  `cargo xtask setup`, a single list of protected paths, path-scoped review checklists.
- Behavior: editing a protected path is refused in session; an invalid commit
  message is refused; a red `verify --fast` refuses the commit.
- Acceptance: hook script tests on every path of the protected list (exit 2) and
  on a free path (exit 0); after `cargo xtask setup` in a temporary repository, a
  real commit with an invalid message is refused; a documented manual repro in a
  Claude session (editing `docs/adr/` refused).

### T4 — GitHub and CI
- Human prerequisites: GitHub Pro account, `gh auth login`.
- Files: Actions workflows (PR: `verify` on Linux x86; protected-path check;
  nightly: Linux ARM skeleton), PR template, labels.
- Behavior: branch protection on `main`: required checks, no direct push,
  squash-merge only.
- Acceptance: a PR with a red `verify` cannot be merged; a PR touching
  `docs/adr/` stays blocked without the human's approval (two test PRs); a direct
  push to `main` is refused; `gh api` confirms squash-only and the required
  checks; the protected-path check reads the same list as the T3 hook.

## Proposed amendments

- 2026-11-16, accepted: the 80-line cap applies to every `CLAUDE.md` and
  `CLAUDE.local.md` other than the root `CLAUDE.md`, not only crate ones
  (section 3, rule 4). Found by the T1 code review: `web/CLAUDE.md` or a
  nested `src/<module>/CLAUDE.md` would otherwise grow unchecked.
- 2026-11-16, accepted: mandatory sections. ADR: Status, Context, Decision,
  Alternatives considered, Consequences. Spec: Status, Context, References,
  Simplest alternative, Multiplayer impact, Slices, Proposed amendments.
- 2026-11-16, accepted: `rust-toolchain.toml` (Rust 1.100.0) lands in T1
  instead of T2, on the human's request.
- 2026-11-16, accepted 2026-11-17 (found in T2): `#![no_std]` does not stop
  `extern crate std;`. With it, `ash-sim` reads the clock and clippy stays
  green. Proposal: an `xtask` source check that refuses `extern crate std` in
  `ash-core` and `ash-sim`. Tests do not need it: the crates are `no_std` only
  outside `cfg(test)`. Resolved by `verify::sim_purity`, in the `files`
  step. It refuses `extern crate std` in every module, tests included: tests
  get std without it.
- 2026-11-16, accepted 2026-11-17 (found in T2): `[profile.release.package.<crate>]`
  `overflow-checks` does not cover generic code of `ash-core` or `ash-sim`
  instantiated in another crate. That code takes the settings of the crate that
  instantiates it (Cargo book, "Overrides and generics"). Proposal: set
  `overflow-checks = true` for the whole release profile, or accept the gap in
  the guardrail table. The whole release profile sets `overflow-checks`.
- 2026-11-16, accepted, resolved in T2: a crate without `[lints] workspace = true`
  escapes every workspace lint. Cargo only warns (`missing_lints_inheritance`).
  `verify` refuses a workspace member that does not inherit
  `[workspace.lints]`.
- 2026-11-16, accepted 2026-11-17 (found in T3): the plan does not define the commit
  message format. T3 enforces: a subject that starts with a capital letter,
  at most 72 characters, no trailing period; a blank line before the body.
  Git's own `Merge ` and `Revert "` messages and `fixup!` / `squash!` commits
  pass. The imperative mood stays a review criterion: no mechanical check
  tells it apart reliably. The human asked that messages stay human: the
  format is only a guardrail. The tone is a review criterion, like the
  imperative mood: a natural subject, a short body that explains the why in
  plain sentences, no line-by-line narration of the diff, no stock phrases.
  No checklist holds it yet: the checklists are scoped by path.
- 2026-11-16, accepted 2026-11-17 (found in T3): the `ash-core` and `ash-sim` bans are
  crate-level attributes in `lib.rs` (section 4). They are lint config, but
  `lib.rs` is not in the protected-path list: every new module edits it.
  Proposal: an `xtask` check that both crate roots keep their `no_std` and
  `deny` attributes. Resolved by `verify::sim_purity`. It also refuses any
  other crate-level attribute that names a banned lint. Its source file
  joins the protected-path list, since it holds the required attributes.
- 2026-11-16, accepted (T4 design, human decision): a single GitHub identity.
  Agents push and open PRs with the human's account, so GitHub cannot tell an
  agent from the human. The human's approval of a constitution change is the
  `constitution-approved` label, which only the human applies. An agent can
  technically apply it too: the guarantee is visible and auditable, not a lock.
  The T4 acceptance criterion "stays blocked without the human's approval"
  reads "stays blocked without the `constitution-approved` label".
- 2026-11-16, accepted (T4 design): the protected-path check is
  `cargo xtask constitution --base <sha> --head <sha> [--approved]`. It lists
  the files changed between the merge base of `base` and `head`, renames
  counted as a deletion plus an addition. It matches them against the list
  read from `base`, with the T3 hook matcher. It fails when a protected path
  is touched without `--approved`. It reads the list and runs code from the
  base only: the CI job uses `pull_request_target`, builds `xtask` from the
  base commit and never executes the PR's code. A PR thus cannot weaken the
  check, the workflow or the list that judges it. A new push to the PR
  removes the label, so an approval never covers later commits. The source
  file of the check joins the protected-path list.
- 2026-11-16, accepted (T4 design): squash-only and "no direct push" need a
  repository ruleset on `main`, with an empty bypass list. Classic branch
  protection cannot enforce squash-only. No review is required: the human
  merges their own PRs. Required checks: `verify` and `constitution`.
- 2026-11-16, accepted (T4 review): the protected-path check judges against
  the current tip of `main`, not the PR's recorded base, and the ruleset
  requires an up-to-date branch: a list entry added after the check ran
  still applies. The label approves the head only if it was applied after
  GitHub received that head, timed by the first `pull_request` run of the
  head SHA. Removing the label on a new push is not enough: GitHub may
  cancel a pending run. The `xtask` dispatch, manifest and build script join
  the protected-path list: otherwise one free PR could disarm the check for
  the next one.
- 2026-11-16, accepted (human decision, T4): the repository is public, not
  private with GitHub Pro (section 1, Hosting). Private Actions minutes were
  blocked by billing; public repositories get standard and ARM runners and
  rulesets at no cost. The constitution workflow stays safe for fork PRs: it
  never executes the PR's code.
