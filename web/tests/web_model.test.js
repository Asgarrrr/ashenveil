// Acceptance of spec 0001, slice S6, and of the web part of spec 0002,
// slices S1 and S2: the perceived-world model of the web client, fed
// through the real WASM codec. `cargo xtask accept web_model`
// builds the client, then runs this file with Node's test runner.
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
// then west (A3).
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

import { color, logLine } from "../build/draw.js";
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
  assert.deepEqual(
    [...world.positions],
    [
      [0, { x: 3, y: 2, z: 0 }],
      [1, { x: 6, y: 4, z: 0 }],
      [2, { x: 3, y: 3, z: 0 }],
    ],
  );
  assert.equal(world.time, 250);

  assert.equal(
    world.offer.length,
    10,
    "every move is open at (3, 2), wait, and an attack on the creature at (3, 3)",
  );
  assert.equal(world.costOf(wait), 50);
  assert.equal(world.costOf(move("northwest")), 100);

  const stepped = (time, entity, direction) => ({
    time,
    entity,
    outcome: { kind: "stepped", direction },
  });
  const waited = (time, entity) => ({ time, entity, outcome: { kind: "waited" } });
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
    { time: 225, entity: 2, outcome: { kind: "bumped", target: 0 } },
  ]);
  assert.equal(world.refused, false);
  assert.equal(world.mayReconnect(), true);
});

test("the snapshot gives each creature its disposition, and the character none", () => {
  // A4: entity 1 is hostile, entity 2 is neutral.
  const world = replay(walk.slice(0, 1));
  assert.deepEqual(
    [...world.dispositions],
    [
      [1, "hostile"],
      [2, "neutral"],
    ],
  );
});

test("the map draws the character, a hostile creature and a neutral creature apart", () => {
  const world = replay(walk.slice(0, 2));
  const colors = [0, 1, 2].map((entity) => color(world, entity));
  assert.equal(new Set(colors).size, 3, `${colors}`);
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
    world.apply({
      kind: "snapshot",
      time: 0,
      width: 5,
      terrain: Array(25).fill("floor"),
      entities: [{ entity: 4, position: { x: 2, y: 2, z: 0 }, disposition: null }],
    });
    world.apply({ kind: "perceived", time: 0, entity: 4, outcome: { kind: "stepped", direction } });
    assert.deepEqual(world.positions.get(4), { x, y, z: 0 }, direction);
  }
});

test("a bump leaves the mover in place and logs whom it bumped into", () => {
  const at = indexOf(
    bump,
    (message) => message.kind === "perceived" && message.outcome.kind === "bumped",
  );
  const before = replay(bump.slice(0, at));
  const bumped = replay(bump.slice(0, at + 1));

  assert.deepEqual(bumped.positions, before.positions, "the bump moved an entity");
  assert.deepEqual(bumped.log.at(-1), {
    time: 200,
    entity: 0,
    outcome: { kind: "bumped", target: 2 },
  });
  assert.deepEqual(
    [...replay(bump).positions],
    [
      [0, { x: 3, y: 3, z: 0 }],
      [1, { x: 7, y: 4, z: 0 }],
      [2, { x: 1, y: 3, z: 0 }],
    ],
  );
});

test("an attack is offered toward the adjacent creature only, and logged as a hit or a miss", () => {
  const first = indexOf(
    attack,
    (message) => message.kind === "perceived" && ["hit", "missed"].includes(message.outcome.kind),
  );
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
      { time: 0, entity: 0, outcome: { kind: "stepped", direction: "southeast" } },
      { time: 100, entity: 0, outcome: { kind: "stepped", direction: "southeast" } },
      { time: 200, entity: 0, outcome: { kind: "hit", target: 2 } },
      { time: 300, entity: 0, outcome: { kind: "missed", target: 2 } },
    ],
  );
  assert.equal(replay(attack.slice(0, first + 1)).positions.get(0).x, before.positions.get(0).x);
});

test("the log line of a strike says whether it hit or missed", () => {
  // The recorded world controls entity 0, drawn as "@".
  const world = replay(attack);
  const struck = (kind) => ({ time: 200, entity: 0, outcome: { kind, target: 1 } });
  assert.equal(logLine(world, struck("hit")), "200  @ hits entity 1");
  assert.equal(logLine(world, struck("missed")), "200  @ misses entity 1");
});

test("a refused command advances nothing and keeps the offer", () => {
  const refusal = indexOf(walk, (message) => message.kind === "refused");
  const beforeRefusal = replay(walk.slice(0, refusal));
  const refused = replay(walk.slice(0, refusal + 1));

  assert.equal(refused.refused, true);
  assert.equal(refused.time, beforeRefusal.time);
  assert.deepEqual(refused.positions, beforeRefusal.positions);
  assert.deepEqual(refused.offer, beforeRefusal.offer);
  assert.equal(refused.costOf(move("north")), undefined, "north is a wall");

  assert.equal(replay(walk.slice(0, refusal + 2)).refused, false);
});

test("the offer leaves once the controlled entity acts", () => {
  assert.deepEqual(replay(walk.slice(0, 3)).offer, []);
});

test("after ControllerTaken the client does not reconnect", () => {
  const world = replay(walk);

  assert.equal(world.ended, "controller_taken");
  assert.equal(world.mayReconnect(), false);
  assert.deepEqual(world.offer, []);
});

test("after Quarantined the client does not reconnect", () => {
  // A production server cannot fault, so this message is not recorded.
  const quarantined = Uint8Array.of(4);
  const world = replay([...walk.slice(0, 2), quarantined]);

  assert.equal(world.ended, "quarantined");
  assert.equal(world.mayReconnect(), false);
  assert.deepEqual(world.offer, []);
});

test("bytes that are not a message decode to nothing", () => {
  assert.equal(decodeServerMessage(Uint8Array.of(9)), undefined);
  assert.equal(decodeServerMessage(walk[0].subarray(1)), undefined);
});

/** The commands `keys` complete, pressed in order on one keyboard. */
function press(keys) {
  const keyboard = new Keyboard();
  return keys.map((code) => keyboard.press(code));
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
