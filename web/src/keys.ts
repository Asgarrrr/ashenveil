// The keys of the commands: the eight moves, wait (R4), and the attack key
// followed by a direction (A3). Keys are matched by physical position
// (`KeyboardEvent.code`), so the letter block keeps its compass shape on
// AZERTY as on QWERTY.

import type { Action, Direction } from "./protocol.js";

const move = (direction: Direction): Action => ({ kind: "move", direction });
const wait: Action = { kind: "wait" };

/** The key that turns the next direction key into an attack toward it. */
const ATTACK = "KeyF";

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

/** Turns key presses into commands. */
export class Keyboard {
  /** The attack key was the last key pressed. */
  private attacking = false;

  /**
   * The command the press `key` completes, if any. After the attack key, a
   * direction key attacks toward its direction; any other key cancels.
   */
  press(key: Pick<KeyboardEvent, "code" | "metaKey" | "ctrlKey" | "altKey">): Action | undefined {
    if (key.code === ATTACK) {
      this.attacking = true;
      return undefined;
    }
    const command = COMMANDS.get(key.code);
    if (!this.attacking) {
      return command;
    }
    this.attacking = false;
    return command?.kind === "move" ? { kind: "attack", direction: command.direction } : undefined;
  }
}
