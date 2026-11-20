// What the client knows of the world: only what the server sent (R15). No
// drawing here, so a renderer can replace `draw.ts` without touching it.

import type {
  Action,
  AvailableAction,
  Position,
  ServerMessage,
  Terrain,
} from "./protocol.js";

/** A perceived change, stamped with the time it happened (R17). */
export interface LogEntry {
  time: number;
  entity: number;
  action: Action;
}

/** Why the server ended the connection for good. */
export type End = "quarantined" | "controller_taken";

export class PerceivedWorld {
  /** The clock: the latest time the server sent. */
  time = 0;
  /** Tiles per row of `terrain`; zero before the first snapshot. */
  width = 0;
  /** Level 0, row by row from the north, each row from the west. */
  terrain: Terrain[] = [];
  readonly positions = new Map<number, Position>();
  /** The entity this client controls, once the server awaits it. */
  controlled: number | undefined;
  /** The actions offered to the controlled entity; empty when not awaited. */
  offer: AvailableAction[] = [];
  readonly log: LogEntry[] = [];
  /** The last command was refused; nothing advanced (R10). */
  refused = false;
  ended: End | undefined;

  apply(message: ServerMessage): void {
    void message;
  }

  get height(): number {
    return 0;
  }

  terrainAt(x: number, y: number): Terrain | undefined {
    void x;
    void y;
    return undefined;
  }

  /** The cost the controlled entity would pay for `action`, if offered. */
  costOf(action: Action): number | undefined {
    void action;
    return undefined;
  }

  /** False once the server ended the connection for good. */
  mayReconnect(): boolean {
    return true;
  }
}
