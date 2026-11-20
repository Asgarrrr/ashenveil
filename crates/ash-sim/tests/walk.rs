//! Acceptance of spec 0001, slice S3: the character walks.
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
use ash_sim::action::{Action, Refused};
use ash_sim::perception::PerceivedChange;
use ash_sim::world::World;

use Direction::{East, North, NorthEast, NorthWest, South, SouthEast, SouthWest, West};

const SEED: u64 = 11;

fn at(x: u32, y: u32) -> TilePosition {
    TilePosition { x, y, z: 0 }
}

fn moved(time: u64, entity: EntityId, direction: Direction) -> PerceivedChange {
    PerceivedChange {
        time: Time::new(time),
        entity,
        action: Action::Move(direction),
    }
}

fn walk(world: &mut World, directions: &[Direction]) {
    for &direction in directions {
        world
            .submit(Action::Move(direction))
            .expect("a scripted move is never refused");
    }
}

/// R10, R11: a refused move is not an action. The whole world, its RNG
/// included, compares equal before and after.
fn assert_refused(world: &mut World, direction: Direction) {
    let before = world.clone();

    assert_eq!(world.submit(Action::Move(direction)), Err(Refused));

    assert_eq!(world.clock(), before.clock());
    assert_eq!(world.command_log(), before.command_log());
    assert_eq!(world.perceived_changes(), before.perceived_changes());
    assert_eq!(*world, before);
}

#[test]
fn a_scripted_walk_gives_the_expected_positions_and_clock() {
    let mut world = World::new(SEED, Speed::new(1));
    let character = world.awaited();
    assert_eq!(world.position(character), at(1, 1));

    // One move in each of the 8 directions, around a loop back to the start.
    let script = [
        (East, at(2, 1)),
        (SouthEast, at(3, 2)),
        (South, at(3, 3)),
        (SouthWest, at(2, 4)),
        (West, at(1, 4)),
        (North, at(1, 3)),
        (NorthEast, at(2, 2)),
        (NorthWest, at(1, 1)),
    ];
    for (step, &(direction, expected)) in script.iter().enumerate() {
        assert_eq!(world.submit(Action::Move(direction)), Ok(()));
        assert_eq!(world.position(character), expected, "after move {step}");
    }

    // A move costs 100 at speed 1 (R5, R6), straight or diagonal.
    assert_eq!(world.clock(), Time::new(800));
    assert_eq!(world.awaited(), character);
    let directions = script.map(|(direction, _)| direction);
    assert_eq!(world.command_log(), directions.map(Action::Move));
    // R16, R17: every move is perceived, in order, stamped with its time.
    let expected_changes: Vec<_> = (0..)
        .step_by(100)
        .zip(directions)
        .map(|(time, direction)| moved(time, character, direction))
        .collect();
    assert_eq!(world.perceived_changes(), expected_changes);
}

#[test]
fn a_move_into_a_wall_is_refused() {
    let mut world = World::new(SEED, Speed::new(1));
    assert_refused(&mut world, North);
    assert_refused(&mut world, NorthWest);

    walk(&mut world, &[East, East, East]);
    assert_eq!(world.position(world.awaited()), at(4, 1));
    // (5, 2) is a wall, but (5, 1) and (4, 2) are floor: no corner is cut.
    assert_refused(&mut world, SouthEast);

    walk(&mut world, &[South]);
    assert_refused(&mut world, East);
}

#[test]
fn a_diagonal_between_two_orthogonal_walls_is_refused() {
    let mut world = World::new(SEED, Speed::new(1));
    walk(&mut world, &[East, East, East, South, South, East]);
    assert_eq!(world.position(world.awaited()), at(5, 3));

    // (6, 2) is floor, but (5, 2) and (6, 3) are walls.
    assert_refused(&mut world, NorthEast);
}

#[test]
fn a_diagonal_along_a_wall_corner_is_refused() {
    let mut world = World::new(SEED, Speed::new(1));
    walk(&mut world, &[East, East, East, East]);
    assert_eq!(world.position(world.awaited()), at(5, 1));
    // (6, 2) is floor; the wall (5, 2) is the corner on the way south.
    assert_refused(&mut world, SouthEast);

    walk(&mut world, &[West, South]);
    assert_eq!(world.position(world.awaited()), at(4, 2));
    // (5, 3) is floor; the wall (5, 2) is the corner on the way east.
    assert_refused(&mut world, SouthEast);
}

#[test]
fn the_walk_replays_identically() {
    let mut first = World::new(SEED, Speed::new(1));
    assert_eq!(first.submit(Action::Move(North)), Err(Refused));
    walk(&mut first, &[East, SouthEast, South, SouthWest, West]);
    first.submit(Action::Wait).expect("a wait is never refused");
    walk(&mut first, &[NorthEast]);

    let mut replay = World::new(SEED, Speed::new(1));
    for &action in first.command_log() {
        assert_eq!(replay.submit(action), Ok(()));
    }

    assert_eq!(replay.perceived_changes(), first.perceived_changes());
    assert_eq!(replay, first);
}
