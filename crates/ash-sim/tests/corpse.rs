//! Acceptance of spec 0003, slice S1: a death leaves a corpse.
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
use ash_sim::action::Action;
use ash_sim::controller::{Controller, Disposition};
use ash_sim::death::Died;
use ash_sim::perception::{self, ChangeKind, Outcome, PerceivedChange, PerceivedCorpse};
use ash_sim::world::World;

/// A seed where the character kills the hostile creature of the skeleton
/// world and can then step onto its tile.
const SEED: u64 = 0;

/// Turns after which a fight that has not ended fails.
const MOST_TURNS: usize = 2_000;

fn at(x: u32, y: u32) -> TilePosition {
    TilePosition { x, y, z: 0 }
}

/// The corpses `viewer` perceives, in the order of the snapshot.
fn corpses(world: &World, viewer: EntityId) -> Vec<PerceivedCorpse> {
    perception::snapshot(world, viewer).corpses
}

/// D2, D4: the corpse of the creature `entity`, which died at `position`,
/// as a character that perceived it alive sees it.
fn creature_corpse(entity: EntityId, position: TilePosition) -> PerceivedCorpse {
    PerceivedCorpse {
        position,
        died: Died::Creature,
        entity: Some(entity),
    }
}

/// Where the awaited character perceives `entity`; `None` once it left the
/// world.
fn standing(world: &World, entity: EntityId) -> Option<TilePosition> {
    perception::snapshot(world, world.awaited())
        .entities
        .iter()
        .find(|perceived| perceived.entity == entity)
        .map(|perceived| perceived.position)
}

/// The direction from `from` to the neighboring tile `to`, if they touch.
fn toward(from: TilePosition, to: TilePosition) -> Option<Direction> {
    Direction::ALL
        .into_iter()
        .find(|&direction| from.step(direction) == to)
}

