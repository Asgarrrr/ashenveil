//! Acceptance of spec 0002, slice S4: the character sees the fight.
//!
//! The fixed map, with x to the east and y to the south; the character
//! starts at (1, 1):
//!
//! ```text
//!   012345678
//! 0 #########
//! 1 #.......#
//! 2 #....#..#
//! 3 #.....#.#
//! 4 #.......#
//! 5 #########
//! ```

use ash_core::entity::{EntityId, Speed};
use ash_core::tile::{Direction, TilePosition};
use ash_core::time::Time;
use ash_sim::action::Action;
use ash_sim::combat::Condition;
use ash_sim::controller::{Controller, Disposition};
use ash_sim::perception::{
    self, ChangeKind, Outcome, PerceivedChange, PerceivedEntity, PerceivedHealth,
};
use ash_sim::world::World;

/// Character turns after which a hunt that has not hit its target fails.
const HUNT_TURNS: usize = 2_000;

fn at(x: u32, y: u32) -> TilePosition {
    TilePosition { x, y, z: 0 }
}

/// The character, created first, then a hostile creature at (7, 4) and a
/// neutral one at (1, 4), both wandering at speed 2 (R6).
fn world_with_creatures(seed: u64) -> (World, EntityId, [EntityId; 2]) {
    let mut world = World::new(seed, Speed::new(1));
    let character = world.awaited();
    let creatures = [
        (Disposition::Hostile, at(7, 4)),
        (Disposition::Neutral, at(1, 4)),
    ]
    .map(|(disposition, start)| {
        world.create_entity(Speed::new(2), start, Controller::Wander(disposition))
    });
    (world, character, creatures)
}

/// What `viewer` perceives of `entity`, which is in the world.
fn perceived(world: &World, viewer: EntityId, entity: EntityId) -> PerceivedEntity {
    perception::snapshot(world, viewer)
        .entities
        .into_iter()
        .find(|perceived| perceived.entity == entity)
        .unwrap_or_else(|| panic!("{viewer:?} does not perceive {entity:?}"))
}

/// What `viewer` perceives of the hit points of `entity`.
fn health(world: &World, viewer: EntityId, entity: EntityId) -> PerceivedHealth {
    perceived(world, viewer, entity).health
}

/// True when `change` is a hit of `attacker` on `target`.
fn is_hit(change: PerceivedChange, attacker: EntityId, target: EntityId) -> bool {
    (change.entity, change.kind) == (attacker, ChangeKind::Act(Outcome::Hit(target)))
}

/// The index of each change of condition among the perceived changes.
fn condition_changes(world: &World) -> Vec<usize> {
    world
        .perceived_changes()
        .iter()
        .enumerate()
        .filter(|(_, change)| matches!(change.kind, ChangeKind::Condition(_)))
        .map(|(index, _)| index)
        .collect()
}

/// The attack toward `target` once it stands next to the awaited character
/// (A3); before that, the offered move that brings the character closest. A
/// wait when no move is offered.
fn strike_or_chase(world: &World, target: TilePosition) -> Action {
    let from = world.position(world.awaited());
    let distance = |position: TilePosition| {
        position
            .x
            .abs_diff(target.x)
            .max(position.y.abs_diff(target.y))
    };
    if let Some(toward) = Direction::ALL
        .into_iter()
        .find(|&direction| from.step(direction) == target)
    {
        return Action::Attack(toward);
    }
    world
        .offers()
        .into_iter()
        .filter_map(|offer| match offer.action {
            Action::Move(direction) => Some(direction),
            Action::Wait | Action::Attack(_) => None,
        })
        .min_by_key(|&direction| distance(from.step(direction)))
        .map_or(Action::Wait, Action::Move)
}

/// The character chases and strikes `target` until a strike hits. Returns
/// the index of that hit among the perceived changes, and its time.
fn hunt_until_hit(world: &mut World, character: EntityId, target: EntityId) -> (usize, Time) {
    for _ in 0..HUNT_TURNS {
        let action = strike_or_chase(world, perceived(world, character, target).position);
        let seen = world.perceived_changes().len();
        let time = world.clock();
        assert_eq!(world.submit(action), Ok(()), "{action:?}");
        if is_hit(world.perceived_changes()[seen], character, target) {
            return (seen, time);
        }
    }
    panic!("no hit on {target:?} in {HUNT_TURNS} turns");
}

#[test]
fn the_first_hit_moves_a_creature_from_unhurt_to_wounded_perceived_right_after_the_hit() {
    let mut hunts_with_a_miss_first = 0;
    for seed in 0..24 {
        // The hostile creature, then the neutral one: an attack strikes
        // either (A3), and both have 6 hit points (A7).
        for prey in 0..2 {
            let (mut world, character, creatures) = world_with_creatures(seed);
            let target = creatures[prey];
            let other = creatures[1 - prey];
            // A10: at full, every creature is unhurt.
            for creature in creatures {
                assert_eq!(
                    health(&world, character, creature),
                    PerceivedHealth::Condition(Condition::Unhurt),
                    "seed {seed}, prey {prey}"
                );
            }

            let (hit, time) = hunt_until_hit(&mut world, character, target);

            // A6, A10, A11: 3 of 6 hit points is below full and not below
            // half. The change of condition is the change right after the
            // hit, at the same time.
            let changes = world.perceived_changes();
            assert_eq!(
                changes.get(hit + 1),
                Some(&PerceivedChange {
                    time,
                    entity: target,
                    kind: ChangeKind::Condition(Condition::Wounded),
                }),
                "seed {seed}, prey {prey}"
            );
            // It is the only one: no miss and no act of a creature changes
            // a condition.
            assert_eq!(
                condition_changes(&world),
                [hit + 1],
                "seed {seed}, prey {prey}"
            );
            let missed_first = changes[..hit]
                .iter()
                .any(|change| change.kind == ChangeKind::Act(Outcome::Missed(target)));
            hunts_with_a_miss_first += usize::from(missed_first);

            // A10: the character perceives the creature it struck as
            // wounded, and the other as before.
            assert_eq!(
                health(&world, character, target),
                PerceivedHealth::Condition(Condition::Wounded),
                "seed {seed}, prey {prey}"
            );
            assert_eq!(
                health(&world, character, other),
                PerceivedHealth::Condition(Condition::Unhurt),
                "seed {seed}, prey {prey}"
            );
        }
    }
    // The hunts differ: some miss before they hit.
    assert!(hunts_with_a_miss_first > 0);
}

