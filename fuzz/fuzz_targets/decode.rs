//! Decoding never panics on any input (spec 0001, S4).

#![no_main]

use ash_protocol::message::{ClientMessage, ServerMessage};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|bytes: &[u8]| {
    let _ = ClientMessage::decode(bytes);
    let _ = ServerMessage::decode(bytes);
});
