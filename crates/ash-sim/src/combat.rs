//! Attacks between entities (A3, A5, A6).

use ash_core::entity::EntityId;
use ash_core::rng::Rng;
use ash_core::tile::Direction;

use crate::perception::Outcome;
use crate::world::World;

/// The outcome of `attacker` striking toward `direction`, which the rules
/// allowed, and the RNG state after the draw that decides it (A6).
#[expect(
    unused_variables,
    reason = "S1: the attack rule is written in the next commit; no attack is allowed yet"
)]
pub(crate) fn attack(
    world: &World,
    attacker: EntityId,
    direction: Direction,
    rng: &Rng,
) -> (Outcome, Rng) {
    ash_core::invariant!("no attack is allowed before the attack rule")
}
