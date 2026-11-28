// Acceptance of spec 0001, slice S6, and of the web part of spec 0002,
// slices S1 to S4, and of spec 0003, slices S1 and S2: the perceived-world
// model of the web client, fed through the real WASM codec. `cargo xtask accept
// web_model` builds the client, then runs this file with Node's test runner.
//
// Each fixture holds the binary messages a real server sent, in hex. The
// client said Hello, then commanded entity 0, which starts at (1, 1). The
// hostile creature, entity 1, starts at (7, 4); the neutral creature,
// entity 2, starts at (1, 4). Both wander.
// `crates/ash-server/tests/web_fixtures.rs` records each session again and
// fails when the server no longer sends these bytes.
//
// `fixtures/walk.json`: move east, move north (into the wall: refused),
// wait, move southeast. A second connection with the same token then took
// the controller.
//
// `fixtures/bump.json`: entity 0 moved southeast twice, then northwest
// into the neutral creature: a bump (R13, A2).
//
// `fixtures/attack.json`: the same world as `bump.json`. Entity 0 moved
// southeast twice, then attacked the neutral creature twice: northwest,
// then west (A3). The first attack made it hostile (A4).
//
// `fixtures/death.json`: another world. Entity 0 moved east, then
// southeast, then attacked the hostile creature to the southeast twice.
// The first hit left it wounded (A10), the second killed it (A8). Entity 0
// then moved southeast, onto the tile the creature left.
//
//   012345678
// 0 #########
// 1 #.......#
// 2 #....#..#
// 3 #.....#.#
// 4 #.......#
// 5 #########

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import { draw, drawMap, entityLine, logLine } from "../build/draw.js";
import { Keyboard } from "../build/keys.js";
import { PerceivedWorld } from "../build/model.js";
import { decodeServerMessage, encodeCommand, loadCodec } from "../build/protocol.js";

await loadCodec(readFileSync(new URL("../pkg/ash_wasm_bg.wasm", import.meta.url)));

/** The messages `name` holds, as bytes. */
function fixture(name) {
  return JSON.parse(readFileSync(new URL(`fixtures/${name}`, import.meta.url), "utf8")).map(
    (hex) => Uint8Array.from(Buffer.from(hex, "hex")),
  );
}

const walk = fixture("walk.json");
const bump = fixture("bump.json");
const attack = fixture("attack.json");
const death = fixture("death.json");
const controllerTaken = walk.length - 1;

/** The index of the first message of `stream` that `found` accepts. */
function indexOf(stream, found) {
  const index = stream.findIndex((bytes) => found(decodeServerMessage(bytes)));
  assert.notEqual(index, -1, "no such message");
  return index;
}

/** A model fed the messages `stream` holds, each decoded by the codec. */
function replay(stream) {
  const world = new PerceivedWorld();
  for (const bytes of stream) {
    const message = decodeServerMessage(bytes);
    assert.ok(message, `undecodable: ${Buffer.from(bytes).toString("hex")}`);
    world.apply(message);
  }
  return world;
}

/** A snapshot at time 0 of a floor of 5 by 5 tiles holding `entities` and `corpses`. */
const snapshot = (entities, corpses = []) => ({
  kind: "snapshot",
  time: 0,
  width: 5,
  terrain: Array(25).fill("floor"),
  entities,
  corpses,
});
/** The corpse at (`x`, `y`) of what died, naming `entity` or no entity (`null`). */
const corpse = (x, y, died, entity) => ({ position: { x, y, z: 0 }, died, entity });
/** The change the death of `entity` at time 0 is. */
const deathOf = (entity) => ({ kind: "perceived", change: { kind: "death", time: 0, entity } });
const hitPoints = (current, max) => ({ kind: "hit_points", current, max });
const inCondition = (condition) => ({ kind: "condition", condition });
const perceivedEntity = (entity, x, y, disposition, health = inCondition("unhurt")) => ({
  entity,
  position: { x, y, z: 0 },
  disposition,
  health,
});

/** The change an act is: at `time`, `entity` did `outcome`. */
const act = (time, entity, outcome) => ({ kind: "act", time, entity, outcome });
/** The outcome of the act `message` carries; `undefined` for any other message. */
const outcomeOf = (message) =>
  message.kind === "perceived" && message.change.kind === "act"
    ? message.change.outcome.kind
    : undefined;
const isDeath = (message) => message.kind === "perceived" && message.change.kind === "death";
const isConditionChange = (message) =>
  message.kind === "perceived" && message.change.kind === "condition";
const isDispositionChange = (message) =>
  message.kind === "perceived" && message.change.kind === "disposition";
/** The disposition `world` holds for each entity. */
const dispositions = (world) =>
  [...world.entities.values()].map(({ entity, disposition }) => [entity, disposition]);

/** The hit points or the condition of each entity `world` holds. */
const healths = (world) => [...world.entities.values()].map(({ entity, health }) => [entity, health]);

/** Where each entity `world` holds stands. */
const positions = (world) =>
  [...world.entities.values()].map(({ entity, position }) => [entity, position]);

const wait = { kind: "wait" };
const move = (direction) => ({ kind: "move", direction });
const strike = (direction) => ({ kind: "attack", direction });

