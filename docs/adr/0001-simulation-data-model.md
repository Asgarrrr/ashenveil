# 0001 — Simulation data model

## Status

Proposed.

## Context

The methodology (section 4) deferred the simulation data model to this ADR:
existing ECS, custom ECS, or per-component arrays. Phase 1 shipped a naive
`Vec` of entities in `crates/ash-sim/src/world.rs`.

Forces:

- `.claude/rules/sim.md`: `no_std` + `alloc`, `unsafe` forbidden, no floats,
  bit-for-bit determinism for one build.
- `docs/design/action-and-time.md`: a time-cost scheduler lets one entity
  act at a time (R2, R7, R8). Refusals depend on what the character
  perceives (R10, R12, R14). A disconnected world keeps its state (R19).
- Pillar 1 (lineage) and pillar 4 (traces): references to an entity must
  stay meaningful after it dies or leaves.
- Agents write most of the code: a compile error beats a runtime panic, and
  a panic quarantines a world.
- Two earlier attempts at this game died partly because foundation work
  delayed playable results.

The full brainstorm, with every challenge pass and the lab evidence, is
`docs/plans/0003-ecs-brainstorm.md`.

## Decision

Ashenveil does not adopt or build a classic ECS. It separates the permanent
identity of an entity from its embodiment in a store, and freezes only the
contract below. Every storage mechanism waits for the game slice that needs
it and for a measurement on the real game.

1. **Closed static schema.** The component set is closed and known at
   compile time. Every borrow is checked by the compiler: no `unsafe`,
   `RefCell`, `Any` or `TypeId`.
2. **Opaque, never-reused entity ids.** An id is scoped to its world, never
   reused, and carries no meaning in its bits or order. Allocation is
   immediate inside `apply` (R7). Exhausting the id space is a violated
   invariant.
3. **Total lookup.** A lookup returns "embodied here" or "issued but not
   embodied here", never a bare `None`. An id the world never issued is a
   violated invariant. Finer cases ("elsewhere", "dead") refine the second
   case later, each with the data it needs.
4. **Choose, resolve, apply.** A controller chooses from its perception and
   its offers. A pure rule resolves the action against the actor's
   perception (refusals) and the true store (outcomes). `apply` is the only
   write path. A refusal writes nothing, advances no clock and draws no
   randomness. Randomness is an explicit input.
5. **Unobservable layout.** No API exposes storage order or row positions.
   Order-sensitive work uses the scheduler order or an explicit key.
6. **Classified world state.** Every world field is saved, derivable or
   external, as tabled in `docs/plans/0003-ecs-brainstorm.md` (F6). The
   scheduler stores a sequence for R8.
7. **Code organization.** Components are plain data in their domain module.
   Systems are rules in their domain module: action rules or scheduled
   processes, both returning effects. The schema owns the layout; systems
   never see it.

## Alternatives considered

- **Existing ECS (bevy_ecs 0.20.1, hecs 0.11.2).** Rejected. Every
  mainstream Rust ECS uses `unsafe` (hecs: 237 lines, bevy_ecs: 2405).
  Their swap-remove storage makes iteration order depend on history. A
  game built on hecs (Egregoria) removed it for determinism and
  serialization.
- **Dynamic type-erased custom ECS.** Rejected. It needs an `unsafe`
  exception or runtime borrow checks that panic.
- **Generational ids.** Rejected. Slot reuse forces a second permanent
  identity for history. EVE Online's id recycling broke logs.
- **Freezing a storage mechanism now.** Rejected. An independent lab (a
  Fable lead and 13 Opus agents, 10 prototypes in
  `~/Documents/Projects/ashenveil-lab/`, outside the repository) measured
  candidates on an Apple M5 Pro under rustc 1.100.0, for example with
  `cargo run --release --manifest-path ~/Documents/Projects/ashenveil-lab/layouts/Cargo.toml --bin bench`:
  - Robin Hood table keyed by id: get 3.8 ns, churn 15.8 ns, 52 B per live
    entity at 1e5 live among 1e7 ids.
  - Sparse set: get 1.8 ns, churn 92 ns, 438 B.
  - `BTreeMap`: get 80 ns.
  - Rows sorted by id: 1.7x slower per action, against a 71 ns toy action.

  No prototype had an awaited controller or a disconnect. The numbers come
  from one machine. They justify no frozen mechanism; the slices
  re-measure on the real game.
- **Byte-canonical layout.** Not frozen. The non-canonical layout passes
  the restore test once passes iterate by id, so an unobservable layout is
  enough and keeps mechanisms replaceable.
- **Challenge passes.** Two advisors and two researchers (Opus, then Fable),
  a Fable review of the reframing against the lab sources, and a
  second-model read. Their findings and dissents are in the brainstorm
  document. Strongest dissent: the full lab design costs more to retrofit
  than to adopt. Answer: the game holds two entities today, and the
  methodology wants game measurements to drive mechanisms.

## Consequences

- Easier: replacing any storage mechanism, since layout is unobservable;
  adding a journal or a world hash, since all writes go through `apply`;
  proving that a controller cannot read hidden state.
- Harder: every rule produces effects instead of mutating in place, which
  adds a step to each new verb.
- The next slices are game slices that bring the contract into the code:
  a second verb, death, a larger map, save at disconnect, the heir. Each
  needs its design source first.
- Revisit if a measured budget cannot be met under this contract, or if
  ADR 0002 (partitions) needs a guarantee the contract forbids.
- Open for ADR 0002: design rule R8 has no meaning across parallel
  regions. This is a stop trigger for the human.
