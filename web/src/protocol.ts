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
 * What the character perceives of another entity's hit points (A10): a
 * kind, never a number.
 */
export type Condition = "unhurt" | "wounded" | "badly_wounded" | "dying";

/**
 * What a snapshot says of the hit points of an entity: numbers for the
 * character the snapshot is for (A9), a condition for every other (A10).
 */
export type PerceivedHealth =
  | { kind: "hit_points"; current: number; max: number }
  | { kind: "condition"; condition: Condition };

/**
 * A change the character perceives, stamped with the time it happened
 * (R17): an entity acts, an entity dies and leaves the world (A8), a hit
 * leaves another entity in another condition (A10), a hit leaves the
 * character itself with these hit points (A9), an attack leaves a creature
 * in another disposition (A4), or a stranger arrives on a tile after the
 * death it follows (D10).
 */
export type PerceivedChange = { time: number; entity: number } & (
  | { kind: "act"; outcome: Outcome }
  | { kind: "death" }
  | { kind: "condition"; condition: Condition }
  | { kind: "hit_points"; current: number; max: number }
  | { kind: "disposition"; disposition: Disposition }
  | { kind: "arrival"; position: Position }
);

export type Terrain = "floor" | "wall";

/** How a creature stands toward the character (A4). */
export type Disposition = "hostile" | "neutral";

export interface Position {
  x: number;
  y: number;
  z: number;
}

/** An action offered to the awaited entity, with the cost it would pay (R9). */
export interface Offer {
  action: Action;
  cost: number;
}

export interface PerceivedEntity {
  entity: number;
  position: Position;
  /** `null` for a character: it has no disposition. */
  disposition: Disposition | null;
  health: PerceivedHealth;
}

/** What died (D4). */
export type Died = "character" | "creature";

/**
 * A corpse the character perceives (D2, D4): where it lies, what died, and
 * the entity that died when the character recognizes it.
 */
export interface PerceivedCorpse {
  position: Position;
  died: Died;
  /** `null` when the character does not recognize the corpse. */
  entity: number | null;
}

export type ServerMessage =
  | {
      kind: "snapshot";
      time: number;
      width: number;
      terrain: Terrain[];
      entities: PerceivedEntity[];
      /** In the order of the deaths. */
      corpses: PerceivedCorpse[];
    }
  | { kind: "awaited"; time: number; entity: number; actions: Offer[] }
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

/**
 * `received`: the perceived changes the client already has, those of each
 * character its controller drove.
 */
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
