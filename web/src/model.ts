// What the client knows of the world: only what the server sent (R15). No
// drawing here, so a renderer can replace `draw.ts` without touching it.

import type {
  Action,
  AvailableAction,
  Direction,
  Outcome,
  Position,
  ServerMessage,
  Terrain,
} from "./protocol.js";

/** A perceived change, stamped with the time it happened (R17). */
export interface LogEntry {
  time: number;
  entity: number;
  outcome: Outcome;
}

/** Why the server ended the connection for good. */
export type End = "quarantined" | "controller_taken";

/**
 * One step toward each direction, as the change of x (east) and of y
 * (south). A perceived step carries no position: the client takes it.
 * A bump leaves the mover in place (R13). A test in `crates/ash-wasm/src/lib.rs`
 * holds this table to the server's offsets.
 */
const STEPS: Record<Direction, [number, number]> = {
  north: [0, -1],
  northeast: [1, -1],
  east: [1, 0],
  southeast: [1, 1],
  south: [0, 1],
  southwest: [-1, 1],
  west: [-1, 0],
  northwest: [-1, -1],
};

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
    this.refused = false;
    switch (message.kind) {
      case "snapshot":
        this.time = message.time;
        this.width = message.width;
        this.terrain = message.terrain;
        this.positions.clear();
        for (const { entity, position } of message.entities) {
          this.positions.set(entity, position);
        }
        break;
      case "awaited":
        this.time = message.time;
        this.controlled = message.entity;
        this.offer = message.actions;
        break;
      case "perceived":
        this.time = message.time;
        this.log.push({ time: message.time, entity: message.entity, outcome: message.outcome });
        this.perceive(message.entity, message.outcome);
        break;
      case "refused":
        this.refused = true;
        break;
      case "quarantined":
      case "controller_taken":
        this.ended = message.kind;
        this.offer = [];
        break;
    }
  }

  get height(): number {
    return this.width === 0 ? 0 : this.terrain.length / this.width;
  }

  terrainAt(x: number, y: number): Terrain | undefined {
    if (x < 0 || x >= this.width || y < 0) {
      return undefined;
    }
    return this.terrain[y * this.width + x];
  }

  /** The cost the controlled entity would pay for `action`, if offered. */
  costOf(action: Action): number | undefined {
    return this.offer.find((offered) => sameAction(offered.action, action))?.cost;
  }

  /** False once the server ended the connection for good. */
  mayReconnect(): boolean {
    return this.ended === undefined;
  }

  private perceive(entity: number, outcome: Outcome): void {
    if (entity === this.controlled) {
      this.offer = [];
    }
    const position = this.positions.get(entity);
    if (outcome.kind === "stepped" && position !== undefined) {
      const [east, south] = STEPS[outcome.direction];
      this.positions.set(entity, { ...position, x: position.x + east, y: position.y + south });
    }
  }
}

function sameAction(a: Action, b: Action): boolean {
  if (a.kind === "wait" || b.kind === "wait") {
    return a.kind === b.kind;
  }
  return a.kind === b.kind && a.direction === b.direction;
}
