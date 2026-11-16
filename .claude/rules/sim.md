---
paths:
  - "crates/ash-core/**"
  - "crates/ash-sim/**"
---

# Simulation crates: `ash-core` and `ash-sim`

Decided in `docs/plans/0000-methodology.md`, sections 1 and 4. Lints and
`xtask` enforce most of these rules. A need to work around one is a stop
trigger, never a local exception.

## Determinism

- Same build + same content + same seed + same commands = same state, bit for
  bit. No promise across versions.
- No I/O and no clock: both crates are `no_std` + `alloc`. Std is allowed in
  tests only.
- No floats: `clippy::float_arithmetic` is denied. Fixed-point goes through
  `ash-core` newtypes.
- No std `HashMap`: use `hashbrown` with the deterministic hasher that
  `ash-core` provides.

## Arithmetic and errors

- Overflow checks stay on in release, and `clippy::arithmetic_side_effects`
  is denied. Use newtypes with declared overflow semantics.
- No `as` conversions: `clippy::as_conversions` is denied.
- A violated invariant crashes, through the single `invariant!` macro. The
  server quarantines that world. Never silence an error with
  `unwrap_or_default`.
- Typed grid indices instead of an indexing ban.
- `unsafe` is forbidden. An exception needs an ADR.
- Silence a lint only with `#[expect(lint, reason = "...")]`.

## Structure

- `ash-core`: coordinates, game time, ids, deterministic RNG, fixed-point,
  integer helpers (sqrt, atan2, exp). `ash-sim`: world, scheduler, systems.
- Dependencies point down: `ash-sim` uses `ash-core`; `ash-core` uses no
  workspace crate. A new dependency for either crate requires an ADR.
- Game systems start as modules of `ash-sim`. A module becomes a crate only
  when a criterion recorded in an ADR is met.
- Modules by domain, not by layer: `src/scheduler/`, `src/world/`, never
  `src/systems/`. One module = one glossary concept of `CONTEXT.md`.

## World and time

- Pure turn-based, with a time-cost scheduler: an action costs N time units.
- The player is an entity with a controller, never a global. Shared worlds
  must stay possible.
- 3D tile grid split into chunks, sparse storage. Rendering is independent of
  the data.
- The data model (ADR 0001) and deterministic parallelism (ADR 0002) are not
  decided. Phase 1 uses simple, replaceable implementations (`Vec` storage,
  single-threaded scheduler) behind the crate boundaries. Do not settle either
  question in code.
