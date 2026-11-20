// Acceptance of spec 0001, slice S6: the perceived-world model of the web
// client, fed through the real WASM codec. `cargo xtask accept web_model`
// builds the client, then runs this file with Node's test runner.
//
// `fixtures/walk.json` holds the binary messages a real server sent, in hex.
// The client said Hello, then commanded entity 0, which starts at (1, 1):
// move east, move north (into the wall: refused), wait, move southeast.
// A second connection with the same token then took the controller.
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

const walk = JSON.parse(
  readFileSync(new URL("fixtures/walk.json", import.meta.url), "utf8"),
).map((hex) => Uint8Array.from(Buffer.from(hex, "hex")));
const controllerTaken = walk.length - 1;

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
  assert.deepEqual([...world.positions], [[0, { x: 3, y: 2, z: 0 }]]);
  assert.equal(world.time, 250);

  assert.equal(world.offer.length, 9, "every move is open at (3, 2), and wait");
  assert.equal(world.costOf(wait), 50);
  assert.equal(world.costOf(move("northwest")), 100);

  assert.deepEqual(world.log, [
    { time: 0, entity: 0, action: move("east") },
    { time: 100, entity: 0, action: wait },
    { time: 150, entity: 0, action: move("southeast") },
  ]);
  assert.equal(world.refused, false);
  assert.equal(world.mayReconnect(), true);
});

test("a refused command advances nothing and keeps the offer", () => {
  const beforeRefusal = replay(walk.slice(0, 4));
  const refused = replay(walk.slice(0, 5));

  assert.equal(refused.refused, true);
  assert.equal(refused.time, beforeRefusal.time);
  assert.deepEqual(refused.positions, beforeRefusal.positions);
  assert.deepEqual(refused.offer, beforeRefusal.offer);
  assert.equal(refused.costOf(move("north")), undefined, "north is a wall");

  assert.equal(replay(walk.slice(0, 6)).refused, false);
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