test("the recorded walk gives the map, positions, clock, costs and log", () => {
  const world = replay(walk.slice(0, controllerTaken));

  assert.equal(world.width, 9);
  assert.equal(world.height, 6);
  assert.equal(world.terrainAt(0, 0), "wall");
  assert.equal(world.terrainAt(1, 1), "floor");
  assert.equal(world.terrainAt(5, 2), "wall");
  assert.equal(world.terrainAt(6, 3), "wall");
  assert.equal(world.terrainAt(9, 0), undefined);

  assert.equal(world.controlled, 0);
  // R16: each creature is one more perceived entity, stepped like the
  // character: entity 1 west, west, east from (7, 4), entity 2 north,
  // northeast, southeast from (1, 4).
  assert.deepEqual(positions(world), [
    [0, { x: 3, y: 2, z: 0 }],
    [1, { x: 6, y: 4, z: 0 }],
    [2, { x: 3, y: 3, z: 0 }],
  ]);
  assert.equal(world.time, 250);

  assert.equal(
    world.offers.length,
    10,
    "every move is open at (3, 2), wait, and an attack on the creature at (3, 3)",
  );
  assert.equal(world.costOf(wait), 50);
  assert.equal(world.costOf(move("northwest")), 100);

  const stepped = (time, entity, direction) => act(time, entity, { kind: "stepped", direction });
  const waited = (time, entity) => act(time, entity, { kind: "waited" });
  assert.deepEqual(world.log, [
    stepped(0, 0, "east"),
    stepped(0, 1, "west"),
    stepped(0, 2, "north"),
    waited(50, 1),
    waited(50, 2),
    waited(75, 1),
    stepped(75, 2, "northeast"),
    waited(100, 0),
    waited(100, 1),
    waited(125, 2),
    stepped(125, 1, "west"),
    stepped(150, 0, "southeast"),
    stepped(150, 2, "southeast"),
    stepped(175, 1, "east"),
    waited(200, 2),
    waited(225, 1),
    // A creature's move into the character is a bump (A13).
    act(225, 2, { kind: "bumped", target: 0 }),
  ]);
  assert.equal(world.refused, false);
  assert.equal(world.mayReconnect(), true);
});

test("the snapshot gives each creature its disposition, and the character none", () => {
  // A4: entity 1 is hostile, entity 2 is neutral.
  const world = replay(walk.slice(0, 1));
  assert.deepEqual(
    [...world.entities.values()].map(({ entity, disposition }) => [entity, disposition]),
    [
      [0, null],
      [1, "hostile"],
      [2, "neutral"],
    ],
  );
});

test("the snapshot gives the character its own hit points, and each creature its condition", () => {
  // A9, A10: numbers for entity 0, which the client controls; a kind for
  // the others.
  assert.deepEqual(healths(replay(walk.slice(0, 1))), [
    [0, hitPoints(10, 10)],
    [1, inCondition("unhurt")],
    [2, inCondition("unhurt")],
  ]);
});

test("hit points decode as the current ones, then the most", () => {
  // A snapshot is tag 0, the time, a width of 1 and one floor tile, then
  // two entities: an id, a position, a disposition, then hit points (tag
  // 0, two numbers) or a condition (tag 1, its own tag).
  const u32 = (value) => [value, 0, 0, 0];
  const entity = (id, health) => [...u32(id), ...u32(0), ...u32(0), ...u32(0), 0, ...health];
  const bytes = Uint8Array.of(
    ...[0, 0, 0, 0, 0, 0, 0, 0, 0],
    ...u32(1),
    ...u32(1),
    0,
    ...u32(2),
    ...entity(4, [0, ...u32(7), ...u32(10)]),
    ...entity(5, [1, 2]),
    // No corpse.
    ...u32(0),
  );
  const message = decodeServerMessage(bytes);
  assert.ok(message, "the snapshot does not decode");
  assert.deepEqual(
    message.entities.map(({ health }) => health),
    [hitPoints(7, 10), inCondition("badly_wounded")],
  );
});

test("a later snapshot replaces every entity of the one before", () => {
  // Each reconnection sends a new snapshot. In the later one, entity 1 is
  // gone, entity 2 stands elsewhere and entity 3 is new.
  const first = [
    perceivedEntity(0, 1, 1, null, hitPoints(10, 10)),
    perceivedEntity(1, 3, 1, "hostile"),
    perceivedEntity(2, 1, 3, "neutral", inCondition("dying")),
  ];
  const later = [
    perceivedEntity(0, 1, 1, null, hitPoints(7, 10)),
    perceivedEntity(2, 2, 3, "neutral", inCondition("wounded")),
    perceivedEntity(3, 4, 4, "hostile"),
  ];
  const world = new PerceivedWorld();
  world.apply(snapshot(first));
  world.apply(snapshot(later));

  assert.deepEqual([...world.entities.values()], later);
});

test("a later snapshot replaces the hit points of the controlled entity", () => {
  // The model learns which entity it controls once the server awaits it.
  const character = (health) => perceivedEntity(0, 1, 1, null, health);
  const world = new PerceivedWorld();
  world.apply(snapshot([character(hitPoints(10, 10))]));
  world.apply({ kind: "awaited", time: 0, entity: 0, actions: [] });
  world.apply(snapshot([character(hitPoints(7, 10))]));

  assert.deepEqual(healths(world), [[0, hitPoints(7, 10)]]);
});

