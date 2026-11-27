# 0002 — Attack

## Status

Delivered (2026-11-27). The human played the milestone and approved the
glossary batch.

## Context

The character can strike a creature, and a creature can die. This is the
second verb, and the first game slice that brings the ADR 0001 contract
into the code: choose, resolve, apply (F4), never-reused ids (F2) and total
lookup (F3). ADR 0001 forbids a data-model-only slice (Consequences;
`docs/plans/0003-ecs-brainstorm.md`, Guardrails). The contract therefore
arrives with the verb that needs it.

Every behavior traces back to the attack design ("A3" = attack rule 3) or
to the action and time design ("R13" = rule 13). The slicing was decided in
the brainstorm of 2026-11-22 after an `advisor` pass and a second-model
read. Both judged a single first slice too large, so S1 and S2 split it.

Declared gaps, in the manner of spec 0001 for R12 and R19:

- S1 and S2 resolve hits and misses, but a hit removes nothing until S3
  (A6).
- Nothing lowers the character's hit points in this spec (A13). Its own hit
  points are shown, and they stay at 10.
- With 6 hit points and 3 damage, play shows only unhurt, wounded, then
  death. Badly wounded and dying exist in the rules and in unit tests.
  Tuning decides whether the numbers change (A7).

## References

- Design: `docs/design/attack.md`, `docs/design/action-and-time.md`.
- Brainstorm records: `docs/plans/0004-attack-brainstorm.md`,
  `docs/plans/0003-ecs-brainstorm.md` (F2 to F7, slice order).
- ADRs: `docs/adr/0001-simulation-data-model.md`.
- Perf budget share: none: cold code. Two entities fight on a fixed map.

## Simplest alternative

Keep `World::act` as an in-place mutation and add the attack branch beside
the move branch. Rejected: ADR 0001 F4 makes `apply` the only write path,
and its Consequences make the next game slice bring the contract.

Also rejected after the `advisor` pass: a first slice that brings the
contract alone on Move, Wait and bump, with no behavior change. It breaks
the "no data-model-only slice" guardrail. It would also shape `Effects`
before any verb needs randomness or damage.

## Multiplayer impact

- A3 lets a character strike any adjacent entity, including another
  character. A character at zero hit points needs the character's death,
  which is out of scope. One world per player makes this impossible in
  production. The shared-world spec must resolve it before two characters
  share a world. Until then, a character that reaches zero hit points is a
  violated invariant.
- Hit points of an entity go only to that entity's controller, like its
  costs (R18). Every other controller perceives a condition (A10).
- Disposition is a property of a creature. Characters have none, so a
  move into another character stays a bump (A2).

## Slices

Sequential. Each slice reaches the browser: the simulation, the wire, the
WASM codec and the web client. Inside a Rust slice, the first commit lays
down the signatures with a failing acceptance test.

### - [x] S1 — The character strikes on purpose

- Files: `crates/ash-sim/src/{lib,world,action,perception,wander}.rs`,
  `crates/ash-sim/src/combat.rs` (new), `crates/ash-sim/src/effect.rs`
  (new), `crates/ash-sim/tests/{attack,walk,creature,time_passes}.rs`,
  `crates/ash-protocol/src/message.rs`,
  `crates/ash-protocol/tests/decode_any_bytes.rs`,
  `crates/ash-server/src/{action,perception}.rs`,
  `crates/ash-server/tests/attack_over_websocket.rs` (new),
  `crates/ash-wasm/src/lib.rs`, `web/src/{main,model,draw,keys,protocol}.ts`,
  `web/tests/web_model.test.js`, the recorded web fixtures.
