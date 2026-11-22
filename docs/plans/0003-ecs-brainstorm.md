# 0003 — Simulation data model brainstorm

Status: direction approved by the human on 2026-11-22. A second-model read
(2026-11-22) raised seven points, all folded in (F1 to F6, slices 2 and 3).
Awaiting the human's final approval. Feeds ADR 0001 (simulation data model). Also produces inputs for ADR
0002 (partitions), ADR 0003 (wire format), ADR 0004 (save format) and the
future level-of-detail design.

## Inputs

- The human chose a custom data model: technical challenge, elegance,
  performance.
- Challenge passes, 2026-11-21 and 2026-11-22: a researcher and an advisor
  on Opus, then on Fable. A final Fable advisor reviewed the reframing
  against the lab sources.
- An independent lab, 2026-11-22: a Fable lead and 13 Opus agents. It did not
  read this document. It built 10 prototypes, all `no_std` + `alloc`,
  `forbid(unsafe_code)`, without dependencies, under rustc 1.100.0.
  - Prototypes and summary: `~/Documents/Projects/ashenveil-lab/` (outside
    the repository, not committed).
  - All numbers come from one machine (Apple M5 Pro). The main session
    re-ran the `integrated` tests (25 passed), not the benches.
  - No prototype has an awaited controller or a disconnect, so the lab
    proves mechanisms on a world without a player.

## Resolution

Ashenveil does not build a classic ECS. A classic ECS rests on dynamic
composition, batch iteration and a systems scheduler. This game needs none
of the three: its component set is static, access is random by entity, and
the time-cost scheduler drives one entity at a time.

The direction: a permanent **identity** of entities, separate from their
**embodiment** in a store. Dwarf Fortress hand-rolls the same split
(historical figures and units).

ADR 0001 freezes the small contract below. Every mechanism waits for the
game slice that needs it, and for a measurement on the real game.

## Contract (ADR 0001)

### F1 — Closed static schema

The set of component types is closed and known at compile time. Every
borrow is checked by the compiler: no `unsafe`, `RefCell`, `Any` or
`TypeId`. Mods, if any, are compiled Rust.

- F4 makes this simple: rules only read (`&Store`), and `apply` alone
  writes, so no rule ever needs two mutable stores at once. Rules read
  through domain views. The field arrangement is a mechanism, out of the
  ADR.

- Rejected: dynamic type-erased storage. It needs an `unsafe` ADR exception,
  or runtime borrow checks that crash a world instead of failing to compile.

### F2 — Opaque, never-reused entity ids

An `EntityId` is never reused. No code may assume that a new id is the
largest, or read meaning into its bits. An id is scoped to its world.
Allocation is immediate inside `apply`, so a birth takes effect at once
(design rule R7). Exhausting the id space is a violated invariant: the
world is quarantined. A test covers the boundary.

- Why: a chronicle can name a dead entity forever (pillar 4). EVE Online's
  id recycling broke logs that pointed at recycled ids.
- The width (`u64`) is a mechanism. The wire form belongs to ADR 0003: the
  client parses ids as a JavaScript `number`, exact up to 2^53.

### F3 — Total lookup

A lookup by id never returns a bare `None`. ADR 0001 freezes two cases:
**embodied here**, or **issued but not embodied here**. Looking up an id
the world never issued is a violated invariant. Finer cases ("elsewhere",
"dead" with a retained record) refine the second case later, each with the
data it needs: "elsewhere" with ADR 0002, "dead" with the identity rule.

- Why: a `Bond { heir }` pointing at an entity elsewhere must not read as
  "no heir" (pillar 1). A scheduler entry for a dead entity must not crash
  the world.
- Deferred: whether every id keeps an entry forever. Dwarf Fortress keeps
  only a share of beings as historical figures. A future design rule grants
  identity (named, chronicled, heir) and says which data survives death.

### F4 — Choose, resolve, apply

```rust
choose(&Perception, &[Offer]) -> Action                    // controller: player or simulation
resolve(&Store, &Perception, actor, Action)
    -> Result<Effects, Refused>                            // pure rule
apply(&mut Store, Effects)                                 // the only write path
```

- A controller sees only what its character perceives, plus the offers
  with their costs (R9, R14). The compiler stops a behavior from reading a
  hidden entity's speed (R18).
- `resolve` reads the actor's perception for refusals and the true store
  for outcomes. A move into a tile the character perceives as impassable
  is refused (R10). A move into an obstacle it does not perceive costs the
  move (R12).
- A refusal changes nothing (R10): no store write, no clock advance, and no
  RNG draw.
- Randomness enters `choose` and `resolve` as an explicit input. The
  scheme, sequential or counter-based, belongs to ADR 0002.
- Every write goes through `apply`. A journal or a world hash can then be
  added later without touching the rules.

### F5 — Physical layout is unobservable

No API exposes storage order or row positions. Order-sensitive work uses
the scheduler order or an explicit key, never table order. World equality
is logical.

- Proof: a differential test varies the storage representation while it
  keeps the logical identities and the scheduling sequence. It replays one
  command log with and without a save and restore at every entity turn,
  and with the stores rebuilt in a shuffled internal order. All runs
  produce the same observable states. Shuffling the creation or scheduling
  history is not a layout change: it may legitimately change ids and R8
  tie order.
