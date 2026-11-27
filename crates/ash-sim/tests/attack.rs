//! Acceptance of spec 0002, slice S1: the character strikes on purpose.
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
use ash_core::time::ActionCost;
use ash_sim::action::{Action, AvailableAction, Refused};
use ash_sim::controller::Controller;
use ash_sim::perception::{Outcome, PerceivedChange};
use ash_sim::world::World;

const SEED: u64 = 17;

/// Character turns after which a fight that has not seen both a hit and a
/// miss fails.
const FIGHT_TURNS: usize = 2_000;

fn at(x: u32, y: u32) -> TilePosition {
    TilePosition { x, y, z: 0 }
}

/// The character, created first, and a wandering creature of speed 2 (R6)
/// at `start`.
fn world_with_creature(seed: u64, start: TilePosition) -> (World, EntityId, EntityId) {
    let mut world = World::new(seed, Speed::new(1));
    let character = world.awaited();
    let creature = world.create_entity(Speed::new(2), start, Controller::Wander);
    (world, character, creature)
}

/// The first attack offered to the awaited character, clockwise from the
/// north.
fn offered_attack(world: &World) -> Option<AvailableAction> {
    world
        .available_actions()
        .into_iter()
        .find(|offered| matches!(offered.action, Action::Attack(_)))
}

fn changes_of(world: &World, entity: EntityId) -> Vec<PerceivedChange> {
    world
        .perceived_changes()
        .iter()
        .copied()
        .filter(|change| change.entity == entity)
        .collect()
}

/// The character strikes whenever an attack is offered and waits
/// otherwise, until it has hit and missed. Every attack is offered at 100
/// (A5) and delays the character's next turn by 100.
fn fight(seed: u64) -> (World, EntityId, EntityId) {
    let (mut world, character, creature) = world_with_creature(seed, at(2, 1));
    let attacks = |world: &World| {
        changes_of(world, character)
            .into_iter()
            .filter(|change| matches!(change.action, Action::Attack(_)))
            .map(|change| change.outcome)
            .collect::<Vec<_>>()
    };
    for _ in 0..FIGHT_TURNS {
        let outcomes = attacks(&world);
        let hit = outcomes
            .iter()
            .any(|outcome| matches!(outcome, Outcome::Hit(_)));
        let missed = outcomes
            .iter()
            .any(|outcome| matches!(outcome, Outcome::Missed(_)));
        if hit && missed {
            return (world, character, creature);
        }
        let Some(offered) = offered_attack(&world) else {
            world.submit(Action::Wait).expect("a wait is never refused");
            continue;
        };
        assert_eq!(offered.cost, ActionCost::new(100), "{offered:?}");
        let clock = world.clock();
        assert_eq!(world.submit(offered.action), Ok(()), "{offered:?}");
        assert_eq!(
            (world.awaited(), world.clock().units() - clock.units()),
            (character, 100),
            "after {offered:?}"
        );
    }
    panic!(
        "no hit and miss in {FIGHT_TURNS} turns: {:?}",
        attacks(&world)
    );
}

#[test]
fn an_adjacent_entity_can_be_attacked_for_100() {
    // The creature stands east of the character, which acts first (R8).
    let (world, ..) = world_with_creature(SEED, at(2, 1));
    // A3, A5: offered with its cost (R9) toward the adjacent entity only.
    let attacks: Vec<_> = world
        .available_actions()
        .into_iter()
        .filter(|offered| matches!(offered.action, Action::Attack(_)))
        .collect();
    assert_eq!(
        attacks,
        [AvailableAction {
            action: Action::Attack(Direction::East),
            cost: ActionCost::new(100),
        }]
    );
}

#[test]
fn explicit_attacks_on_the_creature_are_perceived_hits_and_misses() {
    let (world, character, creature) = fight(SEED);
    let attacks: Vec<PerceivedChange> = changes_of(&world, character)
        .into_iter()
        .filter(|change| matches!(change.action, Action::Attack(_)))
        .collect();
    // A11: each attack is perceived as a hit or a miss naming its target.
    for change in &attacks {
        assert!(
            matches!(
                change.outcome,
                Outcome::Hit(target) | Outcome::Missed(target) if target == creature
            ),
            "{change:?}"
        );
    }
    // The other changes keep their outcomes: a wait is a wait.
    for change in changes_of(&world, character) {
        if change.action == Action::Wait {
            assert_eq!(change.outcome, Outcome::Waited, "{change:?}");
        }
    }
}