- Behavior:
  - Migration first, with every existing test and web fixture green:
    `resolve(&World, &Perception, actor, Action, &Rng) -> Result<Effects,
    Refused>` is pure. It reads the actor's perception for refusals and the
    true world for outcomes (F4). `apply(&mut World, Effects)` is the only
    write path: positions, clock, scheduling, RNG state, perceived changes
    and entity creation all go through it. Move, Wait and bump go through it
    unchanged.
  - `Action::Attack(Direction)`, base cost 100 divided by speed (A5). It is
    offered for each direction whose adjacent tile holds an entity (A3).
  - An attack hits when `draw % 100 < 70` (A6). A miss changes nothing
    else; the attacker pays the cost.
  - The perceived change replaces `bumped` with an outcome: stepped,
    waited, bumped, hit or missed, each naming its target where it has one
    (A11).
  - The web maps an attack to a key that is not already mapped, followed by
    a direction. The log shows hits and misses.
  - A move into any entity stays a bump until S2.
- Invariants:
  - `resolve` reads the RNG and never mutates it. `Effects` carries the
    next RNG state. A refusal draws nothing, writes nothing and advances
    nothing (F4, R10). The live world and the command-log replay never
    diverge.
  - The wander never chooses `Attack`: it filters the offers before it
    draws. The offers stay the same for every controller (R3, A13).
  - No message carries the odds of a hit (A12) or another entity's speed or
    cost (R18).
- Perf budget: none.
- Acceptance: `cargo xtask accept attack` — with a fixed seed, explicit
  attacks on the creature produce a perceived hit and a perceived miss,
  each costing 100. After the wander has drawn, an attack into an empty
  direction is refused: RNG state, clock, offers, command log and the
  replay that follows are unchanged. The wander's choice never draws among
  attack offers. The run replays identically. Over WebSocket, the changes
  carry outcomes, with no odds. The decoder rejects unknown tags, truncated
  payloads and impossible action and outcome pairs.

### - [x] S2 — A move into a hostile creature is an attack

- Files: `crates/ash-sim/src/{world,combat,action,perception,controller}.rs`,
  `crates/ash-sim/tests/hostility.rs` (new),
  `crates/ash-protocol/src/message.rs`,
  `crates/ash-protocol/tests/decode_any_bytes.rs`,
  `crates/ash-server/src/perception.rs`,
  `crates/ash-server/tests/hostility_over_websocket.rs` (new),
  `crates/ash-wasm/src/lib.rs`, `web/src/{model,draw,protocol}.ts`,
  `web/tests/web_model.test.js`, the recorded web fixtures.
- Behavior:
  - Each creature has a `Disposition`: hostile or neutral (A4). The
    character perceives it: the snapshot carries it for each creature, and
    the web shows it.
  - The skeleton world holds the character, the current creature made
    hostile, and a new neutral creature. Both scenarios of the design's
    player experience are playable.
  - A move into a tile held by a creature the actor perceives as hostile
    resolves to an attack on it (A1). A move into any other entity stays a
    bump (A2, R13).
- Invariants: the move's outcome is an attack outcome, with the attack's
  cost (A5). A disposition is perceived as a kind, never a number (R18).
- Perf budget: none.
- Acceptance: `cargo xtask accept hostility` — with a fixed seed, a move
  into the hostile creature is a perceived hit or miss costing 100. A move
  into the neutral creature is a bump. An explicit attack on the neutral
  creature is a hit or a miss. The run replays identically. Over WebSocket,
  the snapshot carries dispositions, and the decoder rejects an unknown
  disposition.

### - [x] S3 — The creature dies

- Files: `crates/ash-sim/src/{world,combat,effect,perception,scheduler}.rs`,
  `crates/ash-core/src/entity.rs`, `crates/ash-sim/tests/death.rs` (new),
  `crates/ash-protocol/src/message.rs`,
  `crates/ash-protocol/tests/decode_any_bytes.rs`,
  `crates/ash-server/src/perception.rs`,
  `crates/ash-server/tests/death_over_websocket.rs` (new),
  `crates/ash-wasm/src/lib.rs`, `web/src/{model,draw,protocol}.ts`,
  `web/tests/web_model.test.js`, the recorded web fixtures.
