# 0001 — Walking skeleton

## Status

Approved (2026-11-20). S1 and S4 to S7 wait for the constitution PR (S0).
S2 and S3 run before S1, so their acceptance runs through `cargo test`
until `cargo xtask accept` exists.

## Context

Phase 1 of `docs/plans/0000-methodology.md` (section 13): an `@` that moves
over WebSocket, with a green replay. It proves the crate boundaries, the
determinism contract and the agent workflow on real game code, and it gives
the human a first thing to play.

Every behavior traces back to the action and time design. Rules are cited
by number ("R8" = rule 8). Two rules get no code in phase 1:

- R12 (a move into an unperceived obstacle) cannot trigger under R16, where
  the character perceives the whole map. Making it testable would need
  perceived state per character, and the design excludes any remembered map.
- R19 holds only while the server runs. Saving to disk is out of scope.

Internals stay deliberately simple and replaceable (methodology, section 4):
`Vec` storage, a single-threaded scheduler, a hand-written codec. They must
not settle ADR 0001 (data model), 0002 (parallelism) or 0003 (wire format).
The rendering is a throwaway Canvas 2D grid. The human expects the game to
render with three.js on WebGPU later, so the client keeps its model of the
perceived world apart from its drawing code.

## References

- Design: `docs/design/action-and-time.md`.
- Brainstorm record: `docs/plans/0002-action-and-time-brainstorm.md`.
- ADRs: none. Phase 1 precedes ADRs 0001 to 0003 and feeds them.
- Perf budget share: none: cold code. `docs/perf/budgets.md` does not exist yet.

## Simplest alternative

Proposed by the `advisor`:

> The simplest skeleton keeps the crate boundaries of the methodology and
> drops everything else. The server is synchronous: `std::net::TcpListener`,
> one thread per connection, sync `tungstenite`, no async runtime. A registry
> maps a resume token to one world, and each world has its own lock. The codec
> is hand-written binary: a tag byte, then fixed-width little-endian fields,
> for about six messages. The simulation adds no dependency: a `Vec` of
> entities, a `BinaryHeap` keyed by due time then insertion sequence, and a
> hand-written RNG in `ash-core`. Replay is a `cargo test`: one seed and one
> command log run twice, comparing the perceived-change stream and the final
> state.

Adopted as the basis of this spec, with these decisions per element:

- Sync server: adopted. One world per player and a turn-based scheduler
  need no concurrency. `tokio-tungstenite` is the upgrade path.
- JSON instead of binary: rejected. Methodology section 1 fixes binary
  messages.
- A TypeScript decoder instead of WASM: rejected. Two codecs drift, and
  `.claude/rules/protocol.md` makes the crate the client's codec. A throwaway
  spike (2026-11-20, rustc 1.100.0, wasm-bindgen 0.2.130) proved the path:
  a no_std `ash-protocol` plus a thin `ash-wasm` export crate pass clippy
  under `forbid(unsafe_code)` on wasm32, `wasm-bindgen --target web` builds,
  a Node round trip works, and the `.wasm` weighs 20 KB. Only free functions
  were exported; other `#[wasm_bindgen]` forms were not tested.
- No protocol crate: rejected. Its message types are where the perception gate
  becomes checkable (R18).

## Multiplayer impact

- The player is an entity with a controller. No code path names "the player"
  as a global; the protocol speaks of entities.
- R2 stops time while the next character to act waits for its controller.
  In a shared world this would let one idle player freeze everyone, which
  contradicts pillar 1 ("only your lineage waits"). Phase 1 keeps one world
  per player, so the conflict cannot occur. The shared-world spec must
  resolve it before two controllers share a world.
- A resume token identifies one controller of one world. A second connection
  with the same token takes the controller; the older connection is closed.

## Constitution needs

Batched into one human PR before S1 starts (call it S0). Each was found by
the `researcher`, the `advisor` or the toolchain spike.

- Dependency matrix (`allowed.rs`): `ash-protocol` → `ash-core`;
  `ash-server` → `ash-core`, `ash-sim`, `ash-protocol`, `tungstenite`;
  `ash-wasm` → `ash-protocol`, `wasm-bindgen`.
- `deny.toml`: a crate-scoped exception
  `{ crate = "unicode-ident", allow = ["Unicode-3.0"] }`. Every proc-macro
  pulls it in (through `thiserror`, which `tungstenite` requires, and through
  `wasm-bindgen-macro`). Tested by the `researcher`: with it, the set passes.
- Fuzzing: `libfuzzer-sys` is `(MIT OR Apache-2.0) AND NCSA`. Either allow
  NCSA for it, or keep the fuzz crate outside the workspace (cargo-fuzz's
  default layout), where `cargo deny` does not see it. Recommendation: the
  second, with the nightly toolchain the methodology already plans.
- `rust-toolchain.toml`: add the `wasm32-unknown-unknown` target.
- Release profile: `overflow-checks` must cover `ash-protocol`. PR #7 already
  extends it to the whole release profile; merge it first.
- `xtask/src/main.rs` is protected; S1 and S6 add subcommands to it. Their
  PRs need the `constitution-approved` label.
