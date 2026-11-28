# 0003 — Riposte and death

## Status

Approved (2026-11-27). The human approved the spec, its slicing and the
three readings of the sources below, as proposed.

## Context

A hostile creature strikes a character, and a character can die. A death
leaves a corpse. The controller of a dead character then drives a stranger
in the same world. The human approved both designs on 2026-11-27:
`docs/design/death.md` is new, and `docs/design/attack.md` gained rules 4,
8, 9, 11, 13 and 14.

Every behavior traces back to a rule. "A4" is attack rule 4, "D6" is death
rule 6 and "R17" is action and time rule 17. "F2" to "F6" are the points
of `docs/plans/0003-ecs-brainstorm.md` that ADR 0001 freezes.

The slicing keeps the six slices of
`docs/plans/0005-riposte-and-death-brainstorm.md`, in their order. The
hostile creature strikes in the last slice. Before it, no played world can
bring a character to zero hit points. The first death the human plays
leaves a corpse and a stranger.

Constraints found in the code, with the slice that treats each:

- The server binds one character to a world for good
  (`crates/ash-server/src/registry.rs`, `HostedWorld::character`). After a
  death, `step` would refuse every command, and a new connection would
  crash on the snapshot of the dead character. S4.
- The stranger comes from an effect of the lethal hit, inside `resolve`. A
  server call to `World::create_entity` would stay out of the replay
  inputs (issue #40). S3.
- `apply` issues every id (ADR 0001, point 2), so `resolve` cannot name
  the stranger. The effect that creates the stranger carries no id. S3
  and S4.
- The server must follow an id across a death. The two cases of the total
  lookup are private to the world today (spec 0002, amendment of S3). S3
  exposes the case it needs: the stranger of a dead character.
- The zero hit points of a character crash through `invariant!`
  (`crates/ash-sim/src/combat.rs`, `hit`). S3.
- A world keeps one list of perceived changes, and `Hello` counts the
  changes of the world. Perception becomes per character. S4 and S5.
- No perceived change gives a struck character its new hit points: only
  its snapshot holds them (issue #42). S5.
- A disposition sits on the wander controller (spec 0002, amendment of
  S2). A second simulation controller arrives. S6.
- `ash-server` gives its integration tests `serve` and `serve_with_fault`
  only, and no creature strikes before S6. No hosted world can strike a
  character in S4 or S5. S4 adds a test entry.

Readings of the sources, confirmed by the human with the approval:

- A6 still says "A miss changes nothing else". A4, as amended, makes a
  missed attack provoke, and decision 3 of 0005 rejects "hostile only
  when hit". S2 follows A4. The human may reword A6.
- D11 binds the stranger, not its controller. A controller keeps
  receiving the changes of each character it drove: "the stranger knows
  none of what happened; the player does" (death design, player
  experience). One line of the 0005 slice proposal says the opposite: "a
  reconnection receives no change from before the arrival". See "Simplest
  alternative".
- D4 says a character recognizes the corpse of an entity it "perceived
  alive". Under whole-map perception (R16), that is an entity that died
  after the character's creation or arrival.

Declared gaps:

- From S2 to S5, a hostile creature still wanders (A13). A provoked
  creature turns hostile and does not strike yet.
- S3 to S5 change nothing the human can play: only tests reach a struck
  character. S1, S2 and S6 are playable.
- A death costs no time, and it is the only way to regain hit points. The
  death design declares it and holds the question.
- A map with no free floor tile gives D8 no tile. It cannot happen today.
  It crashes through `invariant!` (death design, open question).
- A hostile creature strikes on the diagonal across a wall corner. S6
  follows A3 as written, and a test pins it. Issue #37 holds the question.
- Issue #40 stays open: `World::create_entity` can still run during play.
  The tests of this spec use it at setup only. A character it creates
  counts as created with its world: it recognizes every corpse (S4).

## References

- Design: `docs/design/death.md`, `docs/design/attack.md`,
  `docs/design/action-and-time.md`.
- Brainstorm records: `docs/plans/0005-riposte-and-death-brainstorm.md`
  (decisions, challenge passes, proposed slices),
  `docs/plans/0003-ecs-brainstorm.md` (F2 to F6).
- ADRs: `docs/adr/0001-simulation-data-model.md`.
- Issues: #42 (closed by S5), #40 and #37 (stay open).
- Perf budget share: none: cold code. Three entities act on a fixed map.

## Simplest alternative

Proposed by the `advisor` and adopted. It replaced a first draft of seven
slices.

- `Hello` keeps its form. Its count covers the perceived changes of every
  character its controller drove, in order: each from its creation or
  arrival to its death. With one character per world, this is today's
  count. The codec of `Hello` and the count of the page do not change.
  From S5, the WebSocket tests compare with the changes of their
  controller, not with the list of the world.
- The server keeps the first character of each token and never moves it.
  It finds the character its controller drives now through the world,
  which records the stranger of each dead character.
- Six slices, as in 0005.

It costs one thing: a new page also receives the changes of the dead
characters its controller drove. D11 binds what the stranger knows: its
snapshot names nothing from before its arrival (S4). The log is the
player's, and the player knows.

Rejected:

- The first draft: `Hello` names a character and counts its changes
  alone. It follows the 0005 line to the letter. It needs a new `Hello`,
  a count that the page restarts, a resume case for a dead character and
  a seventh slice. The second-model read found a loss: a connection cut
  between the arrival and the next turn resumes with the wrong pair.
- The server creates the stranger with `World::create_entity`. The
  creation would stay out of the replay inputs (issue #40), and the order
  of D9 would depend on the transport.
- Unit tests on `step` and `greeting` instead of a test entry. They are no
  `cargo xtask accept` target, and every proof over WebSocket would wait
  for S6.

## Multiplayer impact

- A death changes the character of one controller and of no other (D6).
  The world records the stranger of each dead character, so each
  controller finds its own.
- A character can kill a character (A3). The victim dies under the same
  rule, and its controller drives a stranger. This closes the gap that
  spec 0002 declared: a character at zero hit points was a violated
  invariant.
- Every stranger arrives by the same rule (D8). Two strangers never share
  a tile: the second finds the start tile held and takes the next one.
- A hostile creature next to several characters picks evenly among them
  (A13).
- Perceived changes are per character. A character's new hit points go to
  that character alone (A9, R18). Every other character perceives its
  condition (A10). Every character embodied at an arrival perceives it
  (R16); none perceives which controller drives the stranger.
- A controller receives only the changes of the characters it drove.
- The server sends changes to a connection at its `Hello` and in reply to
  its own command. Nothing is pushed. A controller whose character another
  controller's command strikes learns it at its next connection. The
  shared-world spec owns the push. In production a world hosts one
  character, so every strike on it happens inside its own command.
- In a shared world only the lineage waits (pillar 1). A stranger arrives
  at once, so nothing waits yet.

## Slices

Sequential. Inside a Rust slice, the first commit lays down the signatures
with a failing acceptance test. S3 changes the simulation alone, and S6
changes no message. Every other slice reaches the browser: the simulation,
the wire, the WASM codec and the web client. A protocol change ships the
server and the client together.

A unit test named below is no `cargo xtask accept` target: `cargo xtask
verify` runs it.

### - [x] S1 — A death leaves a corpse

- Files: `crates/ash-sim/src/{lib,world,combat,effect,perception}.rs`,
  `crates/ash-sim/src/death.rs` (new), `crates/ash-sim/tests/corpse.rs`
  (new), `crates/ash-protocol/src/message.rs`,
  `crates/ash-protocol/tests/decode_any_bytes.rs`,
  `crates/ash-server/src/perception.rs`,
  `crates/ash-server/tests/corpse_over_websocket.rs` (new),
  `crates/ash-server/tests/death_over_websocket.rs` and every test that
  builds or matches a snapshot, `crates/ash-wasm/src/lib.rs`,
  `web/src/{model,draw,protocol}.ts`, `web/tests/web_model.test.js`, the
  recorded web fixtures, `docs/plans/0003-ecs-brainstorm.md` (F6 table).
- Behavior:
  - The module `death` holds what a death changes. `combat::hit` asks it
    for the effects of a death (D1).
  - Every death leaves a `Corpse` on the tile where the entity died (D2).
    A corpse holds the identity of the entity that died, its tile, and
    what died: a character or a creature. It is not an entity: it has no
    id of its own, no scheduler entry and no controller.
  - The rule returns the corpse among the effects of the death. `apply`
    writes it (F4).
  - A corpse never blocks (D3). An entity can step onto its tile, and
    several corpses can lie on one tile.
  - The snapshot carries every corpse, in the order of the deaths: its
    tile, what died, and the dead entity when its viewer recognizes the
    corpse (D4). Until S4, every viewer recognizes every corpse.
  - The web draws a corpse under the entities. At a perceived death, the
    web model puts a corpse on the tile where it perceived the entity.
  - The death test of spec 0002 keeps its claim for entities: no later
    message shows the dead creature as an entity or as acting.
- Invariants:
  - The corpses are saved world state (F6, D12): the F6 table gains the
    row. No effect removes or moves a corpse.
  - A corpse is never an occupant: no move, bump or attack reads it.
  - The snapshot lists corpses by an explicit key, the order of the
    deaths, never by the storage layout (ADR 0001, point 5).
- Perf budget: none.
- Acceptance:
  - `cargo xtask accept corpse` — with a fixed seed, the character kills
    the hostile creature. A corpse lies on the tile of the death and names
    the creature. A move onto that tile is a step. Two deaths on one tile
    leave two corpses, in the order of the deaths. The run replays
    identically.
  - `cargo xtask accept corpse_over_websocket` — the snapshot of a later
    connection holds the corpse: its tile, a creature, the dead id.
  - `cargo xtask accept decode_any_bytes` — the decoder rejects a
    malformed corpse.
  - `cargo xtask accept web_model` — a perceived death leaves a corpse in
    the model, and a snapshot replaces the corpses.

### - [x] S2 — An attacked neutral creature becomes hostile

- Files:
  `crates/ash-sim/src/{world,combat,action,effect,perception,controller}.rs`,
  `crates/ash-sim/tests/provoked.rs` (new),
  `crates/ash-protocol/src/message.rs`,
  `crates/ash-protocol/tests/decode_any_bytes.rs`,
  `crates/ash-server/src/perception.rs`,
  `crates/ash-server/tests/provoked_over_websocket.rs` (new),
  `crates/ash-wasm/src/lib.rs`, `web/src/{model,draw,protocol}.ts`,
  `web/tests/web_model.test.js`, the recorded web fixtures.
- Behavior:
  - A character's attack on a neutral creature makes it hostile, on a hit
    and on a miss (A4). An effect carries the new disposition, and `apply`
    writes it.
  - The change of disposition is a new kind of perceived change. It names
    the creature and its new disposition. It comes after every other
    change of that attack (A11).
  - A creature that dies of the attack does not change disposition (A11).
  - A move into the creature is then an attack (A1). The snapshot shows
    the creature as hostile.
  - The web model updates the disposition at the change, and the log
    shows it.
- Invariants:
  - A disposition changes only from neutral to hostile (A4). An attack on
    a hostile creature adds no change of disposition.
  - A refused attack changes no disposition (R10).
  - The disposition is a property of the creature, the same for every
    character (D12).
- Perf budget: none.
- Acceptance:
  - `cargo xtask accept provoked` — with fixed seeds, a missed attack on
    the neutral creature is followed by its change of disposition. A hit
    is followed by the change of condition, then by the change of
    disposition. The next move into the creature is a hit or a miss. A
    second attack adds no change of disposition. An attack into an empty
    direction is refused: no disposition, RNG state or perceived change
    moves. The run replays identically.
  - A unit test builds a neutral creature that the next hit kills: the
    attack adds no change of disposition. No play reaches this case.
  - `cargo xtask accept provoked_over_websocket` — the change of
    disposition reaches the client, and a later snapshot shows the
    creature as hostile.
  - `cargo xtask accept decode_any_bytes` — the decoder rejects a
    malformed change of disposition.
  - `cargo xtask accept web_model` — the model holds the new disposition.

### - [x] S3 — A character dies and a stranger arrives, in the simulation

- Files: `crates/ash-sim/src/{world,combat,death,effect,map}.rs`,
  `crates/ash-sim/tests/character_death.rs` (new),
  `crates/ash-sim/tests/death.rs` (the test of the fourth hit on a
  character), `docs/plans/0003-ecs-brainstorm.md` (F6 table).
- Behavior:
  - A lethal hit on a character returns the effects of its death, like a
    creature's: the removal, the corpse and the perceived death (D1). The
    `invariant!` crash of `combat::hit` goes.
  - The same effects create the stranger: a character with a new id, 10
    hit points and speed 1 (D6, A7, R6). `resolve` returns the creation
    with no id; `apply` issues the id.
  - The stranger arrives on the tile where the world's first character
    started (D8). When an entity holds that tile, it takes the free floor
    tile the fewest moves away. Moves follow R4 and R11 and ignore every
    entity. Among equals, the first in row order wins: from the north,
    each row from the west.
  - `resolve` chooses the tile on the world without the dead character: a
    character that dies on the start tile leaves it free. `apply` chooses
    nothing (F4).
  - The stranger is due at the time of the death, after every entity
    already due then (D7, D9). The world awaits it at its turn (R2).
  - The world records the stranger of each dead character.
    `World::stranger_of` answers for an id the world issued: the stranger
    that follows a dead character, or none (F3, D6).
  - A stranger dies like any character, and another stranger arrives
    (D13).
- Invariants:
  - No embodied entity has zero hit points, whatever its controller (D1).
  - No effect embodies a dead id again (D14, F2).
  - The seed and the command log replay the stranger: it is no replay
    input (issue #40).
  - The stranger of a dead character is saved world state: the F6 table
    gains the row.
  - The awaited entity is always embodied.
  - No free floor tile within reach is an `invariant!` crash.
- Perf budget: none.
- Acceptance:
  - `cargo xtask accept character_death` — on a world with two
    characters, one strikes the other until it dies. The lethal hit is
    followed by the perceived death. A character corpse lies on its tile.
    A stranger stands on the start tile with a new id and 10 hit points.
  - The same target: a third entity is due at the time of the death. It
    acts before the stranger, and the world then awaits the stranger
    (D9).
  - The same target: the death changes nothing else. The clock, the
    other entities, their hit points, their dispositions and the earlier
    corpses stay (D12). The other character has no stranger.
  - The same target: setups put entities on the start tile (1, 1) and
    around it. The stranger takes (2, 1), then (1, 2), then (2, 2).
  - The same target: the stranger dies in turn, and a second stranger
    arrives. `World::stranger_of` answers each one. The corpse of the dead
    character says a character, in the snapshot of another character. The
    run replays identically from the same setup.
  - A unit test pins a distance that R11 lengthens: (6, 2) is six moves
    from (1, 1), not five.

### - [ ] S4 — The controller follows the stranger, which knows nothing of the past

- Files: `crates/ash-sim/src/{world,death,effect,perception}.rs`,
  `crates/ash-sim/tests/stranger.rs` (new),
  `crates/ash-protocol/src/message.rs`,
  `crates/ash-protocol/tests/decode_any_bytes.rs`,
  `crates/ash-server/src/{lib,registry,connection,perception}.rs`,
  `crates/ash-server/tests/character_death_over_websocket.rs` (new),
  `crates/ash-wasm/src/lib.rs`, `web/src/{model,draw,protocol}.ts`,
  `web/tests/web_model.test.js`, the recorded web fixtures.
- Behavior:
  - The creation of a stranger is perceived as its arrival, a new kind of
    perceived change (D10). It names the stranger and its tile, at the
    time of the death. One effect carries the creation and the arrival;
    `apply` writes both with the id it issues. The changes of a
    character's death come in this order: the hit, the death, the
    arrival.
  - The simulation gives the perceived changes of one character, in
    order: from its creation or arrival to its own death (D5, D11). The
    list of the whole world stays public, as test and replay evidence.
    The server reads the per-character changes only.
  - A character recognizes the corpse of an entity it perceived alive
    (D4): an entity that died after its creation or arrival (R16). Its
    snapshot names the dead entity of a recognized corpse only. Every
    corpse shows what died.
  - The server keeps the first character of each token
    (`HostedWorld::character`). It finds the character its controller
    drives now through `World::stranger_of`. Nothing in the server
    changes at a death.
  - The server sends a controller the perceived changes of each character
    it drove, in order. `Hello` keeps its form: `received` counts those
    changes.
  - A command during which the character dies is answered with: the
    changes through the death, the arrival and the later changes, the
    snapshot of the stranger, then its turn when the world awaits it. The
    greeting and the reply share one rule.
  - A new connection receives the snapshot of the character its
    controller drives now. A command that names a dead character is
    refused.
  - `serve_with_struck_character` is a test entry beside
    `serve_with_fault`. The world of its token holds two characters side
    by side and no creature. The second has no connection. Whenever the
    world awaits it, the server makes it attack toward the start tile of
    the first, or wait when that attack is not offered.
  - The web model adds the stranger at its arrival and takes the
    stranger's snapshot whole. The page controls the entity of its next
    turn. The log keeps every line: it shows the death and the arrival.
- Invariants:
  - A controller drives one embodied character at a time (D6, R20). No
    other controller's character changes.
  - After the death of the character it drives, the next change a
    controller receives is the arrival of its stranger (D10).
  - No perceived change holds an id that `apply` did not issue.
  - Recognition reads saved state: the order of the corpses and the
    stranger of each dead character. It never reads the perceived
    changes, which are external (F6). A character that no death created
    recognizes every corpse.
  - The stranger's snapshot names no entity that died before its arrival
    (D4, D11).
  - The test entry cannot reach production: `serve` never sets it, and no
    message can.
- Perf budget: none.
- Acceptance:
  - `cargo xtask accept stranger` — on a world with two characters, the
    arrival follows the death at the same time and names the stranger and
    its tile. The perceived changes of the stranger begin with its
    arrival. Those of the dead character end with its death. Those of the
    other character hold all three.
  - The same target: the stranger's snapshot shows the corpse of the dead
    character as a character, with no entity named. The other character's
    snapshot names it. A creature that dies after the arrival leaves a
    corpse the stranger recognizes.
  - `cargo xtask accept character_death_over_websocket` — on the test
    entry, the client waits until its character dies. One reply holds the
    lethal hit, the death, the arrival, the snapshot of the stranger and
    its turn, in that order. The snapshot gives the stranger 10 hit
    points of 10 and an unnamed character corpse. The next command moves
    the stranger.
  - The same target: a command that names the dead character is refused
    and changes nothing. A new connection with a count of zero receives
    every change from the first, then the snapshot of the stranger. The
    stranger dies in turn, and the client controls a second stranger.
  - The same target: after the two deaths, a connection at each count
    receives exactly the rest of the changes. The counts tested stand
    before each lethal hit, after each death and after each arrival. The
    corpse of the dead character crosses as a character.
  - `cargo xtask accept decode_any_bytes` — the decoder rejects a
    malformed arrival.
  - `cargo xtask accept web_model` — after the death of its character,
    the model shows the stranger, controls it and keeps the log.

### - [ ] S5 — A struck character perceives its own hit points

- Files: `crates/ash-sim/src/{combat,perception,world}.rs`,
  `crates/ash-sim/tests/own_hit_points.rs` (new),
  `crates/ash-protocol/src/message.rs`,
  `crates/ash-protocol/tests/decode_any_bytes.rs`,
  `crates/ash-server/src/perception.rs`,
  `crates/ash-server/tests/own_hit_points_over_websocket.rs` (new),
  `crates/ash-wasm/src/lib.rs`, `web/src/{model,draw,protocol}.ts`,
  `web/tests/web_model.test.js`.
- Behavior:
  - A hit that a character survives adds a perceived change of a new
    kind: its hit points, current and maximum (A9). It comes right after
    the hit, before any change of condition (A11).
  - Only the struck character perceives its hit points (A9, R18). Every
    other character perceives its change of condition, when the hit
    crosses a threshold (A10).
  - A character never perceives its own change of condition (A9).
  - A lethal hit adds no hit points: the death follows the hit (A11).
  - The web model replaces the numbers of its own character at the
    change. It never shows a condition for it.
- Invariants:
  - No message carries the hit points of an entity to the controller of
    another (A10, R18).
  - No message carries the condition of a character to its own
    controller (A9).
  - `Hello` still counts what its controller received: the changes its
    characters perceived.
- Perf budget: none.
- Acceptance:
  - `cargo xtask accept own_hit_points` — on a world with two characters,
    the struck character perceives each hit, then 7, 4 and 1 of 10. The
    striker perceives each hit, then wounded, badly wounded and dying,
    and no hit points. The lethal hit adds no hit points.
  - A unit test builds a character that a hit leaves in the same
    condition: it perceives its hit points, and no character perceives a
    change of condition.
  - `cargo xtask accept own_hit_points_over_websocket` — on the test
    entry, the replies to the client's waits hold each hit, then 7, 4 and
    1 of 10. No message names a condition for the client's own character.
  - The same target: the client strikes the second character once. It
    receives the condition of that character and never its hit points.
    A connection at each count then receives exactly the rest of the
    changes of its controller, through a death and an arrival.
  - `cargo xtask accept decode_any_bytes` — the decoder rejects malformed
    hit points.
  - `cargo xtask accept web_model` — the model shows 7 of 10 after the
    hit and no condition for its own character.
  - The pull request closes issue #42.

### - [ ] S6 — A hostile creature strikes

- Files:
  `crates/ash-sim/src/{lib,world,combat,controller,wander,perception}.rs`,
  `crates/ash-sim/src/aggression.rs` (new),
  `crates/ash-sim/tests/aggression.rs` (new),
  `crates/ash-server/src/perception.rs`,
  `crates/ash-server/tests/aggression_over_websocket.rs` (new),
  `web/tests/web_model.test.js`.
- Files that host the skeleton world, where the hostile creature now
  strikes: `crates/ash-sim/tests/{condition,death,hostility,corpse,provoked}.rs`,
  `crates/ash-server/tests/{attack,condition,creature,death,hostility,walk,corpse,provoked}_over_websocket.rs`,
  `crates/ash-server/tests/web_fixtures.rs` and the recorded web fixtures.
- Behavior:
  - Migration first, with every existing test green: the disposition is
    stored on the creature, and its controller derives from it (spec
    0002, amendment of S2; F6). A hostile creature still wanders.
  - The aggression is the controller of a hostile creature (A13). On its
    turn it attacks a character on an adjacent tile. One draw picks among
    the attack offers that target a character, clockwise from the north,
    even when there is one. With no such offer, it chooses as the wander
    does, with the same draw.
  - The wander is the controller of a neutral creature (A13). A provoked
    creature passes from the wander to the aggression with its
    disposition (A4).
  - A creature's attack is the attack of spec 0002: base cost 100 divided
    by its speed, one draw, a hit under 70, 3 hit points (A14, A5, A6).
  - A creature never attacks a creature, and its move into any entity
    stays a bump (A13).
  - The skeleton world keeps its three entities. Its hostile creature now
    strikes. Each existing fight test keeps its claim, on seeds where its
    character lives. It compares with the changes its character
    perceived.
- Invariants:
  - The offers stay the same for every controller (R3). The aggression
    chooses only among its offers (R14).
  - The aggression reads only what its creature perceives and its offers
    (ADR 0001, point 4).
  - An attack by a creature changes no disposition: a character has none
    (A4).
  - The live world and the command-log replay never diverge.
- Perf budget: none.
- Acceptance:
  - `cargo xtask accept aggression` — with fixed seeds, a hostile
    creature next to the character attacks on each of its turns: a hit
    and a miss, each costing it 50. A hit removes 3 hit points. Next to
    two characters, it strikes the one its draw designates: two seeds
    designate different characters.
  - The same target: with no character adjacent, a hostile creature next
    to a neutral creature never attacks it. Its turn is the wander's: the
    same action and the same RNG state. A neutral creature next to a
    character never attacks; once provoked, it does.
  - The same target: a hostile creature strikes on the diagonal across a
    wall corner (A3).
  - The same target: in the skeleton world, the character reaches zero
    hit points. Its corpse lies there, and a stranger arrives. The
    creature keeps its hit points and its disposition, and it strikes the
    stranger too (D12). The run replays identically.
  - `cargo xtask accept aggression_over_websocket` — in the skeleton
    world, the client keeps its character next to the hostile creature.
    A reply holds a hit by the creature, then the character's hit points.
    A later reply holds the lethal hit, the death, the arrival, the
    snapshot of the stranger and its turn. The next command moves the
    stranger.
  - `cargo xtask accept web_model` — the recorded fixtures of a fight
    replay in the model.
  - Milestone: the human plays the riposte and a death, then approves the
    glossary batch.

## Glossary proposals

Each definition restates the rule named beside it. The signatures commit
of each slice uses the code names. The human approves the rows into
`CONTEXT.md` at the milestone.

New entries:

| Term | Proposed definition | Source | Code name | Avoid |
|---|---|---|---|---|
| Stranger | The new character a controller drives after the character it drove died: a new identity, full hit points, the speed of a character. It knows nothing of the past. | D6, D11, pillar 1 | `stranger` in names, as `World::stranger_of` | respawn |
| Corpse | What a death leaves on the tile where the entity died. Not an entity: it never acts and never blocks. It names the entity that died. | D2, D3 | `Corpse`, `PerceivedCorpse` | body, remains |
| Arrival | The entry of a stranger into a world, at the time of the death it follows. Every character embodied then perceives it. It is the stranger's first perceived change. | D5, D7, D10, D11 | `ChangeKind::Arrival` | spawn |
| Aggression | A simulation controller, that of a hostile creature. It attacks a character on an adjacent tile and picks evenly among several. With none, it chooses as the wander does. | A13, A14 | `Controller::Aggression`, module `aggression` | |

Amended entries:

| Term | Proposed definition | Source | Code name | Avoid |
|---|---|---|---|---|
| Wander | A simulation controller, that of a neutral creature. It picks evenly among the actions the rules allow, except attacks. | A13 | `Controller::Wander`, module `wander` | |
| Disposition | Whether a creature is hostile or neutral. A character perceives which. A neutral creature that survives a character's attack becomes hostile and stays hostile. | A4, A11 | `Disposition` | |
| Death | The end of an entity's embodiment for good: the entity leaves the world and leaves a corpse. Today, at zero hit points. | D1, D2, ADR 0001 point 3 | `ChangeKind::Death`, module `death` | |
| Perceived change | One change a character perceives, stamped with its time and ordered. A character perceives changes from its creation or arrival to its own death. | R17, D5, D11 | `PerceivedChange` | |
| Snapshot | The whole state one character perceives. Sent when its controller connects, and when its controller starts to drive it. | D11, spec 0002 S4 | `Snapshot` | |

Choices behind the rows:

- "Riposte" names this spec and its brainstorm, not the controller. A
  creature created hostile strikes a character that never struck it, so
  the word would mislead as a term. The acceptance tests take the term:
  `aggression`.
- The second-model read proposes "hostile behavior" in place of
  "aggression". The `advisor` found no better term. The human kept
  "aggression" at the approval of this spec: the code names follow.
- "Pursuit" is a different concept, out of scope (attack design). It is
  no synonym, so it stays out of the Avoid column.
- "Body" is avoided for a corpse: body parts are a later design.
- The perceived changes of one character have no term of their own. The
  spec writes "the perceived changes of a character".
- The second character of the test entry is no term: it is a character
  that the server drives for the tests.

## Proposed amendments

- 2026-11-28, recorded (found in S1, review): the rank of a corpse is the
  count of the earlier corpses. It is the explicit key of the order of the
  deaths while no effect removes a corpse. An effect that removes a corpse
  needs a saved counter of deaths first.
- 2026-11-28, recorded (found in S1, bug injection): six bugs passed the
  S1 tests. A fix batch adds one test for each: the order of two deaths on
  two tiles, the order of the corpses on the wire, an entity tag out of
  range followed by its bytes, the color and the glyph of a drawn corpse,
  and the level of a corpse through the codec.
- 2026-11-28, accepted by the human (found in S1, bug injection): no test
  reaches the corpse of a character before S3. Three bugs pass: the corpse
  of a character says "creature" in the death rule, in the snapshot, or in
  the server mapping. This amends:
  - S3 Acceptance, `character_death`, gains: "The corpse of the dead
    character says a character, in the snapshot of another character."
  - S4 Acceptance, `character_death_over_websocket`, gains: "The corpse of
    the dead character crosses as a character."
- 2026-11-28, recorded (found in S2): the file list of S2 misses the tests
  that match every kind of perceived change. Nine existing test files gain
  the change of disposition in their matches:
  `crates/ash-sim/tests/{attack,creature,death,hostility}.rs`,
  `crates/ash-server/tests/{attack,condition,creature,death,hostility}_over_websocket.rs`,
  and the shared `crates/ash-server/tests/websocket/change.rs`. No claim
  of these tests changes.
- 2026-11-28, recorded (found in S2, review): a rule that provokes on a
  bump passed the `provoked` test. Its bump control now compares every
  change of the bump (A2).
- 2026-11-28, recorded (found in S2, review): the decoder accepts a change
  of disposition to neutral, which the simulation never makes. S2 asks to
  reject a malformed change, not an impossible one. The decoder rejects a
  change with no disposition.
- 2026-11-28, recorded (found in S3): the signatures commit removes the
  `invariant!` crash of `combat::hit`. With the crash, the acceptance test
  fails by a panic, not by an assertion. `World::stranger_of` answers none
  on that commit.
- 2026-11-28, recorded (found in S3): a stranger has speed 1, held by one
  constant of the module `death` (R6). The speed that `World::new` takes
  for the first character serves the tests alone. The start tile of D8 is
  the constant where `World::new` puts the first character: it is no saved
  state, and the F6 table gains no row for it.
- 2026-11-28, recorded (found in S3): no test reaches the crash of "no
  free floor tile within reach". On the skeleton map every floor tile is
  reachable, and the tile of the dead character is always free.
- 2026-11-28, recorded (found in S3, review and bug injection): nine bugs
  passed the S3 tests. A fix batch adds one test for each: the stranger
  scheduled after the turn of the striker, a stranger on the tile of a
  creature, a search from where the first character stands now, the
  corpse of a character on the tile of the stranger, a dead character that
  frees the start tile alone, the speed of the dead or of the striker,
  tiles ranked in a straight line, and a dead creature that gets a
  stranger.
- 2026-11-28, recorded (found in S3): one comment of
  `crates/ash-sim/src/perception.rs` says no character can arrive after a
  death. A stranger now can, and it still recognizes every corpse. S4
  rewrites that code and the comment with it.
