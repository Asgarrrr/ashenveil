// Throwaway Canvas 2D drawing of the perceived world. It reads the model and
// never changes it; the real renderer will replace this file.

import type { PerceivedWorld } from "./model.js";
import type { Action, Condition, Outcome, PerceivedChange, PerceivedEntity } from "./protocol.js";

const TILE = 32;

/** The page elements the drawing fills. */
export interface View {
  map: HTMLCanvasElement;
  status: HTMLElement;
  clock: HTMLElement;
  entities: HTMLElement;
  actions: HTMLElement;
  log: HTMLElement;
}

/** `attacking`: the attack key is armed, and the next direction key strikes. */
export function draw(
  view: View,
  world: PerceivedWorld,
  connected: boolean,
  attacking: boolean,
): void {
  drawMap(view.map, world);
  view.status.textContent = status(world, connected, attacking);
  view.clock.textContent = String(world.time);
  view.entities.replaceChildren(
    ...[...world.entities.values()].map((perceived) => item(entityLine(world, perceived))),
  );
  view.actions.replaceChildren(
    ...world.offers.map(({ action, cost }) => item(`${describe(action)}: ${cost}`)),
  );
  view.log.replaceChildren(
    ...world.log
      .slice(-20)
      .reverse()
      .map((change) => item(logLine(world, change))),
  );
}

/**
 * The color of `entity` on the map. A hostile creature stands apart from a
 * neutral one: a move into it is an attack (A1, A4).
 */
function color(world: PerceivedWorld, entity: number): string {
  if (entity === world.controlled) {
    return "#fc3";
  }
  return world.entities.get(entity)?.disposition === "hostile" ? "#f55" : "#6cf";
}

/**
 * The line the entity list shows for `perceived`: its name, then its hit
 * points as numbers (A9) or its condition (A10).
 */
export function entityLine(world: PerceivedWorld, perceived: PerceivedEntity): string {
  const { entity, health } = perceived;
  const shown =
    health.kind === "hit_points"
      ? `${health.current}/${health.max} hit points`
      : words(health.condition);
  return `${name(world, entity)}: ${shown}`;
}

/** The line the log shows for `change`: when, who, and what it did. */
export function logLine(world: PerceivedWorld, change: PerceivedChange): string {
  const who = `${change.time}  ${name(world, change.entity)}`;
  switch (change.kind) {
    case "act":
      return `${who} ${happened(world, change.outcome)}`;
    case "death":
      return `${who} dies`;
    case "condition":
      return `${who} looks ${words(change.condition)}`;
  }
}

/** The color of a corpse: apart from every entity, which it never is (D2). */
const CORPSE_COLOR = "#a87";

/**
 * The terrain, then one glyph for each corpse, then one for each entity, in
 * its color. An entity standing on a corpse hides it (D3).
 */
export function drawMap(canvas: HTMLCanvasElement, world: PerceivedWorld): void {
  canvas.width = world.width * TILE;
  canvas.height = world.height * TILE;
  const context = canvas.getContext("2d");
  if (context === null) {
    return;
  }
  for (let y = 0; y < world.height; y++) {
    for (let x = 0; x < world.width; x++) {
      context.fillStyle = world.terrainAt(x, y) === "wall" ? "#555" : "#1a1a1a";
      context.fillRect(x * TILE, y * TILE, TILE, TILE);
    }
  }
  context.font = `${TILE - 6}px monospace`;
  context.textAlign = "center";
  context.textBaseline = "middle";
  context.fillStyle = CORPSE_COLOR;
  for (const { position: { x, y } } of world.corpses) {
    context.fillText("%", (x + 0.5) * TILE, (y + 0.5) * TILE);
  }
  for (const [entity, { position: { x, y } }] of world.entities) {
    context.fillStyle = color(world, entity);
    context.fillText(entity === world.controlled ? "@" : "e", (x + 0.5) * TILE, (y + 0.5) * TILE);
  }
}

function status(world: PerceivedWorld, connected: boolean, attacking: boolean): string {
  switch (world.ended) {
    case "quarantined":
      return "This world broke a rule of the simulation and is in quarantine.";
    case "controller_taken":
      return "Another connection took control of this character.";
    case undefined:
      if (!connected) {
        return "Connecting…";
      }
      if (attacking) {
        return "Attack: press a direction. Escape cancels.";
      }
      return world.refused ? "Refused: nothing happened." : "";
  }
}

function name(world: PerceivedWorld, entity: number): string {
  return entity === world.controlled ? "@" : `entity ${entity}`;
}

/** The codec names a condition in snake case. */
function words(condition: Condition): string {
  return condition.replace("_", " ");
}

function happened(world: PerceivedWorld, outcome: Outcome): string {
  switch (outcome.kind) {
    case "stepped":
      return `steps ${outcome.direction}`;
    case "waited":
      return "waits";
    case "bumped":
      return `bumps into ${name(world, outcome.target)}`;
    case "hit":
      return `hits ${name(world, outcome.target)}`;
    case "missed":
      return `misses ${name(world, outcome.target)}`;
  }
}

function describe(action: Action): string {
  return action.kind === "wait" ? "wait" : `${action.kind} ${action.direction}`;
}

function item(text: string): HTMLLIElement {
  const element = document.createElement("li");
  element.textContent = text;
  return element;
}