- CI: a job that builds the web client and runs its tests (S6).
  Multi-architecture replay (section 10) needs no new job if the human
  agrees: the replay test runs on x86 in `pr.yml` and on ARM in
  `nightly.yml`, both through `cargo xtask verify`. A cross-architecture
  hash comparison would be a new job.

## Slices

Sequential. Each slice ends in a behavior a test or a human can observe.
The design gaps found by the review are resolved in the design (R8, R20).

### - [ ] S1 — The acceptance command

- Files: `xtask/src/accept.rs`, `xtask/src/main.rs` (protected: label).
- Invariants: `cargo xtask accept <test>` runs exactly the named test across
  the workspace and exits non-zero when it fails, when it is missing, or when
  the filter matches more than one test.
- Perf budget: none.
- Acceptance: `cargo test -p xtask accept` — negative tests: a failing test,
  a missing test and an ambiguous filter each make `accept` exit non-zero.

### - [ ] S2 — Time passes; replay is green

- Files: `crates/ash-core/src/{lib,time,entity,rng,invariant}.rs`,
  `crates/ash-sim/src/{lib,world,scheduler,action,perception}.rs`,
  `crates/ash-sim/tests/time_passes.rs`.
- Behavior: a world holds one character that can only wait (R4, base cost 50,
  R5). Speed divides the base cost (R6). The cost is paid after the effect
  (R7). Entities are scheduled by due time; ties go to the one that became due
  first (R8). Time stops while the next character awaits its controller (R2).
  The awaited character can query its available actions and their costs (R9).
  Every change is perceived in order with its time (R15 to R17).
- Invariants: `ash-core` and `ash-sim` stay `no_std` with no new dependency.
  Game time is a newtype; overflow is an `invariant!` crash. The RNG is a
  hand-written generator in `ash-core`, pinned by known-output tests. The
  command log records only accepted commands. The replay test runs inside
  `cargo xtask verify`, so `pr.yml` (x86) and `nightly.yml` (ARM) both run it.
- Perf budget: none.
- Acceptance: `cargo xtask accept time_passes` — a scripted series of waits
  gives the expected clock and perceived-change stream; a same-time tie
  resolves in due order; a second run of the same seed and command log gives
  an identical stream and final state.

### - [ ] S3 — The character walks

- Files: `crates/ash-core/src/tile.rs`, `crates/ash-sim/src/map.rs`,
  `crates/ash-sim/src/action.rs`, `crates/ash-sim/tests/walk.rs`.
- Behavior: a fixed map with walls. Move in 8 directions for base cost 100
  (R4, R5). A move into a perceived wall is refused: no cost, no clock
  change, no RNG draw, nothing logged (R10). A diagonal between two
  orthogonal walls or along a wall corner is refused (R11). Under whole-map
  perception (R16), moves appear in the perceived-change stream (R17).
- Invariants: coordinates are 3D; the skeleton uses one level. Typed tile
  indices, no raw indexing.
- Perf budget: none.
- Acceptance: `cargo xtask accept walk` — a scripted walk gives the expected
  positions and clock; a wall move and a blocked diagonal leave clock, log
  and stream unchanged; the walk replays identically.

### - [ ] S4 — The character walks over WebSocket

- Files: `crates/ash-protocol/` (new), `crates/ash-server/` (new),
  `crates/ash-server/tests/walk_over_websocket.rs`, `fuzz/` (outside the
  workspace).
- Behavior: a client connects with a resume token, receives a timestamped
  snapshot, then a message that it is awaited, carrying each available
  action and its cost (R9; the client never hardcodes a cost). It sends
  commands and receives perceived changes in order (R17). A refused command
  gets a defined reply and advances nothing (R10). A reconnect with the same
  token finds the world unchanged (R19).
- Invariants: `ash-protocol` depends on `ash-core` only; its messages name
  entities, never "the player". Decoding never panics on any input. The
  server never steps a world while it holds the registry lock. The costs of
  the awaited entity go only to that entity's controller (R18).
- Perf budget: none.
- Acceptance: `cargo xtask accept walk_over_websocket` — a Rust WebSocket
  client walks, gets a refusal, disconnects while awaited, reconnects with
  its token, and finds the same clock, positions and pending turn; the world
  state equals a replay of the accepted commands. Plus a property test
  feeding random bytes to the decoder, and the fuzz target building on
  nightly.

### - [ ] S5 — Quarantine and takeover

- Files: `crates/ash-server/src/{registry,quarantine}.rs`,
  `crates/ash-server/tests/quarantine.rs`.
- Behavior: an invariant violation inside one world quarantines that world
  only; other worlds keep running. A quarantined world is never stepped
  again, and its command log is kept. Its client receives a defined message.
  A new connection for a character takes its controller, and the older
  connection is closed (R20).
- Invariants: the release profile keeps `panic = "unwind"`. No production
  command can trigger the test fault.
- Perf budget: none.
- Acceptance: `cargo xtask accept quarantine` — two clients, a fault injected
  into one world: that client gets the quarantine message, the other keeps
  walking. A second connection with the first token takes the controller, and the
  first connection is closed.

