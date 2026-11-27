//! Acceptance of spec 0003, slice S2: an attacked neutral creature becomes
//! hostile.
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
use ash_sim::action::{Action, Refused};
use ash_sim::combat::Condition;
use ash_sim::controller::{Controller, Disposition};
use ash_sim::perception::{self, ChangeKind, Outcome, PerceivedChange};
use ash_sim::world::World;

/// A seed whose first draw misses (A6).
const MISS_SEED: u64 = 18;

/// A seed whose first draw hits (A6).
const HIT_SEED: u64 = 182;

/// Turns after which a fight that has not ended fails.
const MOST_TURNS: usize = 2_000;

fn at(x: u32, y: u32) -> TilePosition {
    TilePosition { x, y, z: 0 }
}

/// The narrow passage at (7, 3) holds a neutral creature. A wall stands on
/// both of its sides, and R11 refuses each diagonal out of it. The hunter
/// stands north of it and the blocker south: the creature can only bump or
/// wait, so the hunter can always strike it to the south. The watcher is
/// the character every world starts with, far away.
struct Passage {
    world: World,
    watcher: EntityId,
    hunter: EntityId,
    creature: EntityId,
    blocker: EntityId,
}

fn passage(seed: u64) -> Passage {
    let mut world = World::new(seed, Speed::new(1));
    let watcher = world.awaited();
    let hunter = world.create_entity(Speed::new(1), at(7, 2), Controller::Player);
    let creature = world.create_entity(
        Speed::new(2),
        at(7, 3),
        Controller::Wander(Disposition::Neutral),
    );
    let blocker = world.create_entity(Speed::new(1), at(7, 4), Controller::Player);
    Passage {
        world,
        watcher,
        hunter,
        creature,
        blocker,
    }
}

/// Every other character waits until the world awaits the hunter. A wait
/// draws nothing, so the hunter's first strike takes the first draw.
fn to_hunter(world: &mut World, hunter: EntityId) {
    for _ in 0..MOST_TURNS {
        if world.awaited() == hunter {
            return;
        }
        world.submit(Action::Wait).expect("a wait is never refused");
    }
    panic!("the hunter had no turn in {MOST_TURNS} turns");
}

/// The changes of the action whose change is at `seen`: that change, then
/// every change up to the next act (A11).
fn changes_of_the_action(world: &World, seen: usize) -> Vec<PerceivedChange> {
    let changes = &world.perceived_changes()[seen..];
    let rest = changes[1..]
        .iter()
        .take_while(|change| !matches!(change.kind, ChangeKind::Act(_)));
    changes[..1].iter().chain(rest).copied().collect()
}

/// The disposition `viewer` perceives of `creature` (A4).
fn disposition(world: &World, viewer: EntityId, creature: EntityId) -> Option<Disposition> {
    perception::snapshot(world, viewer)
        .entities
        .iter()
        .find(|perceived| perceived.entity == creature)
        .and_then(|perceived| perceived.disposition)
}

/// The changes of disposition every character perceived, in order.
fn changes_of_disposition(world: &World) -> Vec<PerceivedChange> {
    world
        .perceived_changes()
        .iter()
        .filter(|change| matches!(change.kind, ChangeKind::Disposition(_)))
        .copied()
        .collect()
}

/// The hunter strikes the creature south once: the changes of that strike.
fn strike(world: &mut World, hunter: EntityId, action: Action) -> Vec<PerceivedChange> {
    to_hunter(world, hunter);
    let seen = world.perceived_changes().len();
    assert_eq!(world.submit(action), Ok(()), "{action:?}");
    changes_of_the_action(world, seen)
}

#[test]
fn a_missed_attack_on_the_neutral_creature_is_followed_by_its_change_of_disposition() {
    let Passage {
        mut world,
        hunter,
        creature,
        ..
    } = passage(MISS_SEED);
    to_hunter(&mut world, hunter);
    let time = world.clock();

    let changes = strike(&mut world, hunter, Action::Attack(Direction::South));

    // A4, A11: the miss, then the change of disposition, at the time of the
    // attack.
    assert_eq!(
        changes,
        [
            PerceivedChange {
                time,
                entity: hunter,
                kind: ChangeKind::Act(Outcome::Missed(creature)),
            },
            PerceivedChange {
                time,
                entity: creature,
                kind: ChangeKind::Disposition(Disposition::Hostile),
            },
        ]
    );
}

#[test]
fn a_hit_on_the_neutral_creature_is_followed_by_its_change_of_condition_then_of_disposition() {
    let Passage {
        mut world,
        hunter,
        creature,
        ..
    } = passage(HIT_SEED);
    to_hunter(&mut world, hunter);
    let time = world.clock();

    let changes = strike(&mut world, hunter, Action::Attack(Direction::South));

    // A11: the hit, what the hit does to the creature, then the change of
    // disposition. 3 of 6 hit points is below full (A6, A10).
    assert_eq!(
        changes,
        [
            PerceivedChange {
                time,
                entity: hunter,
                kind: ChangeKind::Act(Outcome::Hit(creature)),
            },
            PerceivedChange {
                time,
                entity: creature,
                kind: ChangeKind::Condition(Condition::Wounded),
            },
            PerceivedChange {
                time,
                entity: creature,
                kind: ChangeKind::Disposition(Disposition::Hostile),
            },
        ]
    );
}

