//! Attacks between entities, their hit points and their death (A1 to A8).

use alloc::vec::Vec;

use ash_core::entity::EntityId;
use ash_core::rng::Rng;
use ash_core::tile::Direction;

use crate::controller::{Controller, Disposition};
use crate::effect::Effect;
use crate::perception::{Outcome, PerceivedChange, Perception};
use crate::world::World;

/// A6: the percentage of attacks that hit.
const HIT_CHANCE: u64 = 70;

/// A6: the hit points a hit removes.
const DAMAGE: u32 = 3;

/// The hit points of an entity (A7). An embodied entity has at least one:
/// at zero it dies and leaves the world (A8).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Health {
    pub(crate) current: u32,
    pub(crate) max: u32,
}

impl Health {
    /// A7: the hit points an entity starts with, all of them: 10 for a
    /// character, 6 for a creature. Provisional numbers.
    pub(crate) fn full(controller: Controller) -> Self {
        let max = match controller {
            Controller::Player => 10,
            Controller::Wander(_) => 6,
        };
        Self { current: max, max }
    }

    /// These hit points less `damage`, never below zero.
    pub(crate) fn damaged(self, damage: u32) -> Self {
        Self {
            current: self.current.saturating_sub(damage),
            ..self
        }
    }
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

/// A6, A8: what a hit on `target` changes, in order. The target loses 3 hit
/// points. At zero it dies: it leaves the world, and every character
/// perceives its death at the time of the hit (A11). The death of a
/// character is out of scope: its zero hit points crash through
/// `invariant!`.
pub(crate) fn hit(world: &World, target: EntityId) -> Vec<Effect> {
    let mut effects = alloc::vec![Effect::Damage {
        entity: target,
        amount: DAMAGE,
    }];
    if world.health(target).damaged(DAMAGE).current == 0 {
        if world.controller(target) == Controller::Player {
            ash_core::invariant!("the character {target:?} reaches zero hit points");
        }
        effects.push(Effect::Remove { entity: target });
        effects.push(Effect::Perceive(PerceivedChange::Death {
            time: world.clock(),
            entity: target,
        }));
    }
    effects
}

#[cfg(test)]
mod tests {
    use ash_core::entity::Speed;
    use ash_core::tile::TilePosition;

    use super::*;
    use crate::action::Action;

    /// A7. The numbers are the design's: a changed one must fail here.
    #[test]
    fn a_character_starts_with_10_hit_points_and_a_creature_with_6() {
        let mut world = World::new(0, Speed::new(1));
        let character = world.awaited();
        let creature = world.create_entity(
            Speed::new(1),
            TilePosition { x: 2, y: 1, z: 0 },
            Controller::Wander(Disposition::Hostile),
        );

        assert_eq!(
            world.health(character),
            Health {
                current: 10,
                max: 10
            }
        );
        assert_eq!(world.health(creature), Health { current: 6, max: 6 });
    }

    /// A6: the first draw of seed 182 hits, the first draw of seed 18
    /// misses. The character strikes first (R8), so its strike takes it.
    #[test]
    fn a_hit_removes_3_hit_points_and_a_miss_none() {
        let creature = Controller::Wander(Disposition::Neutral);
        for (seed, controller, current, max) in [
            (182, creature, 3, 6),
            (18, creature, 6, 6),
            (182, Controller::Player, 7, 10),
            (18, Controller::Player, 10, 10),
        ] {
            let mut world = World::new(seed, Speed::new(1));
            let east = TilePosition { x: 2, y: 1, z: 0 };
            let target = world.create_entity(Speed::new(1), east, controller);

            assert_eq!(world.submit(Action::Attack(Direction::East)), Ok(()));

            assert_eq!(
                world.health(target),
                Health { current, max },
                "seed {seed}, {controller:?}"
            );
        }
    }

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
