// The messages of `ash-protocol`, through its Rust codec compiled to WASM
// (`crates/ash-wasm`). The shapes below mirror the JSON that crate emits.

import init, {
  decode_server_message,
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

export type Action = { kind: "wait" } | { kind: "move"; direction: Direction };

export type Terrain = "floor" | "wall";

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
  | { kind: "perceived"; time: number; entity: number; action: Action }
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
  void bytes;
  void decode_server_message;
  return undefined;
}

export function encodeHello(token: bigint): Uint8Array {
  return encode_hello(token);
}

export function encodeCommand(entity: number, action: Action): Uint8Array {
  void entity;
  void action;
  void encode_move;
  void encode_wait;
  return new Uint8Array();
}