test("the map draws the character, a hostile creature and a neutral creature apart, and no dead entity", () => {
  const world = replay(walk.slice(0, 2));
  // A canvas that keeps the fill style of each glyph drawn on it.
  const styles = [];
  const context = {
    fillRect() {},
    fillText() {
      styles.push(this.fillStyle);
    },
  };
  drawMap({ getContext: () => context }, world);

  assert.equal(styles.length, 3, "one glyph for each entity");
  assert.equal(new Set(styles).size, 3, `${styles}`);

  // A8, D2: the map drawn after a death holds no glyph for the dead entity,
  // and one for its corpse.
  world.apply(deathOf(1));
  styles.length = 0;
  drawMap({ getContext: () => context }, world);
  assert.equal(styles.length, 3, "one glyph for the corpse and for each living entity");
});

test("the map draws each corpse on its tile, under the entities", () => {
  // The character stands on the first corpse (D3). The second corpse lies
  // halfway between the two entities.
  const world = new PerceivedWorld();
  world.apply(
    snapshot(
      [perceivedEntity(0, 2, 1, null, hitPoints(10, 10)), perceivedEntity(3, 4, 3, "neutral")],
      [corpse(2, 1, "creature", 1), corpse(3, 2, "character", null)],
    ),
  );
  // The model learns it controls the character, which it then draws apart.
  world.apply({ kind: "awaited", time: 0, entity: 0, actions: [] });
  // A canvas that keeps the fill style of each tile, and the text, the point
  // and the fill style of each glyph drawn on it.
  const tiles = [];
  const glyphs = [];
  const context = {
    fillRect() {
      tiles.push(this.fillStyle);
    },
    fillText(text, x, y) {
      glyphs.push({ text, point: [x, y], style: this.fillStyle });
    },
  };
  drawMap({ getContext: () => context }, world);

  // The two corpses first, then the two entities over them.
  assert.equal(glyphs.length, 4);
  const [under, between, character, creature] = glyphs.map(({ point }) => point);
  assert.deepEqual(under, character);
  assert.deepEqual(between, [0, 1].map((axis) => (character[axis] + creature[axis]) / 2));
  // A corpse is no entity, nor terrain: it is drawn apart from all of them.
  const [first, second, ...entities] = glyphs.map(({ style }) => style);
  assert.equal(first, second);
  assert.ok(!entities.includes(first), `${first} is the style of an entity`);
  assert.ok(!tiles.includes(first), `${first} is the style of a tile`);
  const [firstText, secondText, ...entityTexts] = glyphs.map(({ text }) => text);
  assert.equal(firstText, secondText);
  assert.equal(new Set(entityTexts).size, 2, "the character and the creature share a glyph");
  assert.ok(!entityTexts.includes(firstText), `${firstText} is the glyph of an entity`);
});

test("a perceived death leaves a corpse on the tile where the model perceived the entity", () => {
  // The recorded death: the hostile creature, entity 1, died at (4, 3).
  const died = indexOf(death, isDeath);
  assert.deepEqual(replay(death.slice(0, died)).corpses, []);
  assert.deepEqual(replay(death).corpses, [corpse(4, 3, "creature", 1)]);

  // A character has no disposition: its corpse is a character's.
  const world = new PerceivedWorld();
  world.apply(
    snapshot(
      [
        perceivedEntity(0, 1, 1, null, hitPoints(10, 10)),
        perceivedEntity(5, 3, 2, "hostile"),
        perceivedEntity(7, 4, 4, null),
      ],
      [corpse(1, 1, "creature", 2)],
    ),
  );
  world.apply(deathOf(7));
  world.apply(deathOf(5));
  // An entity the model does not hold leaves no corpse it could place.
  world.apply(deathOf(9));

  // D2: in the order of the deaths, after the corpses of the snapshot.
  assert.deepEqual(world.corpses, [
    corpse(1, 1, "creature", 2),
    corpse(4, 4, "character", 7),
    corpse(3, 2, "creature", 5),
  ]);
});

test("a snapshot replaces the corpses", () => {
  const cast = [perceivedEntity(0, 1, 1, null), perceivedEntity(1, 2, 1, "hostile")];
  const world = new PerceivedWorld();
  world.apply(snapshot(cast, [corpse(3, 3, "creature", 4)]));
  world.apply(deathOf(1));
  const later = [corpse(2, 1, "creature", 1), corpse(4, 1, "character", null)];
  world.apply(snapshot(cast.slice(0, 1), later));

  assert.deepEqual(world.corpses, later);
});

test("a corpse decodes to its tile, what died and the entity it names", () => {
  // A snapshot is tag 0, the time, a width of 1 and one floor tile, no
  // entity, then two corpses: a tile, what died (tag 0 a character, 1 a
  // creature), then tag 0 for no entity, or tag 1 and the entity. The first
  // corpse lies on level 3.
  const u32 = (value) => [value, 0, 0, 0];
  const bytes = Uint8Array.of(
    ...[0, 0, 0, 0, 0, 0, 0, 0, 0],
    ...u32(1),
    ...u32(1),
    0,
    ...u32(0),
    ...u32(2),
    ...[...u32(3), ...u32(2), ...u32(3), 1, 1, ...u32(6)],
    ...[...u32(1), ...u32(4), ...u32(0), 0, 0],
  );
  const message = decodeServerMessage(bytes);
  assert.ok(message, "the snapshot does not decode");
  assert.deepEqual(message.corpses, [
    { ...corpse(3, 2, "creature", 6), position: { x: 3, y: 2, z: 3 } },
    corpse(1, 4, "character", null),
  ]);
});

