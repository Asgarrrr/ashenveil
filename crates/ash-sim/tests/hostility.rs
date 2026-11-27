//! Acceptance of spec 0002, slice S2: a move into a hostile creature is an
//! attack.
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
use ash_core::time::{ActionCost, Time};
use ash_sim::action::{Action, Refused};
use ash_sim::combat::Disposition;
use ash_sim::controller::Controller;
use ash_sim::perception::{self, Outcome, PerceivedChange};
use ash_sim::world::World;

const SEED: u64 = 23;

/// Character turns after which a run that has not shown what it looks for
/// fails.
const RUN_TURNS: usize = 2_000;

fn at(x: u32, y: u32) -> TilePosition {
    TilePosition { x, y, z: 0 }
}

struct Cast {
    character: EntityId,
    hostile: EntityId,
    neutral: EntityId,
}

/// The character, created first, then a hostile and a neutral wandering
/// creature of speed 2 (R6).
fn world_with_creatures(seed: u64, hostile: TilePosition, neutral: TilePosition) -> (World, Cast) {
    let mut world = World::new(seed, Speed::new(1));
    let cast = Cast {
        character: world.awaited(),
        hostile: world.create_entity(
            Speed::new(2),
            hostile,
            Controller::Wander(Disposition::Hostile),
        ),
        neutral: world.create_entity(
            Speed::new(2),
            neutral,
            Controller::Wander(Disposition::Neutral),
        ),
    };
    (world, cast)
}

fn changes_of(world: &World, entity: EntityId) -> Vec<PerceivedChange> {
    world
        .perceived_changes()
        .iter()
        .copied()
        .filter(|change| change.entity == entity)
        .collect()
}

/// A11: an attack is perceived as a hit or a miss.
fn is_attack(outcome: Outcome) -> bool {
    matches!(outcome, Outcome::Hit(_) | Outcome::Missed(_))
}

/// The cost the awaited character is offered `action` for (R9).
fn offered_cost(world: &World, action: Action) -> Option<ActionCost> {
    world
        .available_actions()
        .into_iter()
        .find(|offered| offered.action == action)
        .map(|offered| offered.cost)
}

/// The direction of the tile next to `from` that `target` stands on.
fn direction_to(world: &World, from: EntityId, target: EntityId) -> Option<Direction> {
    Direction::ALL
        .into_iter()
        .find(|&direction| world.position(from).step(direction) == world.position(target))
}

/// The offered move that brings the awaited character closest to `target`:
/// the move onto its tile once adjacent. A wait when no move is offered.
fn move_toward(world: &World, target: EntityId) -> Action {
    let from = world.position(world.awaited());
    let target = world.position(target);
    let distance = |position: TilePosition| {
        position
            .x
            .abs_diff(target.x)
            .max(position.y.abs_diff(target.y))
    };
    world
        .available_actions()
        .into_iter()
        .filter_map(|offered| match offered.action {
            Action::Move(direction) => Some(direction),
            Action::Wait | Action::Attack(_) => None,
        })
        .min_by_key(|&direction| distance(from.step(direction)))
        .map_or(Action::Wait, Action::Move)
}

/// The character takes `action`, a move or an attack. The change it makes is
/// the one the rules give for what stands on the tile ahead, and it costs
/// 100 (R5, A5): a move into the hostile creature is an attack on it (A1), a
/// move into the neutral creature is a bump (A2), an attack strikes either
/// (A3).
fn act(world: &mut World, cast: &Cast, action: Action) -> Outcome {
    let (Action::Move(direction) | Action::Attack(direction)) = action else {
        panic!("{action:?} has no tile ahead");
    };
    let ahead = [cast.hostile, cast.neutral]
        .into_iter()
        .find(|&creature| direction_to(world, cast.character, creature) == Some(direction));
    let clock = world.clock();
    let seen = world.perceived_changes().len();

    assert_eq!(world.submit(action), Ok(()), "{action:?}");

    let change = world.perceived_changes()[seen];
    assert_eq!((change.time, change.entity), (clock, cast.character));
    let expected = match (action, ahead) {
        (Action::Move(_), Some(creature)) if creature == cast.neutral => {
            change.outcome == Outcome::Bumped(creature)
        }
        (_, Some(creature)) => {
            change.outcome == Outcome::Hit(creature) || change.outcome == Outcome::Missed(creature)
        }
        (_, None) => change.outcome == Outcome::Stepped(direction),
    };
    assert!(expected, "{action:?} toward {ahead:?} gave {change:?}");
    assert_eq!(
        (world.awaited(), world.clock().units() - clock.units()),
        (cast.character, 100),
        "after {action:?}"
    );
    change.outcome
}

