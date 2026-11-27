# NNNN — <Spec name>

<!-- Copy to docs/specs/NNNN-<name>.md. `cargo xtask verify` checks that every
     `## ` section below is present. The human approves the spec and its slicing
     once. The spec stays living until delivered. -->

## Status

Draft | Approved (YYYY-MM-DD) | Delivered (YYYY-MM-DD).

## Context

What this spec delivers and why now. A spec never invents a mechanic: every
behavior traces back to a design document.

## References

- Design: `docs/design/<system>.md`
- ADRs: `docs/adr/NNNN-<title>.md`
- Perf budget share: `docs/perf/budgets.md` (or "none: cold code").

## Simplest alternative

The simplest approach that could work, proposed by the `advisor`. Either adopt
it, or say why it is not enough.

## Multiplayer impact

How this spec behaves if two players share one world. Mandatory for any spec
that touches the scheduler, the world or the player. Write "none" with a reason
otherwise.

## Slices

Each slice is one coherent behavior change, verifiable and reviewable in one
pass. Check the box when the slice is merged.

### - [ ] S1 — <slice name>

- Files: <paths created or changed>.
- Invariants: <what must stay true>.
- Perf budget: <benchmark and threshold, or "none">.
- Acceptance: `cargo xtask accept <test>` — <what it proves>.

## Glossary proposals

Fill this table when the spec is drafted, from the nouns of the design. Each
definition restates a rule and cites it; the signatures commit of each slice
uses these names. The human approves the rows into `CONTEXT.md` at the
milestone. Write "none" with a reason when the spec adds no term.

| Term | Proposed definition | Source | Code name | Avoid |
|---|---|---|---|---|
| <term> | <the rule it restates> | `docs/design/<system>.md` R<n> | `<Name>` | <rejected synonyms> |

## Proposed amendments

(empty)