#[test]
fn the_lethal_hit_adds_no_condition_change() {
    for seed in 0..24 {
        let (mut world, character, [hostile, _]) = world_with_creatures(seed);

        let (first, _) = hunt_until_hit(&mut world, character, hostile);
        let (lethal, time) = hunt_until_hit(&mut world, character, hostile);

        // A8, A11: the death follows the lethal hit, at the same time.
        assert_eq!(
            world.perceived_changes().get(lethal + 1),
            Some(&PerceivedChange {
                time,
                entity: hostile,
                kind: ChangeKind::Death,
            }),
            "seed {seed}"
        );
        // The creature changed condition once, after its first hit: a dead
        // creature has no condition.
        assert_eq!(condition_changes(&world), [first + 1], "seed {seed}");
    }
}

#[test]
fn the_snapshot_of_a_character_holds_its_own_hit_points_and_the_condition_of_the_others() {
    let (mut world, first, creatures) = world_with_creatures(0);
    let second = world.create_entity(Speed::new(1), at(4, 4), Controller::Player);
    let own = PerceivedHealth::HitPoints {
        current: 10,
        max: 10,
    };
    let unhurt = PerceivedHealth::Condition(Condition::Unhurt);

    // A9, A10: the hit points of the character the snapshot is for, and of
    // no other entity: not of the other character, not of the awaited one.
    assert_eq!(world.awaited(), first);
    for (viewer, other) in [(first, second), (second, first)] {
        assert_eq!(health(&world, viewer, viewer), own, "{viewer:?}");
        for entity in creatures.into_iter().chain([other]) {
            assert_eq!(
                health(&world, viewer, entity),
                unhurt,
                "{viewer:?} perceives {entity:?}"
            );
        }
    }
}

#[test]
fn each_hit_on_a_character_shows_its_next_condition_to_the_other_and_its_hit_points_to_itself() {
    let mut misses = 0;
    for seed in 0..8 {
        // The attacker strikes the character east of it, which waits.
        let mut world = World::new(seed, Speed::new(1));
        let attacker = world.awaited();
        let target = world.create_entity(Speed::new(2), at(2, 1), Controller::Player);

        // A6, A7, A10: 10 hit points, less 3 at each hit. 7 is below full,
        // 4 is below half, 1 is below a quarter.
        let hits = [
            (7, Condition::Wounded),
            (4, Condition::BadlyWounded),
            (1, Condition::Dying),
        ];
        for (left, condition) in hits {
            let mut turns = 0;
            let time = loop {
                assert!(
                    turns < HUNT_TURNS,
                    "seed {seed}: no hit leaves {left} hit points in {HUNT_TURNS} turns"
                );
                turns += 1;
                if world.awaited() == target {
                    assert_eq!(world.submit(Action::Wait), Ok(()));
                    continue;
                }
                let seen = world.perceived_changes().len();
                let time = world.clock();
                assert_eq!(world.submit(Action::Attack(Direction::East)), Ok(()));
                let changes = &world.perceived_changes()[seen..];
                if is_hit(changes[0], attacker, target) {
                    // A11: the hit, then the change of condition, at the
                    // same time.
                    assert_eq!(
                        changes[1..],
                        [PerceivedChange {
                            time,
                            entity: target,
                            kind: ChangeKind::Condition(condition),
                        }],
                        "seed {seed}, {left} hit points left"
                    );
                    break time;
                }
                // A6: a miss changes nothing else.
                assert_eq!(
                    changes,
                    [PerceivedChange {
                        time,
                        entity: attacker,
                        kind: ChangeKind::Act(Outcome::Missed(target)),
                    }],
                    "seed {seed}"
                );
                misses += 1;
            };

            // A10: the attacker perceives a condition, never the hit points.
            assert_eq!(
                health(&world, attacker, target),
                PerceivedHealth::Condition(condition),
                "seed {seed}, at {time:?}"
            );
            // A9: the struck character perceives its own hit points, as
            // numbers.
            assert_eq!(
                health(&world, target, target),
                PerceivedHealth::HitPoints {
                    current: left,
                    max: 10,
                },
                "seed {seed}, at {time:?}"
            );
            // Nothing struck the attacker (A13).
            assert_eq!(
                health(&world, target, attacker),
                PerceivedHealth::Condition(Condition::Unhurt),
                "seed {seed}, at {time:?}"
            );
            assert_eq!(
                health(&world, attacker, attacker),
                PerceivedHealth::HitPoints {
                    current: 10,
                    max: 10,
                },
                "seed {seed}, at {time:?}"
            );
        }
    }
    // The seeds differ: some strikes miss.
    assert!(misses > 0);
}
