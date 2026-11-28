//! Acceptance of spec 0003, slice S5, in the simulation: a struck character
//! perceives its own hit points (A9), and every other character its
//! condition (A10).
//!
//! The fixed map, with x to the east and y to the south; the first
//! character of every world starts at (1, 1), the start tile:
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

use ash_core::entity::Speed;
use ash_core::tile::{Direction, TilePosition};
use ash_core::time::Time;
use ash_sim::action::Action;
use ash_sim::combat::Condition;
use ash_sim::controller::Controller;
use ash_sim::perception::{ChangeKind, Outcome, PerceivedChange};
use ash_sim::world::World;

/// A6: the first four draws of seed 2 hit, at 10, 26, 51 and 36 of 100.
const SEED: u64 = 2;

/// Turns after which a fight that has not ended fails.
const MOST_TURNS: usize = 100;

#[test]
fn the_struck_character_perceives_its_hit_points_and_the_striker_its_condition() {
    // The striker holds the start tile and strikes the character east of
    // it on each of its turns; the struck character waits on each of its
    // own. Both have speed 1: an attack costs 100, a wait 50 (R6, A5).
    let mut world = World::new(SEED, Speed::new(1));
    let striker = world.awaited();
    let east = TilePosition { x: 2, y: 1, z: 0 };
    let struck = world.create_entity(Speed::new(1), east, Controller::Player);
    for _ in 0..MOST_TURNS {
        if world.stranger_of(struck).is_some() {
            break;
        }
        let action = if world.awaited() == striker {
            Action::Attack(Direction::East)
        } else {
            Action::Wait
        };
        assert_eq!(world.submit(action), Ok(()), "{action:?}");
    }
    let Some(stranger) = world.stranger_of(struck) else {
        panic!("the struck character lives after {MOST_TURNS} turns");
    };

    let change = |time, entity, kind| PerceivedChange {
        time: Time::new(time),
        entity,
        kind,
    };
    let hit = |time| change(time, striker, ChangeKind::Act(Outcome::Hit(struck)));
    let waited = |time| change(time, struck, ChangeKind::Act(Outcome::Waited));
    let hit_points =
        |time, current| change(time, struck, ChangeKind::HitPoints { current, max: 10 });
    let condition = |time, condition| change(time, struck, ChangeKind::Condition(condition));
    let death = change(300, struck, ChangeKind::Death);
    // D8: the striker holds the start tile, and the struck character's tile
    // is the nearest free one.
    let arrival = change(300, stranger, ChangeKind::Arrival(east));

    // A9, A11: each hit the struck character survives, then its hit points.
    // The lethal hit adds none: the death follows it.
    assert_eq!(
        world.perceived_changes_of(struck),
        [
            hit(0),
            hit_points(0, 7),
            waited(0),
            waited(50),
            hit(100),
            hit_points(100, 4),
            waited(100),
            waited(150),
            hit(200),
            hit_points(200, 1),
            waited(200),
            waited(250),
            hit(300),
            death,
        ]
    );
    // A10, R18: the striker perceives each hit, then the condition it
    // leaves, and never a hit point.
    assert_eq!(
        world.perceived_changes_of(striker),
        [
            hit(0),
            condition(0, Condition::Wounded),
            waited(0),
            waited(50),
            hit(100),
            condition(100, Condition::BadlyWounded),
            waited(100),
            waited(150),
            hit(200),
            condition(200, Condition::Dying),
            waited(200),
            waited(250),
            hit(300),
            death,
            arrival,
        ]
    );
    // A11: the world holds both, the hit points right after the hit and
    // before the change of condition.
    assert_eq!(
        world.perceived_changes(),
        [
            hit(0),
            hit_points(0, 7),
            condition(0, Condition::Wounded),
            waited(0),
            waited(50),
            hit(100),
            hit_points(100, 4),
            condition(100, Condition::BadlyWounded),
            waited(100),
            waited(150),
            hit(200),
            hit_points(200, 1),
            condition(200, Condition::Dying),
            waited(200),
            waited(250),
            hit(300),
            death,
            arrival,
        ]
    );
    // D11: the stranger perceives nothing of the hits before its arrival.
    assert_eq!(world.perceived_changes_of(stranger), [arrival]);
}