/// The character moves into the hostile creature until it has hit and
/// missed, moves into the neutral creature until it bumps, then attacks the
/// neutral creature on purpose.
fn run(seed: u64) -> (World, Cast) {
    let (mut world, cast) = world_with_creatures(seed, at(7, 4), at(1, 4));

    let mut strikes = Vec::new();
    let hit_and_missed = |strikes: &[Outcome]| {
        strikes.contains(&Outcome::Hit(cast.hostile))
            && strikes.contains(&Outcome::Missed(cast.hostile))
    };
    for _ in 0..RUN_TURNS {
        if hit_and_missed(&strikes) {
            break;
        }
        let action = move_toward(&world, cast.hostile);
        if action == Action::Wait {
            world.submit(action).expect("a wait is never refused");
            continue;
        }
        let outcome = act(&mut world, &cast, action);
        strikes.extend(is_attack(outcome).then_some(outcome));
    }
    assert!(
        hit_and_missed(&strikes),
        "no hit and miss by a move in {RUN_TURNS} turns: {strikes:?}"
    );

    let mut bumped = false;
    for _ in 0..RUN_TURNS {
        if bumped {
            break;
        }
        let action = move_toward(&world, cast.neutral);
        if action == Action::Wait {
            world.submit(action).expect("a wait is never refused");
            continue;
        }
        bumped = act(&mut world, &cast, action) == Outcome::Bumped(cast.neutral);
    }
    assert!(bumped, "no bump into the neutral creature");

    for _ in 0..RUN_TURNS {
        let action = match direction_to(&world, cast.character, cast.neutral) {
            Some(direction) => Action::Attack(direction),
            None => move_toward(&world, cast.neutral),
        };
        if action == Action::Wait {
            world.submit(action).expect("a wait is never refused");
            continue;
        }
        act(&mut world, &cast, action);
        if matches!(action, Action::Attack(_)) {
            return (world, cast);
        }
    }
    panic!("no attack on the neutral creature in {RUN_TURNS} turns");
}

#[test]
fn a_character_perceives_the_disposition_of_each_creature() {
    let (mut world, cast) = world_with_creatures(SEED, at(7, 4), at(1, 4));
    let other = world.create_entity(Speed::new(1), at(4, 4), Controller::Player);

    // A4: a kind for each creature. A character has no disposition.
    let dispositions: Vec<_> = perception::snapshot(&world)
        .entities
        .iter()
        .map(|perceived| (perceived.entity, perceived.disposition))
        .collect();
    assert_eq!(
        dispositions,
        [
            (cast.character, None),
            (cast.hostile, Some(Disposition::Hostile)),
            (cast.neutral, Some(Disposition::Neutral)),
            (other, None),
        ]
    );
}

#[test]
fn a_move_into_the_hostile_creature_is_the_attack_on_it() {
    let into_creature = Direction::East;
    let mut outcomes = Vec::new();
    for seed in 0..32 {
        // The hostile creature stands east of the character, which acts
        // first (R8).
        let (mut moved, cast) = world_with_creatures(seed, at(2, 1), at(7, 4));
        let mut attacked = moved.clone();
        // R9, R14: the move is offered, at the cost of the attack it is
        // (A5).
        let cost = offered_cost(&moved, Action::Attack(into_creature));
        assert_eq!(cost, Some(ActionCost::new(100)));
        assert_eq!(offered_cost(&moved, Action::Move(into_creature)), cost);

        assert_eq!(moved.submit(Action::Move(into_creature)), Ok(()));
        assert_eq!(attacked.submit(Action::Attack(into_creature)), Ok(()));

        // A1: a hit or a miss of the creature, perceived at once (R7). The
        // mover stays in place and pays the attack.
        let change = moved.perceived_changes()[0];
        assert_eq!((change.time, change.entity), (Time::ZERO, cast.character));
        assert!(
            change.outcome == Outcome::Hit(cast.hostile)
                || change.outcome == Outcome::Missed(cast.hostile),
            "seed {seed}: {change:?}"
        );
        assert_eq!(moved.position(cast.character), at(1, 1));
        assert_eq!(
            (moved.awaited(), moved.clock()),
            (cast.character, Time::new(100)),
            "seed {seed}"
        );
        // The command log keeps what the controller submitted.
        assert_eq!(moved.command_log(), [Action::Move(into_creature)]);

        // The world after the move is the world after the attack, its RNG
        // included: the runs that follow are the same.
        for _ in 0..3 {
            assert_eq!(moved.perceived_changes(), attacked.perceived_changes());
            moved.submit(Action::Wait).expect("a wait is never refused");
            attacked
                .submit(Action::Wait)
                .expect("a wait is never refused");
        }
        assert_eq!(
            moved.perceived_changes(),
            attacked.perceived_changes(),
            "seed {seed}"
        );
        outcomes.push(change.outcome);
    }
    // A6: some moves hit, some miss.
    assert!(
        outcomes
            .iter()
            .any(|outcome| matches!(outcome, Outcome::Hit(_)))
            && outcomes
                .iter()
                .any(|outcome| matches!(outcome, Outcome::Missed(_))),
        "{outcomes:?}"
    );
}

