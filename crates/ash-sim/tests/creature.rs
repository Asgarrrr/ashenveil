//! Acceptance of spec 0001, slice S7: the creature.
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
use ash_sim::controller::{Controller, Disposition};
use ash_sim::perception::{ChangeKind, Outcome, PerceivedChange};
use ash_sim::world::World;

const SEED: u64 = 13;

fn at(x: u32, y: u32) -> TilePosition {
    TilePosition { x, y, z: 0 }
}

/// The character, created first, and a wandering creature of speed 2 (R6)
/// at `start`.
fn world_with_creature(seed: u64, start: TilePosition) -> (World, EntityId, EntityId) {
    let mut world = World::new(seed, Speed::new(1));
    let character = world.awaited();
    let creature = world.create_entity(
        Speed::new(2),
        start,
        Controller::Wander(Disposition::Neutral),
    );
    (world, character, creature)
}

/// The first move offered to the awaited character, clockwise from the
/// north.
fn first_offered_move(world: &World) -> Action {
    world
        .offers()
        .into_iter()
        .map(|offer| offer.action)
        .find(|action| matches!(action, Action::Move(_)))
        .expect("a move is offered")
}

/// One act: when, who, and what it did.
type Act = (Time, EntityId, Outcome);

/// Every perceived change of `world`, in order: no entity is struck here,
/// so each is an act.
fn acts(world: &World) -> Vec<Act> {
    world
        .perceived_changes()
        .iter()
        .map(|&change| match change.kind {
            ChangeKind::Act(outcome) => (change.time, change.entity, outcome),
            ChangeKind::Death | ChangeKind::Condition(_) => panic!("unexpected {change:?}"),
        })
        .collect()
}

/// When `entity` did what, in order.
fn acts_of(world: &World, entity: EntityId) -> Vec<(Time, Outcome)> {
    acts(world)
        .into_iter()
        .filter(|&(_, actor, _)| actor == entity)
        .map(|(time, _, outcome)| (time, outcome))
        .collect()
}

#[test]
fn at_creation_the_character_acts_first_then_the_creature() {
    let (mut world, character, creature) = world_with_creature(SEED, at(7, 4));
    assert_eq!((world.awaited(), world.clock()), (character, Time::ZERO));
    assert_eq!(world.perceived_changes(), []);

    world.submit(Action::Wait).expect("a wait is never refused");

    // R8: both are due at time zero; the one created first acts first.
    let first_two: Vec<_> = acts(&world)
        .into_iter()
        .take(2)
        .map(|(time, entity, _)| (time, entity))
        .collect();
    assert_eq!(first_two, [(Time::ZERO, character), (Time::ZERO, creature)]);
}

#[test]
fn the_creature_moves_twice_per_character_move_for_50_and_waits_for_25() {
    let (mut world, character, creature) = world_with_creature(SEED, at(7, 4));
    let character_moves = 12;
    for _ in 0..character_moves {
        let action = first_offered_move(&world);
        world
            .submit(action)
            .expect("an offered move is never refused");
    }

    let character_times: Vec<u64> = acts_of(&world, character)
        .iter()
        .map(|(time, _)| time.units())
        .collect();
    let every_100: Vec<u64> = (0..character_moves).map(|turn| turn * 100).collect();
    assert_eq!(character_times, every_100);

    // R6: the time from one creature action to its next is the cost it paid.
    let creature_acts = acts_of(&world, creature);
    for pair in creature_acts.windows(2) {
        let paid = pair[1].0.units() - pair[0].0.units();
        let cost = match pair[0].1 {
            Outcome::Waited => 25,
            Outcome::Stepped(_) | Outcome::Bumped(_) | Outcome::Hit(_) | Outcome::Missed(_) => 50,
        };
        assert_eq!(paid, cost, "after {:?}", pair[0]);
    }
    assert!(
        creature_acts
            .iter()
            .any(|&(_, outcome)| outcome == Outcome::Waited),
        "the seeded run shows no creature wait"
    );

    // Between two character moves, a creature that only moves moves twice.
    let per_character_move: Vec<Vec<Outcome>> = every_100
        .iter()
        .map(|&start| {
            creature_acts
                .iter()
                .filter(|(time, _)| (start..start + 100).contains(&time.units()))
                .map(|&(_, outcome)| outcome)
                .collect()
        })
        .collect();
    let only_moves: Vec<usize> = per_character_move
        .iter()
        .filter(|outcomes| {
            outcomes
                .iter()
                .all(|outcome| matches!(outcome, Outcome::Stepped(_) | Outcome::Bumped(_)))
        })
        .map(Vec::len)
        .collect();
    assert!(
        !only_moves.is_empty(),
        "the seeded run never has the creature only move: {per_character_move:?}"
    );
    assert!(
        only_moves.iter().all(|&moves| moves == 2),
        "{per_character_move:?}"
    );

    // R8: at a shared time, the character, due first, acts first.
    for (time, outcome) in acts_of(&world, character) {
        let first_then = acts(&world).into_iter().find(|&(other, ..)| other == time);
        assert_eq!(first_then, Some((time, character, outcome)));
    }
}

