// The page: connects to the server, feeds every message to the model, sends
// the command of each key, and redraws.

import { draw, type View } from "./draw.js";
import { Keyboard } from "./keys.js";
import { PerceivedWorld } from "./model.js";
import { decodeServerMessage, encodeCommand, encodeHello, loadCodec } from "./protocol.js";

/** `cargo xtask play` starts the server here (`xtask/src/play.rs`). */
const SERVER = "ws://127.0.0.1:8001";
const RECONNECT_DELAY_MS = 1000;
const TOKEN_KEY = "ashenveil.resume-token";

await loadCodec();

const world = new PerceivedWorld();
const view: View = {
  map: element("map") as HTMLCanvasElement,
  status: element("status"),
  clock: element("clock"),
  entities: element("entities"),
  actions: element("actions"),
  log: element("log"),
};
const token = resumeToken();
const keyboard = new Keyboard();
let socket: WebSocket | undefined;

connect();
document.addEventListener("keydown", (event) => {
  const action = keyboard.press(event.code);
  // R14: only an offered action is sent, so none is refused.
  if (action === undefined || world.controlled === undefined || world.costOf(action) === undefined) {
    return;
  }
  event.preventDefault();
  socket?.send(encodeCommand(world.controlled, action));
  // A second key before the next turn must not send a command for this one.
  world.offer = [];
  redraw();
});

function connect(): void {
  const opened = new WebSocket(SERVER);
  opened.binaryType = "arraybuffer";
  opened.onopen = () => {
    socket = opened;
    // R17: the log holds every perceived change received, so the server
    // sends only the others.
    opened.send(encodeHello(token, world.log.length));
  };
  opened.onmessage = (event: MessageEvent<ArrayBuffer>) => {
    const message = decodeServerMessage(new Uint8Array(event.data));
    if (message === undefined) {
      opened.close();
      return;
    }
    world.apply(message);
    redraw();
  };
  opened.onclose = () => {
    socket = undefined;
    redraw();
    // The server answers a quarantined world, or a taken character, the
    // same way on every reconnect.
    if (world.mayReconnect()) {
      setTimeout(connect, RECONNECT_DELAY_MS);
    }
  };
  redraw();
}

function redraw(): void {
  draw(view, world, socket !== undefined);
}

/** The token of this browser's world, chosen once and kept. */
function resumeToken(): bigint {
  const stored = localStorage.getItem(TOKEN_KEY);
  if (stored !== null) {
    return BigInt(stored);
  }
  const [high = 0n, low = 0n] = crypto.getRandomValues(new BigUint64Array(2));
  const chosen = (high << 64n) | low;
  localStorage.setItem(TOKEN_KEY, chosen.toString());
  return chosen;
}

function element(id: string): HTMLElement {
  const found = document.getElementById(id);
  if (found === null) {
    throw new Error(`the page has no #${id}`);
  }
  return found;
}