#[test]
fn a_move_into_another_character_is_a_bump() {
    // A2: a character has no disposition, so it is never perceived as
    // hostile.
    let mut world = World::new(SEED, Speed::new(1));
    let character = world.awaited();
    let other = world.create_entity(Speed::new(1), at(2, 1), Controller::Player);

    assert_eq!(world.submit(Action::Move(Direction::East)), Ok(()));

    assert_eq!(
        world.perceived_changes(),
        [PerceivedChange {
            time: Time::ZERO,
            entity: character,
            outcome: Outcome::Bumped(other),
        }]
    );
}

#[test]
fn a_wandering_creature_bumps_a_hostile_creature_and_never_attacks_it() {
    // A13: only the character attacks. The two creatures start side by side
    // in the east corridor.
    let (mut world, cast) = world_with_creatures(SEED, at(7, 2), at(7, 3));
    for _ in 0..500 {
        world.submit(Action::Wait).expect("a wait is never refused");
    }

    let neutral_changes = changes_of(&world, cast.neutral);
    assert!(
        neutral_changes
            .iter()
            .any(|change| change.outcome == Outcome::Bumped(cast.hostile)),
        "the seeded run never moves the neutral creature into the hostile one"
    );
    assert!(
        world
            .perceived_changes()
            .iter()
            .all(|change| !is_attack(change.outcome)),
        "a creature attacked"
    );
}

#[test]
fn a_diagonal_move_past_a_wall_corner_into_the_hostile_creature_is_refused() {
    // (5, 2) and (6, 3) are walls: R11 refuses the move from (5, 3) to
    // (6, 2), whatever stands there. A refused move is not an action (R10),
    // so it is not an attack either.
    let mut world = World::new(SEED, Speed::new(1));
    let mover = world.create_entity(Speed::new(1), at(5, 3), Controller::Player);
    world.create_entity(
        Speed::new(2),
        at(6, 2),
        Controller::Wander(Disposition::Hostile),
    );
    // The mover's turn comes before the first turn of the creature (R8).
    world.submit(Action::Wait).expect("a wait is never refused");
    assert_eq!(world.awaited(), mover);
    let before = world.clone();
    let past_the_corner = Action::Move(Direction::NorthEast);
    assert_eq!(offered_cost(&world, past_the_corner), None);

    assert_eq!(world.submit(past_the_corner), Err(Refused));

    assert_eq!(world, before);
}

#[test]
fn the_run_replays_identically() {
    let (first, _) = run(SEED);
    // The character struck the hostile creature by moves alone: the log
    // keeps the moves it submitted, and holds one attack, the last command.
    let attacks: Vec<usize> = first
        .command_log()
        .iter()
        .enumerate()
        .filter(|(_, action)| matches!(action, Action::Attack(_)))
        .map(|(index, _)| index)
        .collect();
    assert_eq!(attacks, [first.command_log().len() - 1]);

    let (mut replay, _) = world_with_creatures(SEED, at(7, 4), at(1, 4));
    for &action in first.command_log() {
        assert_eq!(replay.submit(action), Ok(()), "{action:?}");
    }
    assert_eq!(replay.perceived_changes(), first.perceived_changes());
    assert_eq!(replay, first);
}
