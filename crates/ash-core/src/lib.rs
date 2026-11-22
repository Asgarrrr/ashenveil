//! Coordinates, game time, ids, deterministic RNG, fixed-point and integer
//! helpers. Every other crate depends on it; it depends on nothing.

// No I/O and no clock in the simulation: std is allowed in tests only.
#![cfg_attr(not(test), no_std)]
// Simulation-only bans, here because `[workspace.lints]` cannot be extended.
#![deny(
    clippy::float_arithmetic,
    clippy::arithmetic_side_effects,
    clippy::disallowed_types
)]

pub mod entity;
mod invariant;
pub mod rng;
pub mod tile;
pub mod time;