test("a perceived step moves the mover toward its direction", () => {
  // x grows to the east, y to the south.
  const steps = {
    north: [2, 1],
    northeast: [3, 1],
    east: [3, 2],
    southeast: [3, 3],
    south: [2, 3],
    southwest: [1, 3],
    west: [1, 2],
    northwest: [1, 1],
  };
  for (const [direction, [x, y]] of Object.entries(steps)) {
    const world = new PerceivedWorld();
    world.apply(snapshot([perceivedEntity(4, 2, 2, null)]));
    world.apply({ kind: "perceived", change: act(0, 4, { kind: "stepped", direction }) });
    assert.deepEqual(positions(world), [[4, { x, y, z: 0 }]], direction);
  }
});

test("a bump leaves the mover in place and logs whom it bumped into", () => {
  const at = indexOf(bump, (message) => outcomeOf(message) === "bumped");
  const before = replay(bump.slice(0, at));
  const bumped = replay(bump.slice(0, at + 1));

  assert.deepEqual(bumped.entities, before.entities, "the bump moved an entity");
  assert.deepEqual(bumped.log.at(-1), act(200, 0, { kind: "bumped", target: 2 }));
  assert.deepEqual(positions(replay(bump)), [
    [0, { x: 3, y: 3, z: 0 }],
    [1, { x: 7, y: 4, z: 0 }],
    [2, { x: 1, y: 3, z: 0 }],
  ]);
});

test("an attack is offered toward the adjacent creature only, and logged as a hit or a miss", () => {
  const first = indexOf(attack, (message) => ["hit", "missed"].includes(outcomeOf(message)));
  const before = replay(attack.slice(0, first));
  // A3, A5: offered toward the creature at 100, and nowhere else.
  assert.equal(before.costOf(strike("northwest")), 100);
  assert.equal(before.costOf(strike("east")), undefined);
  assert.equal(before.costOf(move("northwest")), 100);

  // A11: the two steps, then a hit and a miss, each naming the creature
  // and nothing more: no odds (A12).
  assert.deepEqual(
    replay(attack).log.filter(({ entity }) => entity === 0),
    [
      act(0, 0, { kind: "stepped", direction: "southeast" }),
      act(100, 0, { kind: "stepped", direction: "southeast" }),
      act(200, 0, { kind: "hit", target: 2 }),
      act(300, 0, { kind: "missed", target: 2 }),
    ],
  );
  assert.deepEqual(replay(attack.slice(0, first + 1)).entities, before.entities);
});

test("a hit on the hostile creature is logged as a hit", () => {
  // A1, A11: the strike a move made is a hit like any other, whatever the
  // disposition of its target.
  const world = new PerceivedWorld();
  world.apply(snapshot([perceivedEntity(0, 1, 1, null), perceivedEntity(1, 2, 1, "hostile")]));
  const hit = act(0, 0, { kind: "hit", target: 1 });
  world.apply({ kind: "perceived", change: hit });

  assert.deepEqual(world.log, [hit]);
});

test("the log line of a strike says whether it hit or missed", () => {
  // The recorded world controls entity 0, drawn as "@".
  const world = replay(attack);
  const struck = (kind) => act(200, 0, { kind, target: 1 });
  assert.equal(logLine(world, struck("hit")), "200  @ hits entity 1");
  assert.equal(logLine(world, struck("missed")), "200  @ misses entity 1");
});

test("a death removes the dead creature, and no other entity, and is logged after its hit", () => {
  const died = indexOf(death, isDeath);
  const before = replay(death.slice(0, died));
  const after = replay(death.slice(0, died + 1));

  assert.deepEqual(positions(before), [
    [0, { x: 3, y: 2, z: 0 }],
    [1, { x: 4, y: 3, z: 0 }],
    [2, { x: 2, y: 2, z: 0 }],
  ]);
  // A8: the hostile creature, entity 1, left the world.
  assert.deepEqual(positions(after), [
    [0, { x: 3, y: 2, z: 0 }],
    [2, { x: 2, y: 2, z: 0 }],
  ]);
  // A11: the hit that killed it, then its death, at the same time.
  assert.deepEqual(after.log.slice(-2), [
    act(300, 0, { kind: "hit", target: 1 }),
    { kind: "death", time: 300, entity: 1 },
  ]);
});

test("a death removes the entity it names, whichever it is", () => {
  const world = new PerceivedWorld();
  world.apply(
    snapshot([
      perceivedEntity(0, 1, 1, null),
      perceivedEntity(3, 2, 1, "neutral"),
      perceivedEntity(5, 3, 1, "hostile"),
      perceivedEntity(7, 4, 1, "hostile"),
    ]),
  );
  const dies = (entity) => world.apply({ kind: "perceived", change: { kind: "death", time: 0, entity } });
  const held = () => [...world.entities.keys()];

  dies(5);
  assert.deepEqual(held(), [0, 3, 7]);
  dies(7);
  assert.deepEqual(held(), [0, 3]);
  // A3, A8: a neutral creature dies like a hostile one.
  dies(3);
  assert.deepEqual(held(), [0]);
});

