// The keys of the nine commands: the eight moves and wait (R4).

import type { Action } from "./protocol.js";

/** The command `key` (a `KeyboardEvent.key`) stands for, if any. */
export function commandForKey(key: string): Action | undefined {
  void key;
  return undefined;
}
