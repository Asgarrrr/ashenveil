//! The web client's codec: `ash-protocol` compiled to WASM, so the browser
//! and the server share one codec.
//!
//! Only free functions cross the boundary. A decoded message crosses as JSON
//! text; `web/src/protocol.ts` declares its shape.

use wasm_bindgen::prelude::wasm_bindgen;

/// The server message `bytes` hold, as JSON; `None` when they do not decode.
#[wasm_bindgen]
pub fn decode_server_message(bytes: &[u8]) -> Option<String> {
    let _ = bytes;
    None
}

/// The first message of a connection: take the controller of the world
/// `token` names.
#[wasm_bindgen]
pub fn encode_hello(token: u128) -> Vec<u8> {
    let _ = token;
    Vec::new()
}

/// `entity` waits.
#[wasm_bindgen]
pub fn encode_wait(entity: u32) -> Vec<u8> {
    let _ = entity;
    Vec::new()
}

/// `entity` moves toward `direction`, a name the decoded JSON uses; `None`
/// for any other name.
#[wasm_bindgen]
pub fn encode_move(entity: u32, direction: &str) -> Option<Vec<u8>> {
    let _ = (entity, direction);
    None
}
