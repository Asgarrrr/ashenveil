// Throwaway Canvas 2D drawing of the perceived world. It reads the model and
// never changes it; the real renderer will replace this file.

import type { PerceivedWorld } from "./model.js";
import type { Action } from "./protocol.js";

const TILE = 32;

/** The page elements the drawing fills. */
export interface View {
  map: HTMLCanvasElement;
  status: HTMLElement;
  clock: HTMLElement;
  actions: HTMLElement;
  log: HTMLElement;
}

export function draw(view: View, world: PerceivedWorld, connected: boolean): void {
  drawMap(view.map, world);
  view.status.textContent = status(world, connected);
  view.clock.textContent = String(world.time);
  view.actions.replaceChildren(
    ...world.offer.map(({ action, cost }) => item(`${describe(action)}: ${cost}`)),
  );
  view.log.replaceChildren(
    ...world.log
      .slice(-20)
      .reverse()
      .map(({ time, entity, action }) => item(`${time}  ${name(world, entity)} ${describe(action)}`)),
  );
}

function drawMap(canvas: HTMLCanvasElement, world: PerceivedWorld): void {
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
  for (const [entity, { x, y }] of world.positions) {
    context.fillStyle = entity === world.controlled ? "#fc3" : "#6cf";
    context.fillText(entity === world.controlled ? "@" : "e", (x + 0.5) * TILE, (y + 0.5) * TILE);
  }
}

function status(world: PerceivedWorld, connected: boolean): string {
  switch (world.ended) {
    case "quarantined":
      return "This world broke a rule of the simulation and is in quarantine.";
    case "controller_taken":
      return "Another connection took control of this character.";
    case undefined:
      if (!connected) {
        return "Connecting…";
      }
      return world.refused ? "Refused: nothing happened." : "";
  }
}

function name(world: PerceivedWorld, entity: number): string {
  return entity === world.controlled ? "@" : `entity ${entity}`;
}

function describe(action: Action): string {
  return action.kind === "wait" ? "wait" : `move ${action.direction}`;
}

function item(text: string): HTMLLIElement {
  const element = document.createElement("li");
  element.textContent = text;
  return element;
}
