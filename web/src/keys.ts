// The keys of the nine commands: the eight moves and wait (R4). Keys are
// matched by physical position (`KeyboardEvent.code`), so the letter block
// keeps its compass shape on AZERTY as on QWERTY.

import type { Action, Direction } from "./protocol.js";

const move = (direction: Direction): Action => ({ kind: "move", direction });
const wait: Action = { kind: "wait" };

/** Row by row from the north-west: letter block, numpad, command. */
const COMPASS: [string, string, Action][] = [
  ["KeyQ", "Numpad7", move("northwest")],
  ["KeyW", "Numpad8", move("north")],
  ["KeyE", "Numpad9", move("northeast")],
  ["KeyA", "Numpad4", move("west")],
  ["KeyS", "Numpad5", wait],
  ["KeyD", "Numpad6", move("east")],
  ["KeyZ", "Numpad1", move("southwest")],
  ["KeyX", "Numpad2", move("south")],
  ["KeyC", "Numpad3", move("southeast")],
];

const COMMANDS = new Map<string, Action>([
  ...COMPASS.flatMap(([letter, numpad, command]): [string, Action][] => [
    [letter, command],
    [numpad, command],
  ]),
  ["ArrowUp", move("north")],
  ["ArrowDown", move("south")],
  ["ArrowLeft", move("west")],
  ["ArrowRight", move("east")],
  ["Space", wait],
]);

/** The command the key at `code` (a `KeyboardEvent.code`) stands for, if any. */
export function commandForKey(code: string): Action | undefined {
  return COMMANDS.get(code);
}