test("a later snapshot keeps the log, the death included", () => {
  // R17: the client resumes with the count of the changes it holds. The
  // snapshot of a reconnection replaces the entities, never the log.
  const cast = [perceivedEntity(0, 1, 1, null), perceivedEntity(1, 2, 1, "hostile")];
  const changes = [act(100, 0, { kind: "hit", target: 1 }), { kind: "death", time: 100, entity: 1 }];
  const world = new PerceivedWorld();
  world.apply(snapshot(cast));
  for (const change of changes) {
    world.apply({ kind: "perceived", change });
  }
  world.apply(snapshot(cast.slice(0, 1)));

  assert.deepEqual(world.log, changes);
});

test("after the death the world goes on without the dead creature", () => {
  const world = replay(death);

  // The character stepped onto the tile the dead creature left (A8).
  assert.deepEqual(positions(world), [
    [0, { x: 4, y: 3, z: 0 }],
    [2, { x: 3, y: 3, z: 0 }],
  ]);
  assert.equal(world.time, 500);
  assert.equal(world.costOf(strike("west")), 100, "the neutral creature stands west");
  assert.equal(world.log.filter(({ kind }) => kind === "death").length, 1);
  // R17: the client resumes with the count of the changes it holds, so the
  // log holds every perceived change: a death is one.
  assert.equal(world.log.length, death.filter((bytes) => decodeServerMessage(bytes).kind === "perceived").length);
});

test("the log line of a death says who died", () => {
  const world = replay(death);
  assert.equal(logLine(world, { kind: "death", time: 300, entity: 1 }), "300  entity 1 dies");
});

test("a change of condition is logged right after its hit, and changes the condition of its entity alone", () => {
  const changed = indexOf(death, isConditionChange);
  const before = replay(death.slice(0, changed));
  const after = replay(death.slice(0, changed + 1));

  // A10, A11: the first hit left the hostile creature, entity 1, wounded.
  assert.deepEqual(after.log.slice(-2), [
    act(200, 0, { kind: "hit", target: 1 }),
    { kind: "condition", time: 200, entity: 1, condition: "wounded" },
  ]);
  assert.deepEqual(healths(before), [
    [0, hitPoints(10, 10)],
    [1, inCondition("unhurt")],
    [2, inCondition("unhurt")],
  ]);
  assert.deepEqual(healths(after), [
    [0, hitPoints(10, 10)],
    [1, inCondition("wounded")],
    [2, inCondition("unhurt")],
  ]);
  assert.deepEqual(positions(after), positions(before));
  // The lethal hit changed no condition: the death followed it.
  assert.equal(replay(death).log.filter(({ kind }) => kind === "condition").length, 1);
});

test("a change of condition that names an entity other than the character changes it to the condition it names", () => {
  const world = new PerceivedWorld();
  world.apply(
    snapshot([
      perceivedEntity(0, 1, 1, null, hitPoints(10, 10)),
      perceivedEntity(3, 2, 1, "neutral"),
      perceivedEntity(5, 3, 1, "hostile"),
    ]),
  );
  const comes = (entity, condition) =>
    world.apply({ kind: "perceived", change: { kind: "condition", time: 0, entity, condition } });
  const conditions = () => healths(world).slice(1);

  comes(5, "badly_wounded");
  assert.deepEqual(conditions(), [
    [3, inCondition("unhurt")],
    [5, inCondition("badly_wounded")],
  ]);
  comes(3, "wounded");
  comes(5, "dying");
  assert.deepEqual(conditions(), [
    [3, inCondition("wounded")],
    [5, inCondition("dying")],
  ]);
  // Its position and its disposition stay.
  assert.deepEqual(world.entities.get(5), perceivedEntity(5, 3, 1, "hostile", inCondition("dying")));
  // A change that names an entity the client does not hold adds none.
  comes(9, "wounded");
  assert.deepEqual([...world.entities.keys()], [0, 3, 5]);
});

test("the entity list shows the hit points of the character as numbers, and each condition beside its name", () => {
  // The recorded world controls entity 0, drawn as "@".
  const lines = (world) => [...world.entities.values()].map((perceived) => entityLine(world, perceived));
  const changed = indexOf(death, isConditionChange);

  // A9, A10.
  assert.deepEqual(lines(replay(death.slice(0, changed))), [
    "@: 10/10 hit points",
    "entity 1: unhurt",
    "entity 2: unhurt",
  ]);
  assert.deepEqual(lines(replay(death.slice(0, changed + 1))), [
    "@: 10/10 hit points",
    "entity 1: wounded",
    "entity 2: unhurt",
  ]);
  const world = replay(death);
  assert.equal(entityLine(world, perceivedEntity(4, 1, 1, null, hitPoints(7, 10))), "entity 4: 7/10 hit points");
  assert.equal(
    entityLine(world, perceivedEntity(4, 1, 1, "hostile", inCondition("badly_wounded"))),
    "entity 4: badly wounded",
  );
});

/**
 * The page elements `draw` fills for `world`, the attack armed or not: a list
 * keeps the text of its items.
 */