#[test]
fn every_character_perceives_the_provoked_creature_as_hostile() {
    for seed in [MISS_SEED, HIT_SEED] {
        let Passage {
            mut world,
            watcher,
            hunter,
            creature,
            blocker,
        } = passage(seed);
        let characters = [watcher, hunter, blocker];
        let dispositions =
            |world: &World| characters.map(|viewer| disposition(world, viewer, creature));
        assert_eq!(
            dispositions(&world),
            [Some(Disposition::Neutral); 3],
            "seed {seed}"
        );

        strike(&mut world, hunter, Action::Attack(Direction::South));

        // A4, D12: a property of the creature, the same for every character.
        assert_eq!(
            dispositions(&world),
            [Some(Disposition::Hostile); 3],
            "seed {seed}"
        );
    }
}

#[test]
fn the_next_move_into_the_provoked_creature_is_an_attack_that_adds_no_change_of_disposition() {
    for seed in [MISS_SEED, HIT_SEED] {
        let Passage {
            mut world,
            hunter,
            creature,
            ..
        } = passage(seed);
        // The control: before the attack, the move into the neutral creature
        // is a bump and nothing else (A2). A bump provokes nothing.
        let mut neutral = world.clone();
        let bump = strike(&mut neutral, hunter, Action::Move(Direction::South));
        assert_eq!(
            bump.iter().map(|change| change.kind).collect::<Vec<_>>(),
            [ChangeKind::Act(Outcome::Bumped(creature))],
            "seed {seed}"
        );

        strike(&mut world, hunter, Action::Attack(Direction::South));
        let changes = strike(&mut world, hunter, Action::Move(Direction::South));

        // A1: a hit or a miss of the creature, now hostile.
        assert!(
            matches!(
                changes[0].kind,
                ChangeKind::Act(Outcome::Hit(target) | Outcome::Missed(target)) if target == creature
            ),
            "seed {seed}: {changes:?}"
        );
        // A4: it stays hostile. The second attack adds no change of
        // disposition, and the first added one.
        assert!(
            changes
                .iter()
                .all(|change| !matches!(change.kind, ChangeKind::Disposition(_))),
            "seed {seed}: {changes:?}"
        );
        assert_eq!(changes_of_disposition(&world).len(), 1, "seed {seed}");
    }
}

#[test]
fn an_attack_into_an_empty_direction_is_refused_and_changes_no_disposition() {
    let Passage {
        mut world,
        watcher,
        hunter,
        creature,
        ..
    } = passage(MISS_SEED);
    to_hunter(&mut world, hunter);
    let before = world.clone();
    // (7, 1) is floor, and no entity stands there (A3).
    let empty = Action::Attack(Direction::North);
    assert!(
        world.offers().iter().all(|offer| offer.action != empty),
        "an attack toward no entity is offered"
    );

    assert_eq!(world.submit(empty), Err(Refused));

    // R10, F4: no disposition, RNG state or perceived change moves.
    assert_eq!(
        disposition(&world, watcher, creature),
        Some(Disposition::Neutral)
    );
    assert_eq!(world.perceived_changes(), before.perceived_changes());
    assert_eq!(world, before);
    // The draw the refusal left is the one the attack that follows takes:
    // the first of the seed, a miss.
    let changes = strike(&mut world, hunter, Action::Attack(Direction::South));
    assert_eq!(changes[0].kind, ChangeKind::Act(Outcome::Missed(creature)));
}

#[test]
fn the_run_replays_identically() {
    for seed in [MISS_SEED, HIT_SEED] {
        let Passage {
            mut world,
            hunter,
            creature,
            ..
        } = passage(seed);
        // The hunter strikes until the creature dies, and no snapshot shows
        // it: by an attack first, then by moves into it.
        let mut action = Action::Attack(Direction::South);
        for _ in 0..MOST_TURNS {
            if disposition(&world, hunter, creature).is_none() {
                break;
            }
            strike(&mut world, hunter, action);
            action = Action::Move(Direction::South);
        }
        assert!(
            world
                .perceived_changes()
                .iter()
                .any(|change| change.entity == creature && change.kind == ChangeKind::Death),
            "seed {seed}: the creature lives after {MOST_TURNS} turns"
        );
        assert_eq!(changes_of_disposition(&world).len(), 1, "seed {seed}");

        let mut replay = passage(seed).world;
        for &action in world.command_log() {
            assert_eq!(replay.submit(action), Ok(()), "seed {seed}: {action:?}");
        }
        assert_eq!(replay.perceived_changes(), world.perceived_changes());
        assert_eq!(replay, world, "seed {seed}");
    }
}