#[test]
fn a_bump_leaves_the_mover_in_place_costs_the_move_and_nothing_else() {
    // The creature stands east of the character, which acts first (R8).
    let (mut bumped, character, creature) = world_with_creature(SEED, at(2, 1));
    let (mut waited, ..) = world_with_creature(SEED, at(2, 1));

    // R13, R14: a move into an occupied tile is offered, and accepted.
    let into_creature = Action::Move(Direction::East);
    assert!(
        bumped
            .offers()
            .iter()
            .any(|offer| offer.action == into_creature)
    );
    assert_eq!(bumped.submit(into_creature), Ok(()));

    // The change both entities perceive names both (R13, R16).
    assert_eq!(
        bumped.perceived_changes().first(),
        Some(&PerceivedChange {
            time: Time::ZERO,
            entity: character,
            kind: ChangeKind::Act(Outcome::Bumped(creature)),
        })
    );
    // The mover stays in place and pays the move: its next turn is at 100.
    assert_eq!(bumped.position(character), at(1, 1));
    assert_eq!(
        (bumped.awaited(), bumped.clock()),
        (character, Time::new(100))
    );

    // Nothing else changes: where the character instead waits twice, also
    // in place for 100, the creature does exactly the same.
    waited
        .submit(Action::Wait)
        .expect("a wait is never refused");
    waited
        .submit(Action::Wait)
        .expect("a wait is never refused");
    assert_eq!(waited.clock(), bumped.clock());
    assert_eq!(waited.position(creature), bumped.position(creature));
    assert_eq!(acts_of(&waited, creature), acts_of(&bumped, creature));
}

#[test]
fn the_creature_wanders_only_through_actions_the_rules_allow() {
    let (mut world, character, creature) = world_with_creature(SEED, at(7, 4));
    // A refused choice would crash the world through `invariant!` (R14).
    for _ in 0..500 {
        world.submit(Action::Wait).expect("a wait is never refused");
        assert_ne!(world.position(creature), world.position(character));
    }

    let creature_acts = acts_of(&world, creature);
    let moves = creature_acts
        .iter()
        .filter(|(_, outcome)| matches!(outcome, Outcome::Stepped(_) | Outcome::Bumped(_)))
        .count();
    assert!(moves > 100, "the creature hardly wanders: {moves} moves");
    // The creature bumps the character too: the bump names it, and the
    // character stays where it waits.
    assert!(
        creature_acts
            .iter()
            .any(|&(_, outcome)| outcome == Outcome::Bumped(character)),
        "the seeded run shows no bump into the character"
    );
    assert_eq!(world.position(character), at(1, 1));
}

#[test]
fn a_seeded_run_replays_identically_and_another_seed_differs() {
    let run = |seed| {
        let (mut world, ..) = world_with_creature(seed, at(7, 4));
        for _ in 0..10 {
            let action = first_offered_move(&world);
            world
                .submit(action)
                .expect("an offered move is never refused");
            world.submit(Action::Wait).expect("a wait is never refused");
        }
        world
    };
    let first = run(SEED);
    // The log holds the character's commands only: the RNG replays the
    // creature.
    assert_eq!(first.command_log().len(), 20);

    let (mut replay, ..) = world_with_creature(SEED, at(7, 4));
    for &action in first.command_log() {
        assert_eq!(replay.submit(action), Ok(()));
    }
    assert_eq!(replay.perceived_changes(), first.perceived_changes());
    assert_eq!(replay, first);

    assert_ne!(
        run(SEED + 1).perceived_changes(),
        first.perceived_changes(),
        "the creature ignores the seed"
    );
}