- Behavior:
  - `Health { current, max }` lives in `combat`. The character has 10, a
    creature 6 (A7).
  - A hit returns a damage effect of 3 (A6). When the target's hit points
    reach zero, the same `Effects` removes it (A8). `apply` writes both at
    once.
  - `PerceivedChange` becomes an enum: an act (entity, action, outcome) or a
    death (entity). A lethal hit is perceived as the hit, then the death,
    at the same time (A11).
  - The snapshot omits dead entities. The web removes a dead entity's
    position and logs its death.
- Invariants:
  - F2: a world counter allocates ids, never `entities.len()`. An id is
    never reused. A counter overflow is an `invariant!` crash, covered by a
    boundary test. `EntityId` stays `u32`: the width is a mechanism.
  - F3: a lookup returns embodied or issued-but-not-embodied. An id the
    world never issued is an `invariant!` crash.
  - The scheduler keeps a dead creature's entry. `advance` looks it up and
    skips it when it is not embodied.
  - No living entity has zero hit points (A8). The awaited entity is always
    embodied.
- Perf budget: none.
- Acceptance: `cargo xtask accept death` — with a fixed seed, the character
  strikes the hostile creature until it dies. The death is perceived after
  the lethal hit. The creature leaves the snapshot, and the world keeps
  running past its scheduler entry; at a tie, that entry acts nothing and
  draws nothing. An entity created afterwards gets a new
  id. The run replays identically. Over WebSocket, the death reaches the
  client and the dead id never appears again. The decoder rejects a
  malformed death change.

### - [x] S4 — The character sees the fight

- Files: `crates/ash-sim/src/{combat,perception,effect}.rs`,
  `crates/ash-sim/tests/condition.rs` (new),
  `crates/ash-protocol/src/message.rs`,
  `crates/ash-protocol/tests/decode_any_bytes.rs`,
  `crates/ash-server/src/{perception,connection}.rs`,
  `crates/ash-server/tests/condition_over_websocket.rs` (new),
  `crates/ash-wasm/src/lib.rs`, `web/src/{model,draw,protocol}.ts`,
  `web/tests/web_model.test.js`, the recorded web fixtures.
- Behavior:
  - The snapshot becomes per character. It carries the character's own
    hit points, current and max (A9). The web shows them as numbers.
  - `Condition` derives from `Health` with integer arithmetic: unhurt at
    `current == max`, dying at `current * 4 < max`, badly wounded at
    `current * 2 < max`, wounded otherwise (A10). It is derivable state,
    never stored (F6).
  - The snapshot carries the condition of every other entity.
  - A hit that crosses a threshold adds a condition change after the hit
    (A11). A lethal hit adds no condition change: the death follows the
    hit.
  - The web shows each entity's condition beside its name and logs
    condition changes.
- Invariants: no message carries another entity's hit points (A10, R18).
- Perf budget: none.
- Acceptance: `cargo xtask accept condition` — one hit moves the hostile
  creature from unhurt to wounded, perceived after the hit. A unit test
  covers the four thresholds. The character's snapshot holds its own hit
  points. Over WebSocket, no field received for another entity decodes to
  hit points.

## Glossary proposals

Approved by the human at the milestone (2026-11-27) and added to
`CONTEXT.md`. Each definition restates the source named beside it. A second,
independent read reworked the first draft.

New entries:

| Term | Proposed definition | Source |
|---|---|---|
| Combat | The system of attacks between entities. | Pillar 3; action and time, "the combat design" |
| Attack | An action that strikes an adjacent entity: it hits or it misses. A character's move into a tile held by a creature it perceives as hostile is an attack on that creature. | A1, A3, A6 |
| Hit | An attack that lowers its target's health. | A6 |
| Miss | An attack that does not hit. The attacker still pays the action cost. | A6 |
| Damage | How much a hit lowers its target's health. Today, a fixed number of hit points. | A6, Out of scope |
| Health | The bodily state of an entity. A character knows its own in numbers and perceives another entity's only as a condition. | Pillar 4, A9, A10 |
| Hit points | Today's measure of health: a current and a maximum number. A hit lowers the current number; at zero the entity dies. | A6 to A8, Out of scope |
| Condition | An entity's health as a kind, never a number: unhurt, wounded, badly wounded or dying. All a character perceives of another entity's health. | A10 |
| Disposition | Whether a creature is hostile or neutral. A character perceives which. | A4 |
| Death | The end of an entity's embodiment for good: the entity leaves the world. Today, at zero hit points. | A8, ADR 0001 point 3 |
| Outcome | What an action did: today stepped, waited, bumped, hit or missed. A character perceives the outcome, never the action submitted. | A11, accepted amendment of S1 |

