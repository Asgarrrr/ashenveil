//! Attacks between entities (A3, A5, A6).

use ash_core::entity::EntityId;
use ash_core::rng::Rng;
use ash_core::tile::Direction;

use crate::perception::Outcome;
use crate::world::World;

/// A6: the percentage of attacks that hit.
const HIT_CHANCE: u64 = 70;

/// The outcome of `attacker` striking toward `direction`, which the rules
/// allowed, and the RNG state after the one draw that decides it (A6).
pub(crate) fn attack(
    world: &World,
    attacker: EntityId,
    direction: Direction,
    rng: &Rng,
) -> (Outcome, Rng) {
    let to = world.position(attacker).step(direction);
    // Under whole-map perception (R16), the target the rules allowed is
    // there.
    let Some(target) = world.occupant(to) else {
        ash_core::invariant!("{attacker:?} attacks {to:?}, which no entity holds")
    };
    let mut after = rng.clone();
    let outcome = if after.next_u64() % 100 < HIT_CHANCE {
        Outcome::Hit(target)
    } else {
        Outcome::Missed(target)
    };
    (outcome, after)
}
