//! The messages between the server and its clients, and their binary codec.
//! The format is independent of the transport.
//!
//! The codec is minimal and replaceable: the wire format belongs to ADR 0003,
//! not decided yet. Messages name entities, never "the player".

// The web client loads this crate as WASM: std is allowed in tests only.
#![cfg_attr(not(test), no_std)]

extern crate alloc;

pub mod message;
mod wire;
