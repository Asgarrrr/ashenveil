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
use ash_sim::action::{Action, Offer, Refused};
use ash_sim::controller::{Controller, Disposition};
use ash_sim::perception::{ChangeKind, Outcome};
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
    let creature = world.create_entity(
        Speed::new(2),
        start,
        Controller::Wander(Disposition::Neutral),
    );
    (world, character, creature)
}

/// The first attack offered to the awaited character, clockwise from the
/// north.
fn offered_attack(world: &World) -> Option<Offer> {
    world
        .offers()
        .into_iter()
        .find(|offer| matches!(offer.action, Action::Attack(_)))
}

/// A11: an attack is perceived as a hit or a miss.
fn is_attack(outcome: Outcome) -> bool {
    matches!(outcome, Outcome::Hit(_) | Outcome::Missed(_))
}

/// What `entity` did, in order: the outcome of each of its acts.
fn outcomes_of(world: &World, entity: EntityId) -> Vec<Outcome> {
    world
        .perceived_changes()
        .iter()
        .filter(|change| change.entity == entity)
        .filter_map(|change| match change.kind {
            ChangeKind::Act(outcome) => Some(outcome),
            ChangeKind::Death | ChangeKind::Condition(_) => None,
        })
        .collect()
}

/// The character strikes whenever an attack is offered and waits
/// otherwise, until it has hit and missed. Every attack is offered at 100
/// (A5) and delays the character's next turn by 100.
///
/// The creature dies at its second hit (A6 to A8). A seed whose second hit
/// comes before its first miss cannot show a miss: the fight fails on it,
/// by name.
fn fight(seed: u64) -> (World, EntityId, EntityId) {
    let (mut world, character, creature) = world_with_creature(seed, at(2, 1));
    let attacks = |world: &World| {
        outcomes_of(world, character)
            .into_iter()
            .filter(|&outcome| is_attack(outcome))
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
        let died = world
            .perceived_changes()
            .iter()
            .any(|change| change.kind == ChangeKind::Death);
        assert!(
            !died,
            "seed {seed}: the creature died before the fight saw a miss"
        );
        let Some(offer) = offered_attack(&world) else {
            world.submit(Action::Wait).expect("a wait is never refused");
            continue;
        };
        assert_eq!(offer.cost, ActionCost::new(100), "{offer:?}");
        let clock = world.clock();
        assert_eq!(world.submit(offer.action), Ok(()), "{offer:?}");
        assert_eq!(
            (world.awaited(), world.clock().units() - clock.units()),
            (character, 100),
            "after {offer:?}"
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
        .offers()
        .into_iter()
        .filter(|offer| matches!(offer.action, Action::Attack(_)))
        .collect();
    assert_eq!(
        attacks,
        [Offer {
            action: Action::Attack(Direction::East),
            cost: ActionCost::new(100),
        }]
    );
}

#[test]
fn an_attacker_of_speed_2_pays_50() {
    // A5: the base cost of 100 is divided by the attacker's speed.
    let mut world = World::new(SEED, Speed::new(2));
    let character = world.awaited();
    world.create_entity(
        Speed::new(2),
        at(2, 1),
        Controller::Wander(Disposition::Neutral),
    );

    let offer = offered_attack(&world).expect("the adjacent creature can be attacked");
    assert_eq!(offer.cost, ActionCost::new(50));
    let clock = world.clock();
    assert_eq!(world.submit(offer.action), Ok(()));
    assert_eq!(
        (world.awaited(), world.clock().units() - clock.units()),
        (character, 50)
    );
}

#[test]
fn explicit_attacks_on_the_creature_are_perceived_hits_and_misses() {
    let (world, character, creature) = fight(SEED);
    let outcomes: Vec<Outcome> = outcomes_of(&world, character)
        .into_iter()
        .filter(|&outcome| is_attack(outcome))
        .collect();
    // A11: each attack is perceived once, as a hit or a miss naming its
    // target.
    let attacks = world
        .command_log()
        .iter()
        .filter(|action| matches!(action, Action::Attack(_)))
        .count();
    assert_eq!(outcomes.len(), attacks, "{outcomes:?}");
    for outcome in outcomes {
        assert!(
            matches!(outcome, Outcome::Hit(target) | Outcome::Missed(target) if target == creature),
            "{outcome:?}"
        );
    }
}

#[test]
fn a_refused_attack_after_the_wander_has_drawn_changes_nothing() {
    let (mut world, character, creature) = world_with_creature(SEED, at(7, 4));
    world.submit(Action::Wait).expect("a wait is never refused");
    assert!(
        !outcomes_of(&world, creature).is_empty(),
        "the wander has not drawn yet"
    );
    let untouched = world.clone();
    let offers = world.offers();
    assert_eq!(offered_attack(&world), None);

    // East is floor and holds no entity; north is a wall (A3, R10).
    for direction in [Direction::East, Direction::North] {
        assert_eq!(world.submit(Action::Attack(direction)), Err(Refused));
        // F4, R10: no RNG draw, no clock, no write.
        assert_eq!(world.clock(), untouched.clock());
        assert_eq!(world.offers(), offers);
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
            let (mut world, character, creature) = world_with_creature(seed, at(3, 1));
            world
                .submit(Action::Move(character_move))
                .expect("a move from the start is never refused");
            outcomes_of(&world, creature)
                .first()
                .map(|&outcome| match outcome {
                    // Next to the character, the move west bumps it.
                    Outcome::Bumped(bumped) if bumped == character => {
                        Outcome::Stepped(Direction::West)
                    }
                    outcome => outcome,
                })
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
    let creature_outcomes = outcomes_of(&world, creature);
    assert!(
        creature_outcomes.contains(&Outcome::Bumped(character)),
        "the seeded run never brings the creature next to the character"
    );
    assert!(
        creature_outcomes.iter().all(|&outcome| !is_attack(outcome)),
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
        offered_attack(&world).map(|offer| offer.action),
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