Amended entries:

| Term | Proposed definition | Source |
|---|---|---|
| Bump | A move into a tile another entity holds that is not an attack: the mover stays, pays the move, and both perceive it. | A1, A2, R13 |
| Wander | A simulation controller that picks evenly among the actions the rules allow, except attacks. | A13, S1 invariant |
| Snapshot | The whole state one character perceives, sent when its controller connects. | S4 Behavior, A9, A10 |
| Quarantine | The state of a world whose simulation violated an invariant. Only that world stops. | The entry itself; "hit" is now a term |

Second batch, the phase 1 names with no entry:

| Term | Proposed definition | Source | Code name |
|---|---|---|---|
| Entity | A thing of a world that acts when it is due, at its own speed, under the same rules whatever its controller. Today, a character or a creature. | R3, R6, R8; entries Character, Creature | `EntityId`; the private row `Entity` in `world.rs`; `actor` for the entity taking an action |
| Action | What an entity does when it is due. It takes effect at once, and its cost is the time the entity spends before it acts again. Today, move, wait or attack. | R4, R7, A3 | `Action` |
| Scheduler | What orders entities by the time they are due. Of two due at the same time, the one that became due first acts first. | R8, ADR 0001 point 6 | `Scheduler` |
| Refusal | The rules turning down a move or an attack, judged on what the entity perceives. A refused move or attack is not an action: it costs nothing, changes nothing and advances no clock. | R10, R11, A3, ADR 0001 point 4 | `Refused`, `ServerMessage::Refused` |
| Offer | An action the rules allow the awaited entity, with the cost it would pay. A controller chooses only among its offers; a wait is always one. | R9, R14, ADR 0001 point 4 | `Offer`, `World::offers`, `Awaited { offers }`; the JSON key stays `actions` |

Choices behind the rows:

- Each rule has one home. A1 lives in "Attack", and "Bump" excludes it.
  "Hostile" and "neutral" have no row: the values of a kind live in the
  entry of the kind, as floor and wall live in "Terrain".
- "Health" is the term, and hit points are today's measure of it. The types
  `Health` and `PerceivedHealth`, the field `health` and the wire key keep
  their names: `PerceivedHealth` holds hit points or a condition, and both
  are health. No rename follows.
- "Combat" is the term of the system, so the module `combat` keeps its
  name. `Health` and `Condition` move to a module `health` when the first
  rule outside combat reads or writes health.
- "Strike" stays plain English, the verb of "attack": the design uses it
  (A3, A13), and a strike can miss. Two doc comments used "struck" for
  "hit"; this change corrects them.
- `Effect::Damage` carries the health a hit leaves, not the damage. Rename
  it when the first effect that raises health exists.
- The rows write "a character": a glossary states what holds for any
  character, and shared worlds must stay possible.

The glossary gains an "Avoid" column: the words never to use for a term.
A cell needs evidence: the word names the same concept in the repository,
or it is the obvious synonym the term replaces. Three cells are filled:
"monster" for creature, "available action" for offer, "HP" for hit points.
A word the attack design uses stays out of the column, so "hostility",
"blow" and "odds" are not avoided.

Recorded with the approval:

- "Health" rests on one line of pillar 4, where health is one item of a
  list. The attack design never uses the word.
- "Combat" as a term makes `docs/design/attack.md` the first version of
  "the combat design" that the action and time design names.
