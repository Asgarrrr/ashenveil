// The messages of `ash-protocol`, through its Rust codec compiled to WASM
// (`crates/ash-wasm`). The shapes below mirror the JSON that crate emits.

import init, {
  decode_server_message,
  encode_attack,
  encode_hello,
  encode_move,
  encode_wait,
} from "../pkg/ash_wasm.js";

export type Direction =
  | "north"
  | "northeast"
  | "east"
  | "southeast"
  | "south"
  | "southwest"
  | "west"
  | "northwest";

export type Action =
  | { kind: "wait" }
  | { kind: "move"; direction: Direction }
  | { kind: "attack"; direction: Direction };

/** What an action did (A11), naming its target where it has one. */
export type Outcome =
  | { kind: "stepped"; direction: Direction }
  | { kind: "waited" }
  | { kind: "bumped"; target: number }
  | { kind: "hit"; target: number }
  | { kind: "missed"; target: number };

/**
 * A change every character perceives, stamped with the time it happened
 * (R17): an entity acts, or an entity dies and leaves the world (A8).
 */
export type PerceivedChange =
  | { kind: "act"; time: number; entity: number; outcome: Outcome }
  | { kind: "death"; time: number; entity: number };

export type Terrain = "floor" | "wall";

/** How a creature stands toward the character (A4). */
export type Disposition = "hostile" | "neutral";

export interface Position {
  x: number;
  y: number;
  z: number;
}

/** An action offered to the awaited entity, with the cost it would pay (R9). */
export interface AvailableAction {
  action: Action;
  cost: number;
}

export interface PerceivedEntity {
  entity: number;
  position: Position;
  /** `null` for a character: it has no disposition. */
  disposition: Disposition | null;
}

export type ServerMessage =
  | {
      kind: "snapshot";
      time: number;
      width: number;
      terrain: Terrain[];
      entities: PerceivedEntity[];
    }
  | { kind: "awaited"; time: number; entity: number; actions: AvailableAction[] }
  | { kind: "perceived"; change: PerceivedChange }
  | { kind: "refused" }
  | { kind: "quarantined" }
  | { kind: "controller_taken" };

/**
 * Loads the codec. The browser fetches the WASM next to the package; Node
 * passes its bytes.
 */
export async function loadCodec(wasm?: BufferSource): Promise<void> {
  await init(wasm === undefined ? undefined : { module_or_path: wasm });
}

/** The message `bytes` hold; `undefined` when they do not decode. */
export function decodeServerMessage(bytes: Uint8Array): ServerMessage | undefined {
  const json = decode_server_message(bytes);
  return json === undefined ? undefined : (JSON.parse(json) as ServerMessage);
}

// The codec returns fresh copies of WASM memory, never shared memory, which
// `WebSocket.send` requires.
type Bytes = Uint8Array<ArrayBuffer>;

/** `received`: the perceived changes of the world the client already has. */
export function encodeHello(token: bigint, received: number): Bytes {
  return encode_hello(token, BigInt(received)) as Bytes;
}

export function encodeCommand(entity: number, action: Action): Bytes {
  if (action.kind === "wait") {
    return encode_wait(entity) as Bytes;
  }
  const encode = action.kind === "move" ? encode_move : encode_attack;
  const bytes = encode(entity, action.direction);
  if (bytes === undefined) {
    throw new Error(`the codec knows no direction ${action.direction}`);
  }
  return bytes as Bytes;
}
