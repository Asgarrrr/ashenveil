// The keys of the nine commands: the eight moves and wait (R4). The numpad
// and the vi keys lay them out as a compass around wait.

import type { Action } from "./protocol.js";

/** Row by row from the north-west: numpad key, vi key, command. */
const LAYOUT: [string, string, Action][] = [
  ["7", "y", { kind: "move", direction: "northwest" }],
  ["8", "k", { kind: "move", direction: "north" }],
  ["9", "u", { kind: "move", direction: "northeast" }],
  ["4", "h", { kind: "move", direction: "west" }],
  ["5", ".", { kind: "wait" }],
  ["6", "l", { kind: "move", direction: "east" }],
  ["1", "b", { kind: "move", direction: "southwest" }],
  ["2", "j", { kind: "move", direction: "south" }],
  ["3", "n", { kind: "move", direction: "southeast" }],
];

const COMMANDS = new Map<string, Action>(
  LAYOUT.flatMap(([numpad, vi, command]) => [
    [numpad, command],
    [vi, command],
  ]),
);

/** The command `key` (a `KeyboardEvent.key`) stands for, if any. */
export function commandForKey(key: string): Action | undefined {
  return COMMANDS.get(key);
}
