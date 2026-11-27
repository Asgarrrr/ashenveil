//! Attacks between entities (A1 to A6).

use ash_core::entity::EntityId;
use ash_core::rng::Rng;
use ash_core::tile::Direction;

use crate::controller::Controller;
use crate::perception::{Outcome, Perception};
use crate::world::World;

/// A6: the percentage of attacks that hit.
const HIT_CHANCE: u64 = 70;

/// A4: how a creature stands toward characters. A kind, never a number
/// (R18).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Disposition {
    /// A character's move into the creature is an attack on it (A1).
    Hostile,
    /// A character's move into the creature is a bump (A2).
    Neutral,
}

/// A1: whether the move of `mover` toward `direction` is an attack on the
/// entity there: `mover` is a character, and it perceives that entity as a
/// hostile creature. A creature's move never is (A13).
pub(crate) fn move_attacks(
    world: &World,
    perception: &Perception<'_>,
    mover: EntityId,
    direction: Direction,
) -> bool {
    let to = perception.position(mover).step(direction);
    world.controller(mover) == Controller::Player
        && perception
            .occupant(to)
            .and_then(|occupant| perception.disposition(occupant))
            == Some(Disposition::Hostile)
}

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

#[cfg(test)]
mod tests {
    use ash_core::entity::Speed;
    use ash_core::tile::TilePosition;

    use super::*;
    use crate::controller::Controller;

    /// A6: an attack hits when `draw % 100 < 70`. The first draw of seed 182
    /// leaves 69, the first draw of seed 18 leaves 70.
    #[test]
    fn an_attack_hits_when_its_one_draw_leaves_less_than_70() {
        let mut world = World::new(0, Speed::new(1));
        let attacker = world.awaited();
        let east = TilePosition { x: 2, y: 1, z: 0 };
        let target = world.create_entity(
            Speed::new(1),
            east,
            Controller::Wander(Disposition::Neutral),
        );

        for (seed, remainder, outcome) in [
            (182, 69, Outcome::Hit(target)),
            (18, 70, Outcome::Missed(target)),
        ] {
            let rng = Rng::new(seed);
            let mut after_one_draw = rng.clone();
            assert_eq!(after_one_draw.next_u64() % 100, remainder, "seed {seed}");
            assert_eq!(
                attack(&world, attacker, Direction::East, &rng),
                (outcome, after_one_draw),
                "seed {seed}"
            );
        }
    }
}