- The entry "Action cost" said the scheduler orders actions. R8 orders
  entities, so the entry now states the cost alone.
- The next spec carries this table from the day it is approved, with the
  code name of each term, so the milestone approves only the differences.
  A change of `docs/templates/spec.md` adds the section.

## Proposed amendments

- 2026-11-27, accepted by the human (found in S1, second-model read): a
  perceived change carries one outcome and no action. The outcome is stepped
  with its direction, waited, bumped, hit or missed; the last three name
  their target. An impossible pair, such as a wait that hit, is then
  unrepresentable in the simulation, on the wire and in the web model.
  This amends:
  - S1 Behavior: "The perceived change replaces `bumped` with an outcome"
    reads "replaces `action` and `bumped` with one outcome".
  - S1 Acceptance: "The decoder rejects unknown tags, truncated payloads and
    impossible action and outcome pairs" reads "The decoder rejects unknown
    tags, truncated payloads and an outcome whose target is its own actor".
  - S3 Behavior: "an act (entity, action, outcome)" reads "an act (entity,
    outcome)".
  - Consequence for S2: a witness cannot tell a strike made by a move from
    an explicit attack. The command log keeps the submitted action.
- 2026-11-27, accepted by the human (second-model read): S2 says a move
  into a creature "the actor perceives as hostile" resolves to an attack. A
  wandering creature that moves into a hostile creature would then attack
  it, against A13 (only the character attacks). S2 Behavior now reads: only
  a character's move into a creature it perceives as hostile resolves to an
  attack (A1). A creature's move into any entity stays a bump (A13, R13).
- 2026-11-27, recorded (found in S1): the world no longer stores the clock
  and the awaited entity. It derives both from the scheduler head, so an
  effect cannot disagree with the scheduler. The F6 table of
  `docs/plans/0003-ecs-brainstorm.md` is updated: both are derivable.
- 2026-11-27, recorded (found in S1): the S1 acceptance runs as four
  commands: `cargo xtask accept attack`, `accept attack_over_websocket`,
  `accept decode_any_bytes` and `accept web_model`.
- 2026-11-27, open (found in S1): an attack on the diagonal across a wall
  corner is allowed. A3 says "adjacent", and R11 forbids only moves past a
  corner. S1 follows A3 as written. The human questions whether diagonals
  stay at all; that belongs to a design brainstorm, not to this spec.
  S2 adds a second case: a diagonal move past a wall corner into a hostile
  creature is refused, because R11 is judged before A1, while the explicit
  attack there is allowed. A test pins it. Issue #37 holds the question.
- 2026-11-27, recorded (found in S2): the S2 acceptance runs as four
  commands: `cargo xtask accept hostility`, `accept
  hostility_over_websocket`, `accept decode_any_bytes` and `accept
  web_model`.
- 2026-11-27, recorded (found in S2): a disposition sits on the wander
  controller, so a character cannot hold one. The glossary defines a
  creature by its controller. A second simulation controller, or a
  controller hand-over, moves the disposition onto the entity.
- 2026-11-27, recorded (found in S2): the neutral creature of the skeleton
  world starts at (1, 4) with speed 2 (R6). The spec gave neither.
- 2026-11-27, recorded (found in S2): an offer shows the cost of the
  submitted move, and the resolved attack charges its own cost. Both are
  100 today (A5, R5). When the base costs diverge, the offer must show the
  cost of the resolved action (R9); a test trips on that day.
- 2026-11-27, proposed for the milestone glossary batch (found in S2): the
  entry "Bump" in `CONTEXT.md` gains an exception: a character's move into
  a creature it perceives as hostile is an attack, not a bump (A1).
- 2026-11-27, recorded (found in S3): the S3 acceptance runs as four
  commands: `cargo xtask accept death`, `accept death_over_websocket`,
  `accept decode_any_bytes` and `accept web_model`.
- 2026-11-27, recorded (found in S3): S3 changed files outside its list.
  `crates/ash-sim/src/action.rs` returns the effects of a hit. The enum
  shape of a perceived change reaches the existing simulation and server
  tests and `web_fixtures.rs`. The test helper `outcome.rs` is renamed
  `change.rs`.