- Why not a byte-canonical layout: the lab's non-canonical layout passes
  the restore test once passes iterate by id. Unobservable layout keeps
  every storage mechanism replaceable.

### F6 — Closed world state

Every field of the world is classified as saved, derivable, or external.
Today's fields (`crates/ash-sim/src/world.rs`):

| Field | Class |
|---|---|
| Stores (entities and their components) | saved |
| Scheduler order, with a stored sequence for R8 | saved |
| Awaited entity | saved |
| Id counter | saved |
| RNG state | saved |
| Clock | saved |
| Map | saved, or derivable from seed and content once world generation exists |
| Indexes | derivable |
| Perceived changes | external: sent to the controller, not world state |
| Command log | external: replay evidence, cut at each version (methodology) |

- A test restores a world while it awaits a controller. The next offers
  and the order of perceived changes must match the original run (R19).
- A new field must be classified before it merges. Any in-flight exchange
  state joins the table when ADR 0002 introduces it.

### F7 — Code organization

- A component is plain data, declared in the module of its domain:
  `combat.rs` holds `Health`, `needs.rs` holds `Hunger`.
- A system is a rule in its domain module: an action rule (`resolve` for
  one action) or a scheduled process (hunger growing, as a scheduler
  event). Both return effects.
- The schema assembles the components from the modules and owns the
  storage layout. A system never sees the layout, so a measurement can
  move a component between tables without touching the rules.
- Modules by domain, never `components/` and `systems/` (methodology
  section 4).

## Deferred mechanisms and the slice that measures them

| Mechanism | Lab evidence | Decided in |
|---|---|---|
| Registry storage and identity rule | flat columns, 21 B per id, lookup 22.8 ns at 1e7 ids | slice 5 and its design |
| Table layout (rows, Robin Hood, canonical) | Robin Hood get 3.8 ns, churn 15.8 ns; reading 6 values: rows 9 ns, columns 26 ns | first slice whose bench shows a cost |
| Occupancy index | query 1.1 ns against 30 µs for a scan | slice 3 |
| Schema macro | 13 compile-fail cases | when hand-written stores repeat |
| Undo journal and rollback | rollback of a 10-action turn 0.44 µs | slice 6, if measured |
| Incremental world hash | divergence found in 1 to 6 µs against 7.5 ms | slice 6, if measured |
| Regions, windows, id blocks | 4.9x to 7.6x on 18 threads; W must not exceed the minimum action cost | ADR 0002 |
| Cohorts | 3.7 ns per member against 28.5 ns per individual | level-of-detail design |

Lab claims to discount:
- "7.0 ns against 72.6 ns for hecs": a naive `Vec` scores 7.9 ns.
- "Sorted rows 1.7x slower": measured against a 71 ns toy action.
- The lab's common brief preset "ids never reused".

Conditions recorded for rollback, if it comes: run it after `catch_unwind`,
never in `Drop`; a failed rollback falls back to plain quarantine;
quarantine stays the policy (constitution).

## Slice order

Each slice is a game feature first. Slices 1, 2 and 5 need a design source
before their spec: a spec cannot invent a mechanic.

1. **A second verb.** `act` becomes choose, resolve, apply (F4). Needs a
   design for the verb.
2. **Death.** Never-reused ids, total lookup, a corpse, a first trace
   (F2, F3). Needs a design for death and traces. Its spec may split it
   into several playable slices (for example dying, then the corpse, then
   the trace) if each has its own acceptance test.
3. **A larger map.** An occupancy index replaces the linear scan; first
   layout bench. Must ride on a designed exploration feature, never ship
   as an index alone.
4. **Save at disconnect.** Closed world state, stored R8 sequence, the
   restore-at-every-turn test (F5, F6). Needs ADR 0004 (save format).
5. **The heir.** Identity outlives embodiment; the registry for bonded
   entities (F3). Needs the lineage design.
6. **A second store and catch-up.** Not a slice: an epic (ADR 0002,
   regions, catch-up, then the journal and the hash if measurements justify
   them). Its own spec re-slices it.

`slice-check` (2026-11-22): slices 1 to 5 each touch data, behavior and a
test, and each is playable. Item 6 was rejected as a slice.

## Open for the human

- **Stop trigger for ADR 0002:** design rule R8 ("the entity that became
  due first acts first") has no meaning across parallel regions. Every
  parallel prototype broke ties by (due, id).
- Typed per-table ids: stored references stay untyped `EntityId`; a typed
  id would only be a short-lived witness returned by a lookup. A corpse is
  a new entity linked to its body's identity. To confirm with the death
  design.
- The chronicle: the lab measured events threaded per entity. It is a game
  feature without a design document yet.

## Guardrails

- F5 differential test.
- Ban `RefCell`, `Any` and `TypeId` in `ash-sim` (constitution change).
- No data-model-only slice.

## Glossary proposals

- Identity, embodiment, store, effect, total lookup: not in `CONTEXT.md` yet.
- Term clash: `CONTEXT.md` defines "snapshot" as the perceived state sent
  to a controller. The methodology uses "snapshot" for saves (ADR 0004).
  This document says "save" for the second meaning.