/// The attack toward `target` once it stands next to the awaited character
/// (A3); before that, the offered move that brings the character closest. A
/// wait when no move is offered.
fn strike_or_chase(world: &World, target: TilePosition) -> Action {
    let from = world.position(world.awaited());
    if let Some(direction) = toward(from, target) {
        return Action::Attack(direction);
    }
    let distance = |position: TilePosition| {
        position
            .x
            .abs_diff(target.x)
            .max(position.y.abs_diff(target.y))
    };
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

/// The entities whose death the characters perceived, in order.
fn deaths(world: &World) -> Vec<EntityId> {
    world
        .perceived_changes()
        .iter()
        .filter(|change| change.kind == ChangeKind::Death)
        .map(|change| change.entity)
        .collect()
}

/// A world built by `setup`, then the accepted commands of `run`: the same
/// world, bit for bit, with the same corpses (F6, D12).
fn assert_replays(run: &World, setup: impl Fn() -> World, viewer: EntityId) {
    let mut replay = setup();
    for &action in run.command_log() {
        assert_eq!(replay.submit(action), Ok(()), "{action:?}");
    }
    assert_eq!(corpses(&replay, viewer), corpses(run, viewer));
    assert_eq!(&replay, run);
}

#[test]
fn the_killed_hostile_creature_leaves_its_corpse_where_it_died_and_a_move_onto_it_is_a_step() {
    let mut world = World::skeleton(SEED);
    let character = world.awaited();
    let hostile = perception::snapshot(&world, character)
        .entities
        .iter()
        .find(|perceived| perceived.disposition == Some(Disposition::Hostile))
        .map(|perceived| perceived.entity)
        .expect("the skeleton world holds a hostile creature");
    assert_eq!(corpses(&world, character), []);

    // The character strikes the creature until it dies. The tile of the
    // last strike is the tile of the death: the strike resolves before any
    // other entity acts (R2).
    let mut struck_at = None;
    for _ in 0..MOST_TURNS {
        let Some(target) = standing(&world, hostile) else {
            break;
        };
        struck_at = Some(target);
        let action = strike_or_chase(&world, target);
        assert_eq!(world.submit(action), Ok(()), "{action:?}");
    }
    assert_eq!(deaths(&world), [hostile], "no death in {MOST_TURNS} turns");
    let died_at = struck_at.expect("the character struck the creature");

    // D2, D4: one corpse, on the tile of the death, naming the creature.
    assert_eq!(
        corpses(&world, character),
        [creature_corpse(hostile, died_at)]
    );

    // D3: the corpse never blocks. No attack is offered toward it, and the
    // move onto its tile is a step, not a bump (R4, R13).
    let from = world.position(character);
    let direction = toward(from, died_at).expect("the character struck from the next tile");
    let offered: Vec<Action> = world
        .offers()
        .into_iter()
        .map(|offer| offer.action)
        .collect();
    assert!(!offered.contains(&Action::Attack(direction)), "{offered:?}");
    let (time, seen) = (world.clock(), world.perceived_changes().len());
    assert_eq!(world.submit(Action::Move(direction)), Ok(()));
    assert_eq!(
        world.perceived_changes()[seen],
        PerceivedChange {
            time,
            entity: character,
            kind: ChangeKind::Act(Outcome::Stepped(direction)),
        }
    );
    assert_eq!(world.position(character), died_at);
    // The corpse stays where it lies, under the character.
    assert_eq!(
        corpses(&world, character),
        [creature_corpse(hostile, died_at)]
    );

    assert_replays(&world, || World::skeleton(SEED), character);
}

/// The narrow passage at (7, 3) holds the first creature. A wall stands on
/// both of its sides, and R11 refuses each diagonal out of it. A hunting
/// character stands north of it, the second creature south. A watching
/// character blocks the only other way out of the tile of the second
/// creature. Both creatures can only bump or wait, until the first dies
/// and the second can step onto its tile.
struct Passage {
    world: World,
    watcher: EntityId,
    hunter: EntityId,
    first: EntityId,
    second: EntityId,
}

/// The tile of both deaths.
const PASSAGE: TilePosition = TilePosition { x: 7, y: 3, z: 0 };

fn passage(seed: u64) -> Passage {
    let mut world = World::new(seed, Speed::new(1));
    let watcher = world.awaited();
    // The second creature to die is created first: the order of the deaths
    // is not the order of creation, nor the order of the ids.
    let second = world.create_entity(
        Speed::new(2),
        at(7, 4),
        Controller::Wander(Disposition::Neutral),
    );
    world.create_entity(Speed::new(1), at(6, 4), Controller::Player);
    let hunter = world.create_entity(Speed::new(1), at(7, 2), Controller::Player);
    let first = world.create_entity(
        Speed::new(2),
        PASSAGE,
        Controller::Wander(Disposition::Hostile),
    );
    Passage {
        world,
        watcher,
        hunter,
        first,
        second,
    }
}

#[test]
fn two_deaths_on_one_tile_leave_two_corpses_in_the_order_of_the_deaths() {
    let Passage {
        mut world,
        watcher,
        hunter,
        first,
        second,
    } = passage(SEED);

    // The hunter strikes south whenever a creature stands in the passage;
    // every other character waits. So both creatures die there.
    for _ in 0..MOST_TURNS {
        if deaths(&world).len() == 2 {
            break;
        }
        let in_passage = [first, second]
            .into_iter()
            .any(|creature| standing(&world, creature) == Some(PASSAGE));
        let action = if world.awaited() == hunter && in_passage {
            Action::Attack(Direction::South)
        } else {
            Action::Wait
        };
        assert_eq!(world.submit(action), Ok(()), "{action:?}");
    }
    assert_eq!(deaths(&world), [first, second]);

    // D2, D3: two corpses on one tile, in the order of the deaths.
    assert_eq!(
        corpses(&world, watcher),
        [
            creature_corpse(first, PASSAGE),
            creature_corpse(second, PASSAGE),
        ]
    );

    assert_replays(&world, || passage(SEED).world, watcher);
}

/// The first creature stands in the corner at (7, 4), the second at (5, 3),
/// north and west of it: the order of the deaths is not the order of the
/// tiles. Walls, R11 and waiting characters leave each creature no move. A
/// striking character stands north of the first, another west of the second.
struct Corners {
    world: World,
    watcher: EntityId,
    first_striker: EntityId,
    second_striker: EntityId,
    first: EntityId,
    second: EntityId,
}

fn corners(seed: u64) -> Corners {
    let mut world = World::new(seed, Speed::new(1));
    let watcher = world.awaited();
    // Created first, the second creature to die has the lower id.
    let second = world.create_entity(
        Speed::new(2),
        at(5, 3),
        Controller::Wander(Disposition::Neutral),
    );
    let second_striker = world.create_entity(Speed::new(1), at(4, 3), Controller::Player);
    world.create_entity(Speed::new(1), at(4, 4), Controller::Player);
    world.create_entity(Speed::new(1), at(5, 4), Controller::Player);
    world.create_entity(Speed::new(1), at(6, 4), Controller::Player);
    let first_striker = world.create_entity(Speed::new(1), at(7, 3), Controller::Player);
    let first = world.create_entity(
        Speed::new(2),
        at(7, 4),
        Controller::Wander(Disposition::Hostile),
    );
    Corners {
        world,
        watcher,
        first_striker,
        second_striker,
        first,
        second,
    }
}

#[test]
fn two_deaths_on_two_tiles_leave_their_corpses_in_the_order_of_the_deaths() {
    let Corners {
        mut world,
        watcher,
        first_striker,
        second_striker,
        first,
        second,
    } = corners(SEED);

    // One striker strikes south until the first creature dies, then the
    // other strikes east until the second dies. Every other character waits.
    for _ in 0..MOST_TURNS {
        let died = deaths(&world).len();
        if died == 2 {
            break;
        }
        let awaited = world.awaited();
        let action = if died == 0 && awaited == first_striker {
            Action::Attack(Direction::South)
        } else if died == 1 && awaited == second_striker {
            Action::Attack(Direction::East)
        } else {
            Action::Wait
        };
        assert_eq!(world.submit(action), Ok(()), "{action:?}");
    }
    assert_eq!(deaths(&world), [first, second]);

    // D2, D4: each corpse on the tile of its death, in the order of the
    // deaths.
    assert_eq!(
        corpses(&world, watcher),
        [
            creature_corpse(first, at(7, 4)),
            creature_corpse(second, at(5, 3)),
        ]
    );

    assert_replays(&world, || corners(SEED).world, watcher);
}