function drawn(world, attacking = false) {
  const list = () => ({
    lines: [],
    replaceChildren(...items) {
      this.lines = items.map((item) => item.textContent);
    },
  });
  const view = {
    map: { getContext: () => null },
    status: {},
    clock: {},
    entities: list(),
    actions: list(),
    log: list(),
  };
  globalThis.document = { createElement: () => ({}) };
  try {
    draw(view, world, true, attacking);
  } finally {
    delete globalThis.document;
  }
  return view;
}

test("the page lists each entity the world holds, with its line, and no dead entity", () => {
  const listed = (last) => drawn(replay(death.slice(0, last + 1))).entities.lines;

  assert.deepEqual(listed(indexOf(death, isConditionChange)), [
    "@: 10/10 hit points",
    "entity 1: wounded",
    "entity 2: unhurt",
  ]);
  // A8: the page drawn after the death lists the hostile creature no more.
  assert.deepEqual(listed(indexOf(death, isDeath)), ["@: 10/10 hit points", "entity 2: unhurt"]);
});

test("the page logs a change of condition above its hit", () => {
  // The log shows the newest change first.
  const world = replay(death.slice(0, indexOf(death, isConditionChange) + 1));
  assert.deepEqual(drawn(world).log.lines.slice(0, 2), [
    "200  entity 1 looks wounded",
    "200  @ hits entity 1",
  ]);
});

test("each condition decodes to its name", () => {
  // A change is tag 2, then kind 2 for a change of condition, the time and
  // the entity, little-endian, then the condition.
  const names = ["unhurt", "wounded", "badly_wounded", "dying"];
  names.forEach((condition, tag) => {
    const bytes = Uint8Array.of(2, 2, 44, 1, 0, 0, 0, 0, 0, 0, 7, 0, 0, 0, tag);
    assert.deepEqual(decodeServerMessage(bytes), {
      kind: "perceived",
      change: { kind: "condition", time: 300, entity: 7, condition },
    });
  });
});

test("the log line of a change of condition says how the entity looks", () => {
  const world = replay(death);
  const looks = (condition) => logLine(world, { kind: "condition", time: 200, entity: 2, condition });
  assert.equal(looks("wounded"), "200  entity 2 looks wounded");
  assert.equal(looks("badly_wounded"), "200  entity 2 looks badly wounded");
  assert.equal(looks("dying"), "200  entity 2 looks dying");
});

test("the recorded attack makes the neutral creature hostile, right after the hit and its condition", () => {
  const changed = indexOf(attack, isDispositionChange);
  const before = replay(attack.slice(0, changed));
  const after = replay(attack.slice(0, changed + 1));

  // A4: entity 2 was neutral, and stays hostile to the end.
  assert.deepEqual(dispositions(before), [
    [0, null],
    [1, "hostile"],
    [2, "neutral"],
  ]);
  assert.deepEqual(dispositions(after), [
    [0, null],
    [1, "hostile"],
    [2, "hostile"],
  ]);
  assert.deepEqual(dispositions(replay(attack)), dispositions(after));
  // A11: the hit, its change of condition, then the change of disposition.
  assert.deepEqual(after.log.slice(-3), [
    act(200, 0, { kind: "hit", target: 2 }),
    { kind: "condition", time: 200, entity: 2, condition: "wounded" },
    { kind: "disposition", time: 200, entity: 2, disposition: "hostile" },
  ]);
  // The second attack changed no disposition.
  assert.equal(replay(attack).log.filter(({ kind }) => kind === "disposition").length, 1);
});

test("a change of disposition changes the disposition of its creature alone", () => {
  const world = new PerceivedWorld();
  world.apply(
    snapshot([
      perceivedEntity(0, 1, 1, null, hitPoints(10, 10)),
      perceivedEntity(3, 2, 1, "neutral", inCondition("wounded")),
      perceivedEntity(5, 3, 1, "neutral"),
    ]),
  );
  const change = { kind: "disposition", time: 0, entity: 3, disposition: "hostile" };
  world.apply({ kind: "perceived", change });

  // Its position and its condition stay.
  assert.deepEqual(world.entities.get(3), perceivedEntity(3, 2, 1, "hostile", inCondition("wounded")));
  assert.deepEqual(dispositions(world), [
    [0, null],
    [3, "hostile"],
    [5, "neutral"],
  ]);
  assert.deepEqual(world.log, [change]);
  // A change that names an entity the client does not hold adds none.
  world.apply({ kind: "perceived", change: { ...change, entity: 9 } });
  assert.deepEqual([...world.entities.keys()], [0, 3, 5]);
});

test("a change of disposition decodes to the disposition its tag names", () => {
  // A change is tag 2, then kind 3 for a change of disposition, the time
  // and the entity, little-endian, then the tag of a disposition.
  const names = { 1: "hostile", 2: "neutral" };
  for (const [tag, disposition] of Object.entries(names)) {
    const bytes = Uint8Array.of(2, 3, 44, 1, 0, 0, 0, 0, 0, 0, 7, 0, 0, 0, Number(tag));
    assert.deepEqual(decodeServerMessage(bytes), {
      kind: "perceived",
      change: { kind: "disposition", time: 300, entity: 7, disposition },
    });
  }
});

test("the page logs a change of disposition as the creature turning hostile", () => {
  const world = replay(attack.slice(0, indexOf(attack, isDispositionChange) + 1));
  // The log shows the newest change first.
  assert.deepEqual(drawn(world).log.lines.slice(0, 3), [
    "200  entity 2 turns hostile",
    "200  entity 2 looks wounded",
    "200  @ hits entity 2",
  ]);
});