- 2026-11-27, recorded (found in S3): a dead creature's scheduler entry
  stays until it reaches the head. `advance` then returns an `Unschedule`
  effect and `apply` drops the entry: the skip is a write, so it goes
  through `apply` (F4). One death leaves no permanent entry.
- 2026-11-27, recorded (found in S3): `EntityIds` lives in `ash-core`
  beside `EntityId`, so the bits of an id stay private to that module.
  `apply` issues the id; a creation effect carries none. The counter holds
  the number of ids issued, so the last value of `u32` is never issued.
- 2026-11-27, recorded (found in S3): a character whose hit points reach
  zero is a violated invariant, raised in `resolve` before any write. The
  design puts the death of a character out of scope, and only a character
  attacks (A13).
- 2026-11-27, recorded (found in S3): the snapshot lists the entities in
  creation order. That order equals the storage order today, and a test
  pins it. A storage change must produce the order from a key, not from the
  layout (ADR 0001, point 5).
- 2026-11-27, recorded (found in S3, second-model read): the two-case
  lookup of F3 is private to the world. The public read `World::position`
  takes an embodied entity and crashes on a dead id. Only tests call it
  from outside the crate, and no production code holds an id across a
  death. The first such caller exposes the two cases.
- 2026-11-27, accepted by the human (found in S3, both reviews): a
  perceived change is a struct `{ time, entity, kind }`, with the kinds act
  (outcome), death and condition. It replaces the enum that repeats the
  time and the entity in each variant. This amends S3 Behavior:
  "`PerceivedChange` becomes an enum: an act (entity, action, outcome) or a
  death (entity)" reads "`PerceivedChange` is a struct: a time, an entity
  and a kind. The kind is an act (outcome) or a death". S4 adds the kind
  condition. The wire bytes and the JSON do not change.
- 2026-11-27, recorded (found in S3, second-model read): a creation during
  play is not in the command log, so a replay omits the new entity. No
  production code creates an entity during play. Issue #40 holds it.
- 2026-11-27, recorded (found in S4): the S4 acceptance runs as four
  commands: `cargo xtask accept condition`, `accept
  condition_over_websocket`, `accept decode_any_bytes` and `accept
  web_model`.
- 2026-11-27, recorded (found in S4): S4 changed files outside its list.
  `crates/ash-sim/src/world.rs` writes the health a hit leaves and holds the
  test of a hit that crosses no threshold, because `apply` is private. The
  new variant, the viewer of a snapshot and the health field reach the
  existing simulation and server tests. `web/index.html` and
  `web/src/main.ts` gain the list of entities.
- 2026-11-27, recorded (found in S4): the snapshot is built for one
  character, `snapshot(world, character)`. The server passes the character
  the connection hosts, never the awaited entity.
- 2026-11-27, recorded (found in S4): a perceived entity holds one health:
  hit points for the viewer's own character, a condition for every other
  entity. It never holds both. The wire form can still give hit points to
  any entity; the server is the boundary (A10, R18). Hit points on the
  snapshot itself would make the rule structural. That is a change of
  shape for the human.
- 2026-11-27, recorded (found in S4): a damage effect carries the health
  the hit leaves, so `hit` computes it once and `apply` only writes (F4).
  Two hits on one target in one `Effects` would lose the first. No rule
  produces that today.
- 2026-11-27, recorded (found in S4): with 6 or 10 hit points and a damage
  of 3, every non-lethal hit crosses a threshold. The test of a hit that
  crosses none uses a health built for it.
- 2026-11-27, open (found in S4, all reviews): no message gives a struck
  character its new hit points. Its controller receives a condition change
  naming its own character, and the web then shows that condition instead
  of the numbers (A9). It needs a second character in the world; a world
  hosts one today, and only a character attacks (A13). The first spec that
  lets a character be struck must decide the message. Issue #42 holds it.
