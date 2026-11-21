// Acceptance of spec 0001, slice S6: the perceived-world model of the web
// client, fed through the real WASM codec. `cargo xtask accept web_model`
// builds the client, then runs this file with Node's test runner.
//
// Each fixture holds the binary messages a real server sent, in hex. The
// client said Hello, then commanded entity 0, which starts at (1, 1). The
// creature, entity 1, starts at (7, 4) and wanders.
//
// `fixtures/walk.json`: move east, move north (into the wall: refused),
// wait, move southeast. A second connection with the same token then took
// the controller.
//
// `fixtures/bump.json`: the client chased the creature until a move of
// entity 0 bumped into it (R13).
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

import { commandForKey } from "../build/keys.js";
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
  // R16: the creature is one more perceived entity, stepped like the
  // character: west, west, north, west from (7, 4).
  assert.deepEqual(
    [...world.positions],
    [
      [0, { x: 3, y: 2, z: 0 }],
      [1, { x: 4, y: 3, z: 0 }],
    ],
  );
  assert.equal(world.time, 250);

  assert.equal(world.offer.length, 9, "every move is open at (3, 2), and wait");
  assert.equal(world.costOf(wait), 50);
  assert.equal(world.costOf(move("northwest")), 100);

  const stepped = (time, entity, action) => ({ time, entity, action, bumped: null });
  assert.deepEqual(world.log, [
    stepped(0, 0, move("east")),
    stepped(0, 1, move("west")),
    stepped(50, 1, move("west")),
    stepped(100, 0, wait),
    stepped(100, 1, move("north")),
    stepped(150, 0, move("southeast")),
    stepped(150, 1, wait),
    stepped(175, 1, move("west")),
    stepped(225, 1, wait),
  ]);
  assert.equal(world.refused, false);
  assert.equal(world.mayReconnect(), true);
});

test("a perceived move steps the mover toward its direction", () => {
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
      entities: [{ entity: 4, position: { x: 2, y: 2, z: 0 } }],
    });
    world.apply({ kind: "perceived", time: 0, entity: 4, action: move(direction), bumped: null });
    assert.deepEqual(world.positions.get(4), { x, y, z: 0 }, direction);
  }
});

test("a bump leaves the mover in place and logs whom it bumped into", () => {
  const at = indexOf(bump, (message) => message.kind === "perceived" && message.bumped !== null);
  const before = replay(bump.slice(0, at));
  const bumped = replay(bump.slice(0, at + 1));

  assert.deepEqual(bumped.positions, before.positions, "the bump moved an entity");
  assert.deepEqual(bumped.log.at(-1), {
    time: 200,
    entity: 0,
    action: move("southeast"),
    bumped: 1,
  });
  assert.deepEqual(
    [...replay(bump).positions],
    [
      [0, { x: 3, y: 2, z: 0 }],
      [1, { x: 6, y: 4, z: 0 }],
    ],
  );
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

test("the numpad and vi keys map to the eight moves and wait", () => {
  const layouts = [
    ["7", "8", "9", "4", "5", "6", "1", "2", "3"],
    ["y", "k", "u", "h", ".", "l", "b", "j", "n"],
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
    assert.deepEqual(keys.map(commandForKey), commands);
  }
  assert.equal(commandForKey("q"), undefined);

  const encoded = commands.map((command) => Buffer.from(encodeCommand(0, command)).toString("hex"));
  assert.equal(new Set(encoded).size, 9, `the codec merges commands: ${encoded}`);
  assert.ok(encoded.every((hex) => hex.length > 0), `a command encodes to nothing: ${encoded}`);
});