/** The text and the center of each glyph `drawMap` draws for `world`, with its fill style. */
function glyphs(world) {
  const drawnGlyphs = [];
  const context = {
    fillRect() {},
    fillText(text, x, y) {
      drawnGlyphs.push({ text, point: [x, y], style: this.fillStyle });
    },
  };
  drawMap({ getContext: () => context }, world);
  return drawnGlyphs;
}

test("after the death of its character, the model shows the stranger, controls it and keeps the log", () => {
  // The client controls entity 0, on the start tile (1, 1), with 1 hit
  // point left. Entity 1, a character east of it, strikes it.
  const world = new PerceivedWorld();
  world.apply(snapshot([perceivedEntity(0, 1, 1, null, hitPoints(1, 10)), perceivedEntity(1, 2, 1, null)]));
  world.apply({ kind: "awaited", time: 0, entity: 0, actions: [{ action: wait, cost: 50 }] });
  // D10: the reply to its wait holds the lethal hit, the death, then the
  // arrival of the stranger, entity 2, on the start tile.
  const changes = [
    act(0, 0, { kind: "waited" }),
    act(50, 1, { kind: "hit", target: 0 }),
    { kind: "death", time: 50, entity: 0 },
    { kind: "arrival", time: 50, entity: 2, position: { x: 1, y: 1, z: 0 } },
  ];
  for (const change of changes) {
    world.apply({ kind: "perceived", change });
  }

  // D6: the model adds the stranger at its arrival, an unhurt character,
  // over the corpse of the dead one.
  assert.deepEqual([...world.entities.values()], [perceivedEntity(1, 2, 1, null), perceivedEntity(2, 1, 1, null)]);
  assert.deepEqual(world.corpses, [corpse(1, 1, "character", 0)]);

  // D11: the snapshot of the stranger, then its turn. The stranger does not
  // recognize the corpse.
  const seenByStranger = [perceivedEntity(1, 2, 1, null), perceivedEntity(2, 1, 1, null, hitPoints(10, 10))];
  const unrecognized = [corpse(1, 1, "character", null)];
  world.apply({ ...snapshot(seenByStranger, unrecognized), time: 50 });
  world.apply({
    kind: "awaited",
    time: 50,
    entity: 2,
    actions: [
      { action: wait, cost: 50 },
      { action: move("south"), cost: 100 },
    ],
  });

  assert.deepEqual([...world.entities.values()], seenByStranger);
  assert.deepEqual(world.corpses, unrecognized);
  assert.equal(world.controlled, 2);
  assert.equal(world.costOf(move("south")), 100);
  assert.equal(world.time, 50);
  // The log keeps every line, the newest first.
  assert.deepEqual(world.log, changes);
  const page = drawn(world);
  assert.deepEqual(page.log.lines, [
    "50  @ arrives",
    "50  entity 0 dies",
    "50  entity 1 hits entity 0",
    "0  entity 0 waits",
  ]);
  assert.deepEqual(page.entities.lines, ["entity 1: unhurt", "@: 10/10 hit points"]);
  // The map draws the corpse, then the other character, then the stranger
  // as the controlled entity, over the corpse.
  const [under, other, stranger] = glyphs(world);
  assert.deepEqual(
    [under.text, other.text, stranger.text],
    ["%", "e", "@"],
  );
  assert.deepEqual(stranger.point, under.point);
  assert.notEqual(stranger.style, other.style);
});

test("an arrival decodes to the stranger and its tile", () => {
  // A change is tag 2, then kind 4 for an arrival, the time and the
  // entity, little-endian, then the tile: x, y and z.
  const u32 = (value) => [value, 0, 0, 0];
  const bytes = Uint8Array.of(2, 4, 44, 1, 0, 0, 0, 0, 0, 0, ...u32(7), ...u32(3), ...u32(2), ...u32(1));
  assert.deepEqual(decodeServerMessage(bytes), {
    kind: "perceived",
    change: { kind: "arrival", time: 300, entity: 7, position: { x: 3, y: 2, z: 1 } },
  });
});

test("a refused command advances nothing and keeps the offers", () => {
  const refusal = indexOf(walk, (message) => message.kind === "refused");
  const beforeRefusal = replay(walk.slice(0, refusal));
  const refused = replay(walk.slice(0, refusal + 1));

  assert.equal(refused.refused, true);
  assert.equal(refused.time, beforeRefusal.time);
  assert.deepEqual(refused.entities, beforeRefusal.entities);
  assert.deepEqual(refused.offers, beforeRefusal.offers);
  assert.equal(refused.costOf(move("north")), undefined, "north is a wall");

  assert.equal(replay(walk.slice(0, refusal + 2)).refused, false);
});

test("the offers leave once the controlled entity acts", () => {
  assert.deepEqual(replay(walk.slice(0, 3)).offers, []);
});

test("after ControllerTaken the client does not reconnect", () => {
  const world = replay(walk);

  assert.equal(world.ended, "controller_taken");
  assert.equal(world.mayReconnect(), false);
  assert.deepEqual(world.offers, []);
});

test("after Quarantined the client does not reconnect", () => {
  // A production server cannot fault, so this message is not recorded.
  const quarantined = Uint8Array.of(4);
  const world = replay([...walk.slice(0, 2), quarantined]);

  assert.equal(world.ended, "quarantined");
  assert.equal(world.mayReconnect(), false);
  assert.deepEqual(world.offers, []);
});

