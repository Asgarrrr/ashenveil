//! World, scheduler and systems. Pure: no I/O, no clock.

// No I/O and no clock in the simulation: std is allowed in tests only.
#![cfg_attr(not(test), no_std)]
// Simulation-only bans, here because `[workspace.lints]` cannot be extended.
#![deny(
    clippy::float_arithmetic,
    clippy::arithmetic_side_effects,
    clippy::disallowed_types
)]

extern crate alloc;

pub mod action;
pub mod combat;
pub mod controller;
pub mod death;
mod effect;
mod map;
pub mod perception;
pub mod scheduler;
mod wander;
pub mod world;
