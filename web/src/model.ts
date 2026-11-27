// What the client knows of the world: only what the server sent (R15). No
// drawing here, so a renderer can replace `draw.ts` without touching it.

import type {
  Action,
  Offer,
  Direction,
  PerceivedChange,
  PerceivedEntity,
  ServerMessage,
  Terrain,
} from "./protocol.js";

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
  /** What is perceived of each entity, by entity (R16, A4). */
  readonly entities = new Map<number, PerceivedEntity>();
  /** The entity this client controls, once the server awaits it. */
  controlled: number | undefined;
  /** The offers of the controlled entity; empty when not awaited. */
  offers: Offer[] = [];
  /** Every perceived change received, in order (R17). */
  readonly log: PerceivedChange[] = [];
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
        this.entities.clear();
        for (const perceived of message.entities) {
          this.entities.set(perceived.entity, perceived);
        }
        break;
      case "awaited":
        this.time = message.time;
        this.controlled = message.entity;
        this.offers = message.actions;
        break;
      case "perceived":
        this.time = message.change.time;
        this.log.push(message.change);
        this.perceive(message.change);
        break;
      case "refused":
        this.refused = true;
        break;
      case "quarantined":
      case "controller_taken":
        this.ended = message.kind;
        this.offers = [];
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
    return this.offers.find((offer) => sameAction(offer.action, action))?.cost;
  }

  /** False once the server ended the connection for good. */
  mayReconnect(): boolean {
    return this.ended === undefined;
  }

  private perceive(change: PerceivedChange): void {
    if (change.kind === "death") {
      // A8: the dead entity left the world.
      this.entities.delete(change.entity);
      return;
    }
    const { entity } = change;
    const perceived = this.entities.get(entity);
    if (change.kind === "condition") {
      // A10: the hit left the entity in this condition.
      if (perceived !== undefined) {
        const { condition } = change;
        this.entities.set(entity, { ...perceived, health: { kind: "condition", condition } });
      }
      return;
    }
    const { outcome } = change;
    if (entity === this.controlled) {
      this.offers = [];
    }
    if (outcome.kind === "stepped" && perceived !== undefined) {
      const [east, south] = STEPS[outcome.direction];
      const { x, y, z } = perceived.position;
      this.entities.set(entity, { ...perceived, position: { x: x + east, y: y + south, z } });
    }
  }
}

function sameAction(a: Action, b: Action): boolean {
  if (a.kind === "wait" || b.kind === "wait") {
    return a.kind === b.kind;
  }
  return a.kind === b.kind && a.direction === b.direction;
}