test("bytes that are not a message decode to nothing", () => {
  assert.equal(decodeServerMessage(Uint8Array.of(9)), undefined);
  assert.equal(decodeServerMessage(walk[0].subarray(1)), undefined);
});

/** The press of the key at `code` while `modifiers` are held. */
const keyPress = (code, modifiers = {}) => ({
  code,
  metaKey: false,
  ctrlKey: false,
  altKey: false,
  ...modifiers,
});

/**
 * The commands `keys` complete, pressed in order on one keyboard. A code
 * alone is that key pressed with no modifier.
 */
function press(keys) {
  const keyboard = new Keyboard();
  return keys.map((key) => keyboard.press(typeof key === "string" ? keyPress(key) : key));
}

test("the letter block and the numpad map to the eight moves and wait", () => {
  const layouts = [
    ["KeyQ", "KeyW", "KeyE", "KeyA", "KeyS", "KeyD", "KeyZ", "KeyX", "KeyC"],
    ["Numpad7", "Numpad8", "Numpad9", "Numpad4", "Numpad5", "Numpad6", "Numpad1", "Numpad2", "Numpad3"],
  ];
  const commands = [
    move("northwest"),
    move("north"),
    move("northeast"),
    move("west"),
    wait,
    move("east"),
    move("southwest"),
    move("south"),
    move("southeast"),
  ];
  for (const keys of layouts) {
    assert.deepEqual(press(keys), commands);
  }
  assert.deepEqual(press(["ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight", "Space"]), [
    move("north"),
    move("south"),
    move("west"),
    move("east"),
    wait,
  ]);
  assert.deepEqual(press(["KeyP"]), [undefined]);

  const encoded = [...commands, strike("east")].map((command) =>
    Buffer.from(encodeCommand(0, command)).toString("hex"),
  );
  assert.equal(new Set(encoded).size, 10, `the codec merges commands: ${encoded}`);
  assert.ok(encoded.every((hex) => hex.length > 0), `a command encodes to nothing: ${encoded}`);
});

test("a move and an attack encode to their own bytes", () => {
  // A command is tag 1, then the entity as a little-endian u32, then the
  // action: tag 1 for a move, tag 2 for an attack, then east, direction 2.
  const hex = (command) => Buffer.from(encodeCommand(7, command)).toString("hex");
  assert.equal(hex(move("east")), "01070000000102");
  assert.equal(hex(strike("east")), "01070000000202");
});

test("the attack key followed by a direction key attacks toward it", () => {
  assert.deepEqual(press(["KeyF", "KeyD"]), [undefined, strike("east")]);
  assert.deepEqual(press(["KeyF", "Numpad1"]), [undefined, strike("southwest")]);
  assert.deepEqual(press(["KeyF", "ArrowUp", "ArrowUp"]), [undefined, strike("north"), move("north")]);
  // Any other key cancels the attack, and is not a command of its own.
  assert.deepEqual(press(["KeyF", "Space", "KeyD"]), [undefined, undefined, move("east")]);
  assert.deepEqual(press(["KeyF", "KeyP", "KeyD"]), [undefined, undefined, move("east")]);
});

test("a key pressed with a modifier neither arms the attack nor completes a command", () => {
  // A3: Cmd+F and Ctrl+F are shortcuts of the browser, so the direction key
  // after them moves, and strikes nothing.
  for (const modifier of ["metaKey", "ctrlKey", "altKey"]) {
    const held = (code) => keyPress(code, { [modifier]: true });
    assert.deepEqual(press([held("KeyF"), "ArrowRight"]), [undefined, move("east")], modifier);
    assert.deepEqual(press([held("ArrowLeft")]), [undefined], modifier);
    // After the attack key it is any other key: it cancels the attack.
    assert.deepEqual(press(["KeyF", held("KeyD"), "KeyD"]), [undefined, undefined, move("east")], modifier);
  }
});

test("the status shows the armed attack until a key completes or cancels it", () => {
  const world = replay(walk.slice(0, 2));
  const keyboard = new Keyboard();
  const statusAfter = (code) => {
    keyboard.press(keyPress(code));
    return drawn(world, keyboard.attacking).status.textContent;
  };

  assert.match(statusAfter("KeyF"), /attack/i);
  assert.equal(statusAfter("Escape"), "");
  assert.match(statusAfter("KeyF"), /attack/i);
  assert.equal(statusAfter("KeyD"), "");

  // The end of the world outranks the armed attack.
  const ended = replay(walk);
  assert.equal(drawn(ended, true).status.textContent, drawn(ended).status.textContent);
});

test("the key help names the attack key, and that key arms the attack", () => {
  const page = readFileSync(new URL("../index.html", import.meta.url), "utf8");
  const help = page.match(/<p>.*?<\/p>/gs).find((paragraph) => /attack/i.test(paragraph));
  assert.ok(help, "the key help says nothing of the attack");
  const [, letter] = /<kbd>(\w)<\/kbd>/.exec(help) ?? [];
  assert.ok(letter, `the key help names no attack key: ${help}`);
  assert.deepEqual(press([`Key${letter}`, "KeyD"]), [undefined, strike("east")]);
});
