---
paths:
  - "crates/ash-core/**"
  - "crates/ash-sim/**"
---

# Review checklist: simulation crates

Rules: `.claude/rules/sim.md`. Each "no" is a finding.

- Does every behavior trace to a design document? A spec cannot invent a
  mechanic.
- Is the state still a function of build, content, seed and commands only:
  no float, no std `HashMap`, no clock, no I/O?
- Does arithmetic go through newtypes with declared overflow semantics?
- Does a violated invariant crash through `invariant!`, with no
  `unwrap_or_default` hiding it?
- Does every `#[expect]` carry a reason that still holds?
- Is the player still an entity with a controller, never a global? Does the
  spec's "Multiplayer impact" section cover this change?
- Does each module name one glossary concept, with the `CONTEXT.md` term?
- Does the change avoid settling the data model or parallelism (ADRs 0001
  and 0002)?
- Does the acceptance test fail by assertion on the signatures commit?
- When the spec gives a perf budget, does its benchmark pass?