#[test]
fn a_refused_attack_after_the_wander_has_drawn_changes_nothing() {
    let (mut world, character, creature) = world_with_creature(SEED, at(7, 4));
    world.submit(Action::Wait).expect("a wait is never refused");
    assert!(
        !changes_of(&world, creature).is_empty(),
        "the wander has not drawn yet"
    );
    let untouched = world.clone();
    let offers = world.available_actions();
    assert_eq!(offered_attack(&world), None);

    // East is floor and holds no entity; north is a wall (A3, R10).
    for direction in [Direction::East, Direction::North] {
        assert_eq!(world.submit(Action::Attack(direction)), Err(Refused));
        // F4, R10: no RNG draw, no clock, no write.
        assert_eq!(world.clock(), untouched.clock());
        assert_eq!(world.available_actions(), offers);
        assert_eq!(world.command_log(), untouched.command_log());
        assert_eq!(world.perceived_changes(), untouched.perceived_changes());
        assert_eq!(world, untouched);
    }

    // The run that follows is the run without the refusals.
    let script = [Action::Move(Direction::East), Action::Wait, Action::Wait];
    let mut without = untouched;
    for action in script {
        assert_eq!(world.submit(action), Ok(()));
        assert_eq!(without.submit(action), Ok(()));
    }
    assert_eq!(world, without);
    assert_eq!(world.position(character), at(2, 1));
}

#[test]
fn the_wander_draws_only_among_the_offers_that_are_not_attacks() {
    // The creature starts at (3, 1). Its first turn, after the character's
    // move, offers it the same moves either way: a move into the character
    // is a bump (R13). Next to the character, it is offered an attack too
    // (R3). The wander ignores that offer before it draws, so the same
    // seed gives the same first action.
    for seed in 0..64 {
        let first_creature_action = |character_move| {
            let (mut world, _, creature) = world_with_creature(seed, at(3, 1));
            world
                .submit(Action::Move(character_move))
                .expect("a move from the start is never refused");
            changes_of(&world, creature)
                .first()
                .map(|change| change.action)
        };
        // East to (2, 1), next to the creature; south to (1, 2), two tiles
        // away.
        assert_eq!(
            first_creature_action(Direction::East),
            first_creature_action(Direction::South),
            "seed {seed}"
        );
    }
}

#[test]
fn the_wander_never_attacks_the_character_next_to_it() {
    let (mut world, character, creature) = world_with_creature(SEED, at(2, 1));
    for _ in 0..500 {
        world.submit(Action::Wait).expect("a wait is never refused");
    }
    let creature_changes = changes_of(&world, creature);
    assert!(
        creature_changes
            .iter()
            .any(|change| change.outcome == Outcome::Bumped(character)),
        "the seeded run never brings the creature next to the character"
    );
    assert!(
        creature_changes
            .iter()
            .all(|change| !matches!(change.action, Action::Attack(_))),
        "the wander chose an attack"
    );
}

#[test]
fn an_entity_of_another_controller_is_offered_the_same_attack() {
    // R3: the offers do not depend on the controller.
    let mut world = World::new(SEED, Speed::new(1));
    let other = world.create_entity(Speed::new(1), at(2, 1), Controller::Player);
    world.submit(Action::Wait).expect("a wait is never refused");
    assert_eq!(world.awaited(), other);
    assert_eq!(
        offered_attack(&world).map(|offered| offered.action),
        Some(Action::Attack(Direction::West))
    );
}

#[test]
fn the_fight_replays_identically() {
    let (first, ..) = fight(SEED);
    assert!(
        first
            .command_log()
            .iter()
            .any(|action| matches!(action, Action::Attack(_))),
        "the log holds no attack"
    );

    let (mut replay, ..) = world_with_creature(SEED, at(2, 1));
    for &action in first.command_log() {
        assert_eq!(replay.submit(action), Ok(()), "{action:?}");
    }
    assert_eq!(replay.perceived_changes(), first.perceived_changes());
    assert_eq!(replay, first);
}
