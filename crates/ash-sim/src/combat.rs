//! Attacks between entities, their hit points and their condition (A1 to
//! A10). The module `death` holds what a lethal hit changes.

use alloc::vec::Vec;

use ash_core::entity::EntityId;
use ash_core::rng::Rng;
use ash_core::tile::Direction;

use crate::controller::{Controller, Disposition};
use crate::death;
use crate::effect::Effect;
use crate::perception::{ChangeKind, Outcome, PerceivedChange, Perception};
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

    /// A10: the condition these hit points show. A product that saturates
    /// is above every `max`, like the product it stands for.
    pub(crate) fn condition(self) -> Condition {
        if self.current == self.max {
            Condition::Unhurt
        } else if self.current.saturating_mul(4) < self.max {
            Condition::Dying
        } else if self.current.saturating_mul(2) < self.max {
            Condition::BadlyWounded
        } else {
            Condition::Wounded
        }
    }
}

/// What a character perceives of another entity's hit points (A10): a kind,
/// never a number (R18). It derives from `Health` and is never stored (F6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Condition {
    /// All of its hit points.
    Unhurt,
    /// Fewer than all of them.
    Wounded,
    /// Fewer than half of them.
    BadlyWounded,
    /// Fewer than a quarter of them.
    Dying,
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

/// A11: what an attack that ended in `outcome` changes after its hit or its
/// miss, in order: what a hit does to its target, then the change of
/// disposition (A4). A creature that dies of the attack keeps its
/// disposition. Any other outcome changes nothing more.
pub(crate) fn struck(world: &World, outcome: Outcome) -> Vec<Effect> {
    let (target, mut effects) = match outcome {
        Outcome::Hit(target) => (target, hit(world, target)),
        Outcome::Missed(target) => (target, Vec::new()),
        Outcome::Stepped(_) | Outcome::Waited | Outcome::Bumped(_) => return Vec::new(),
    };
    let died = effects
        .iter()
        .any(|effect| matches!(effect, Effect::Remove { entity } if *entity == target));
    if !died {
        effects.extend(provoked(world, target));
    }
    effects
}

/// A4: what an attack on `target`, which lives on, changes of its
/// disposition. A neutral creature becomes hostile, perceived at the time
/// of the attack. A hostile creature stays hostile, and a character has no
/// disposition: nothing changes. Only a character attacks a creature (A13).
fn provoked(world: &World, target: EntityId) -> Vec<Effect> {
    let _ = (world, target);
    Vec::new()
}

/// A6, A8, A10: what a hit on `target` changes, in order. The target loses
/// 3 hit points. At zero it dies, with the effects of a death (D1). The
/// death of a character is out of scope: its zero hit points crash through
/// `invariant!`. A target that lives on in another condition is perceived
/// in it, at the time of the hit (A11).
fn hit(world: &World, target: EntityId) -> Vec<Effect> {
    let before = world.health(target);
    let health = before.damaged(DAMAGE);
    let mut effects = alloc::vec![Effect::Damage {
        entity: target,
        health,
    }];
    if health.current == 0 {
        if world.controller(target) == Controller::Player {
            ash_core::invariant!("the character {target:?} reaches zero hit points");
        }
        effects.extend(death::effects(world, target));
    } else if health.condition() != before.condition() {
        effects.push(Effect::Perceive(PerceivedChange {
            time: world.clock(),
            entity: target,
            kind: ChangeKind::Condition(health.condition()),
        }));
    }
    effects
}

#[cfg(test)]
mod tests {
    use ash_core::entity::Speed;
    use ash_core::tile::TilePosition;

    use super::*;
    use crate::action::{self, Action};

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

    /// A10: unhurt at full, wounded below full, badly wounded below half,
    /// dying below a quarter. Each row is the last hit point of a condition
    /// or the first of the next.
    #[test]
    fn a_condition_is_unhurt_wounded_badly_wounded_or_dying_by_its_threshold() {
        use Condition::{BadlyWounded, Dying, Unhurt, Wounded};
        let conditions = [
            // A character: half of 10 is 5, a quarter is 2.5.
            (10, 10, Unhurt),
            (9, 10, Wounded),
            (5, 10, Wounded),
            (4, 10, BadlyWounded),
            (3, 10, BadlyWounded),
            (2, 10, Dying),
            (1, 10, Dying),
            // A creature: half of 6 is 3, a quarter is 1.5.
            (6, 6, Unhurt),
            (5, 6, Wounded),
            (3, 6, Wounded),
            (2, 6, BadlyWounded),
            (1, 6, Dying),
            // Exactly half and exactly a quarter are not below them.
            (4, 8, Wounded),
            (3, 8, BadlyWounded),
            (2, 8, BadlyWounded),
            (1, 8, Dying),
            // The one hit point of an entity that has no more is all of them.
            (1, 1, Unhurt),
            (1, 2, Wounded),
            // The widest hit points: no product wraps into a lower condition.
            (u32::MAX, u32::MAX, Unhurt),
            (u32::MAX - 1, u32::MAX, Wounded),
            (u32::MAX / 2 + 1, u32::MAX, Wounded),
            (u32::MAX / 2, u32::MAX, BadlyWounded),
            (u32::MAX / 4 + 1, u32::MAX, BadlyWounded),
            (u32::MAX / 4, u32::MAX, Dying),
        ];
        for (current, max, condition) in conditions {
            assert_eq!(
                Health { current, max }.condition(),
                condition,
                "{current} of {max}"
            );
        }
    }

    /// F4: the rule decides what a hit changes. `resolve` returns the change
    /// of condition, after the hit (A11); `apply` only writes it.
    #[test]
    fn the_resolution_of_a_hit_across_a_threshold_returns_the_condition_change() {
        // The first draw of seed 182 hits (A6).
        let rng = Rng::new(182);
        let mut world = World::new(0, Speed::new(1));
        let attacker = world.awaited();
        // Hostile already, so the attack changes no disposition (A4).
        let target = world.create_entity(
            Speed::new(1),
            TilePosition { x: 2, y: 1, z: 0 },
            Controller::Wander(Disposition::Hostile),
        );
        let perception = Perception::whole_map(&world);

        let Ok(effects) = action::resolve(
            &world,
            &perception,
            attacker,
            Action::Attack(Direction::East),
            &rng,
        ) else {
            panic!("the rules refuse the attack");
        };

        let perceived: Vec<PerceivedChange> = effects
            .in_order
            .into_iter()
            .filter_map(|effect| {
                if let Effect::Perceive(change) = effect {
                    Some(change)
                } else {
                    None
                }
            })
            .collect();
        // A6, A10: 3 of 6 hit points is below full and not below half.
        assert_eq!(
            perceived,
            [
                PerceivedChange {
                    time: world.clock(),
                    entity: attacker,
                    kind: ChangeKind::Act(Outcome::Hit(target)),
                },
                PerceivedChange {
                    time: world.clock(),
                    entity: target,
                    kind: ChangeKind::Condition(Condition::Wounded),
                },
            ]
        );
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