### - [ ] S6 — The character walks in the browser

- Files: `crates/ash-wasm/` (new), `xtask/src/play.rs`, `xtask/src/accept.rs`
  (resolve `web_model` to the Node test), `xtask/src/main.rs` (protected:
  label), `web/` (new: `index.html`, `src/*.ts`, `package.json`,
  `tsconfig.json`), `.gitignore`.
- Behavior: `cargo xtask play` builds the WASM package and the TypeScript,
  serves `web/`, and starts the server. The page draws the map and every
  perceived entity, maps keys to the 8 moves and to wait, shows the clock,
  the cost of each action before it is chosen (R9), and a log of perceived
  changes stamped with their time (R17).
- Invariants: the WASM package and the compiled TypeScript are git-ignored.
  The perceived-world model knows nothing about drawing, and neither knows
  "the player" beyond the controlled entity id. Toolchain: `wasm-bindgen-cli`
  pinned to the `Cargo.lock` version, `typescript` only, native ES modules,
  no bundler.
- Perf budget: none.
- Acceptance: `cargo xtask accept web_model` — Node's built-in test runner
  feeds a recorded message stream to the model and checks the map, positions,
  clock, offered costs and timestamped log; a key-mapping test checks the
  9 commands. Then the human plays.

### - [ ] S7 — The creature

- Files: `crates/ash-sim/src/{controller,wander}.rs`,
  `crates/ash-sim/tests/creature.rs`,
  `crates/ash-server/tests/creature_over_websocket.rs`.
- Behavior: a creature with speed 2 moves for 50 and waits for 25 (R6). It
  wanders with the deterministic RNG, choosing only among actions that will
  not be refused (R14). At world creation, entities become due in creation
  order, the character first (R8). A move into an occupied tile is a bump:
  the mover stays and pays the move, both entities perceive it, nothing
  else changes (R13). The browser shows the
  creature through the existing entity-generic model (R16).
- Invariants: the creature follows the same rules as the character (R3).
  No message field, in the snapshot or in a change, can carry another
  entity's speed, cost or next turn (R18).
- Perf budget: none.
- Acceptance: `cargo xtask accept creature` — a seeded run shows two creature
  moves per character move, costs 50 and 25, a bump perceived by both with
  no other effect, and an identical replay; over WebSocket, the snapshot and
  the changes received for the creature decode only to positions and
  perceived actions.

## Design gaps

Both gaps found by the second-model review were decided by the human on
2026-11-20 and are now design rules: creation order settles the first tie
(R8), and the newest connection takes the controller (R20).

## Glossary proposals

For the human at the milestone: **bump** (a move into an occupied tile),
**perception** (what a character learns of the world; the only channel to
the client), **speed** (the divisor of an entity's base costs), **base cost**
(an action's cost at speed 1), **map** (the fixed terrain of a world:
floor and wall tiles), **resume token** (identifies one controller of
one world across connections).

## Proposed amendments

- S3 (found in the slice): the file list also needs `crates/ash-sim/src/world.rs`,
  both `lib.rs` files, and `crates/ash-sim/tests/time_passes.rs` (its `submit`
  calls now handle a `Result`). No acceptance criterion changes.
- S3 (found in the S3 review): `available_actions` offered only wait once
  moves existed. S3 now offers wait and every move the map allows, with
  their costs (R9, and R14 for S7). No acceptance criterion changes.
- S4 (found in the S3 review): the snapshot needs the terrain (R16), and
  `ash-sim` has no accessor for it. S4 may add one to `crates/ash-sim/src/`.
- S4 (found in S4): S4 also touches `crates/ash-core/src/{entity,time}.rs`
  (raw-value accessors for the codec) and `crates/ash-sim/src/{map,world}.rs`
  (`World::terrain`, `World::terrain_width`, `World::entities`; the map module
  stays private). The client chooses its
  resume token (`u128`); tokens are not secrets in phase 1. `Awaited`
  carries the clock, so the client can show it. No acceptance criterion
  changes.
- S5 (found in the S4 review): today two connections with one token both
  drive the world, a panic poisons the world lock and later `Hello`s close
  silently, and sockets have no read timeout, so closing the older
  connection (R20) needs a way to interrupt its blocking read (for example
  a cloned `TcpStream` and `shutdown`).
- S5 (found in S5): S5 also touches `crates/ash-server/src/{connection,lib}.rs`
  and `crates/ash-protocol/` (messages `Quarantined` and `ControllerTaken`,
  with their round-trip and decoder tests). The old connection receives
  `ControllerTaken` before it closes, so a client knows not to reconnect.
  No acceptance criterion changes.
- S7 (found in the S4 review): the out-of-turn guard and the R18 guard of
  `ash-server` cannot be reached with one entity; S7 adds a negative test
  for each. Each world gets its own seed, kept in the registry, so worlds
  differ and replays stay possible.
- S6 (found in S4): `cargo xtask play` needs a server binary:
  `crates/ash-server/src/main.rs` joins S6's files.
- S6 (found in the S1 review): `accept` resolves Cargo integration targets
  only; S6 extends it to the Node `web_model` test. Added to S6's files.
