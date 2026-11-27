//! Acceptance of spec 0002, slice S3: the creature dies.
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
use ash_core::rng::Rng;
use ash_core::tile::{Direction, TilePosition};
use ash_core::time::Time;
use ash_sim::action::Action;
use ash_sim::combat::Condition;
use ash_sim::controller::{Controller, Disposition};
use ash_sim::perception::{self, ChangeKind, Outcome, PerceivedChange};
use ash_sim::world::World;

const SEED: u64 = 29;

/// Character turns after which a hunt that has not killed its target fails.
const HUNT_TURNS: usize = 2_000;

fn at(x: u32, y: u32) -> TilePosition {
    TilePosition { x, y, z: 0 }
}

/// The entities the awaited character perceives, in the order of the
/// snapshot.
fn embodied(world: &World) -> Vec<EntityId> {
    perception::snapshot(world, world.awaited())
        .entities
        .iter()
        .map(|perceived| perceived.entity)
        .collect()
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

/// True when `change` names `entity`, as the one that acts, is acted on,
/// dies, or changes condition. The patterns name every field, so a new one
/// fails to compile here.
fn names(change: PerceivedChange, entity: EntityId) -> bool {
    let PerceivedChange {
        time: _,
        entity: named,
        kind,
    } = change;
    named == entity
        || match kind {
            ChangeKind::Act(Outcome::Stepped(_) | Outcome::Waited)
            | ChangeKind::Death
            | ChangeKind::Condition(_) => false,
            ChangeKind::Act(
                Outcome::Bumped(target) | Outcome::Hit(target) | Outcome::Missed(target),
            ) => target == entity,
        }
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

/// A10: the change the first hit on `entity` makes at time zero: it comes
/// from unhurt to wounded.
fn wounded(entity: EntityId) -> PerceivedChange {
    PerceivedChange {
        time: Time::ZERO,
        entity,
        kind: ChangeKind::Condition(Condition::Wounded),
    }
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
        .available_actions()
        .into_iter()
        .filter_map(|offered| match offered.action {
            Action::Move(direction) => Some(direction),
            Action::Wait | Action::Attack(_) => None,
        })
        .min_by_key(|&direction| distance(from.step(direction)))
        .map_or(Action::Wait, Action::Move)
}

/// The character strikes `target` until it leaves the world.
fn hunt(world: &mut World, target: EntityId) {
    for _ in 0..HUNT_TURNS {
        let Some(position) = standing(world, target) else {
            return;
        };
        let action = strike_or_chase(world, position);
        assert_eq!(world.submit(action), Ok(()), "{action:?}");
    }
    panic!("{target:?} is still in the world after {HUNT_TURNS} turns");
}

/// A6: whether each successive draw of a world seeded with `seed` makes a
/// strike hit.
fn strikes(seed: u64) -> impl Iterator<Item = bool> {
    let mut rng = Rng::new(seed);
    core::iter::repeat_with(move || rng.next_u64() % 100 < 70)
}

/// The seeds whose first two strikes hit.
fn seeds_of_two_hits() -> impl Iterator<Item = u64> {
    (0..200).filter(|&seed| strikes(seed).take(2).all(|hit| hit))
}

/// Two characters stand west and east of a hostile creature; a third
/// character, when asked, stands further east. All are due at time zero in
/// creation order (R8): the two characters, the creature, then the third.
struct Ambush {
    first: EntityId,
    second: EntityId,
    creature: EntityId,
    third: Option<EntityId>,
}

/// The two characters strike the creature before its first turn: the first
/// by a move into it (A1), the second by `lethal`. On a seed of two hits, it
/// dies at time zero.
fn ambush(seed: u64, lethal: Action, with_third: bool) -> (World, Ambush) {
    let mut world = World::new(seed, Speed::new(1));
    let cast = Ambush {
        first: world.awaited(),
        second: world.create_entity(Speed::new(1), at(3, 1), Controller::Player),
        creature: world.create_entity(
            Speed::new(2),
            at(2, 1),
            Controller::Wander(Disposition::Hostile),
        ),
        third: with_third.then(|| world.create_entity(Speed::new(1), at(4, 1), Controller::Player)),
    };
    assert_eq!(world.submit(Action::Move(Direction::East)), Ok(()));
    assert_eq!(world.awaited(), cast.second);
    assert_eq!(world.submit(lethal), Ok(()));
    (world, cast)
}

/// The first character strikes the second, which stands east of it and
/// waits, until `hits` strikes have hit.
fn strike_a_character(seed: u64, hits: usize) -> (World, EntityId) {
    let mut world = World::new(seed, Speed::new(1));
    let attacker = world.awaited();
    let target = world.create_entity(Speed::new(2), at(2, 1), Controller::Player);
    let landed = |world: &World| {
        world
            .perceived_changes()
            .iter()
            .filter(|change| {
                (change.entity, change.kind) == (attacker, ChangeKind::Act(Outcome::Hit(target)))
            })
            .count()
    };
    let mut turns = 0;
    while landed(&world) < hits {
        assert!(
            turns < HUNT_TURNS,
            "fewer than {hits} hits on {target:?} in {HUNT_TURNS} turns"
        );
        turns += 1;
        let action = if world.awaited() == attacker {
            Action::Attack(Direction::East)
        } else {
            Action::Wait
        };
        assert_eq!(world.submit(action), Ok(()), "{action:?}");
    }
    (world, target)
}

#[test]
fn a_creature_dies_at_its_second_hit_and_its_death_is_perceived_right_after() {
    let mut misses_per_hunt = Vec::new();
    for seed in 0..24 {
        // The hostile creature, then the neutral one: an attack strikes
        // either (A3), and both have 6 hit points (A7).
        for prey in 0..2 {
            let (mut world, character, creatures) = world_with_creatures(seed);
            let target = creatures[prey];
            let survivor = creatures[1 - prey];

            hunt(&mut world, target);

            let changes = world.perceived_changes();
            let strike = |outcome| {
                changes
                    .iter()
                    .enumerate()
                    .filter(move |(_, change)| {
                        (change.entity, change.kind) == (character, ChangeKind::Act(outcome))
                    })
                    .map(|(index, _)| index)
                    .collect::<Vec<_>>()
            };
            // A6, A7: 6 hit points, less 3 at each hit. A miss removes none.
            let hits = strike(Outcome::Hit(target));
            assert_eq!(hits.len(), 2, "seed {seed}, prey {prey}: {changes:?}");
            misses_per_hunt.push(strike(Outcome::Missed(target)).len());

            // A8, A11: the death is the change right after the lethal hit,
            // at the same time, and the only death.
            let lethal = hits[1];
            let time = changes[lethal].time;
            let deaths: Vec<(usize, PerceivedChange)> = changes
                .iter()
                .copied()
                .enumerate()
                .filter(|(_, change)| change.kind == ChangeKind::Death)
                .collect();
            assert_eq!(
                deaths,
                [(
                    lethal + 1,
                    PerceivedChange {
                        time,
                        entity: target,
                        kind: ChangeKind::Death,
                    }
                )],
                "seed {seed}, prey {prey}"
            );
            // The first hit left the creature in the world: the second hit
            // names it.
            assert!(hits[0] < lethal);

            // A8: it left the world. The others are perceived as before.
            assert_eq!(
                embodied(&world),
                [character, survivor],
                "seed {seed}, prey {prey}"
            );
        }
    }
    // The hunts differ: a creature dies with no miss, and after misses.
    assert!(misses_per_hunt.contains(&0), "{misses_per_hunt:?}");
    assert!(
        misses_per_hunt.iter().any(|&misses| misses > 1),
        "{misses_per_hunt:?}"
    );
}

#[test]
fn each_kill_in_one_world_removes_the_creature_it_names_and_is_perceived() {
    let (mut world, character, [hostile, neutral]) = world_with_creatures(SEED);
    let third = world.create_entity(
        Speed::new(2),
        at(7, 1),
        Controller::Wander(Disposition::Neutral),
    );
    // In creation order: after the first death, the id of a creature is
    // no longer its rank among the entities.
    let prey = [hostile, neutral, third];
    for (killed, &target) in prey.iter().enumerate() {
        hunt(&mut world, target);

        // A11: one death for each kill, in order.
        assert_eq!(deaths(&world), prey[..=killed], "kill {killed}");
        // A8: the snapshot holds the character, then the survivors.
        let mut living = vec![character];
        living.extend(&prey[killed + 1..]);
        assert_eq!(embodied(&world), living, "kill {killed}");
    }
}

#[test]
fn the_world_runs_on_past_the_scheduler_entry_of_the_dead_creature() {
    for seed in 0..8 {
        let (mut world, character, [hostile, neutral]) = world_with_creatures(seed);
        hunt(&mut world, hostile);
        let seen = world.perceived_changes().len();
        let died_at = world.clock();

        // The creature acted for 25 or 50 (R6): its entry is due before the
        // character has waited twice.
        let waits: u64 = 40;
        for _ in 0..waits {
            assert_eq!(world.submit(Action::Wait), Ok(()));
            // The world never awaits the dead creature.
            assert_eq!(world.awaited(), character, "seed {seed}");
        }

        // R7: the clock ran at the character's cost alone.
        assert_eq!(
            world.clock().units() - died_at.units(),
            waits * 50,
            "seed {seed}"
        );
        let after = &world.perceived_changes()[seen..];
        // The dead creature acts nothing and is never named again.
        assert!(
            after.iter().all(|&change| !names(change, hostile)),
            "seed {seed}: {after:?}"
        );
        // R6: the other creature keeps its turns: it moves for 50 and
        // waits for 25, from one of its acts to the next.
        let turns: Vec<(Time, Outcome)> = after
            .iter()
            .filter(|change| change.entity == neutral)
            .filter_map(|change| match change.kind {
                ChangeKind::Act(outcome) => Some((change.time, outcome)),
                ChangeKind::Death | ChangeKind::Condition(_) => None,
            })
            .collect();
        assert!(turns.len() > 30, "seed {seed}: {} turns", turns.len());
        for pair in turns.windows(2) {
            let cost = if pair[0].1 == Outcome::Waited { 25 } else { 50 };
            assert_eq!(
                pair[1].0.units() - pair[0].0.units(),
                cost,
                "seed {seed}: after {:?}",
                pair[0]
            );
        }
    }
}

#[test]
fn at_a_tie_the_entry_of_the_dead_creature_acts_nothing_and_draws_nothing() {
    let mut third_and_fourth_draws = Vec::new();
    for seed in seeds_of_two_hits() {
        let (mut world, cast) = ambush(seed, Action::Attack(Direction::West), true);
        let third = cast.third.expect("the ambush has a third character");

        // A8, A10, A11: both strikes hit at time zero. The first wounds the
        // creature, the second is lethal.
        let hit_by = |entity| PerceivedChange {
            time: Time::ZERO,
            entity,
            kind: ChangeKind::Act(Outcome::Hit(cast.creature)),
        };
        let killed = [
            hit_by(cast.first),
            wounded(cast.creature),
            hit_by(cast.second),
            PerceivedChange {
                time: Time::ZERO,
                entity: cast.creature,
                kind: ChangeKind::Death,
            },
        ];
        assert_eq!(world.perceived_changes(), killed, "seed {seed}");

        // R8: the entry of the creature is due at zero, before the entry of
        // the third character. The world passes it: nothing acts, and the
        // clock stays.
        assert_eq!(
            (world.awaited(), world.clock()),
            (third, Time::ZERO),
            "seed {seed}"
        );

        // Nothing drew either: the strike of the third character, on the
        // second, takes the third draw of the seed. A hit wounds the second
        // character (A10).
        assert_eq!(world.submit(Action::Attack(Direction::West)), Ok(()));
        let mut draws = strikes(seed).skip(2);
        let (third_draw, fourth_draw) = (draws.next(), draws.next());
        let struck = |outcome| PerceivedChange {
            time: Time::ZERO,
            entity: third,
            kind: ChangeKind::Act(outcome),
        };
        let strike = if third_draw == Some(true) {
            vec![struck(Outcome::Hit(cast.second)), wounded(cast.second)]
        } else {
            vec![struck(Outcome::Missed(cast.second))]
        };
        assert_eq!(
            world.perceived_changes()[killed.len()..],
            strike,
            "seed {seed}"
        );
        third_and_fourth_draws.push((third_draw, fourth_draw));
    }
    // The seeds tell a third draw from a fourth, both ways.
    for hits in [true, false] {
        assert!(
            third_and_fourth_draws.contains(&(Some(hits), Some(!hits))),
            "{third_and_fourth_draws:?}"
        );
    }
}

#[test]
fn at_a_tie_with_a_living_creature_the_entry_of_the_dead_creature_draws_nothing() {
    let mut fourth_and_fifth_draws = Vec::new();
    for seed in seeds_of_two_hits() {
        // Due at time zero in creation order (R8): two characters, the
        // creature they kill, a second creature, then a third character.
        let mut world = World::new(seed, Speed::new(1));
        let second = world.create_entity(Speed::new(1), at(3, 1), Controller::Player);
        let dead = world.create_entity(
            Speed::new(2),
            at(2, 1),
            Controller::Wander(Disposition::Hostile),
        );
        let living = world.create_entity(
            Speed::new(2),
            at(7, 4),
            Controller::Wander(Disposition::Neutral),
        );
        let third = world.create_entity(Speed::new(1), at(4, 1), Controller::Player);
        assert_eq!(world.submit(Action::Move(Direction::East)), Ok(()));
        assert_eq!(world.submit(Action::Attack(Direction::West)), Ok(()));
        assert_eq!(deaths(&world), [dead], "seed {seed}");

        // The world passed the entry of the dead creature, then the living
        // creature took its turn: the wander drew once, the third draw. Four
        // changes come before its act: the two hits, the change of condition
        // after the first (A10) and the death.
        assert_eq!(
            (world.awaited(), world.clock()),
            (third, Time::ZERO),
            "seed {seed}"
        );
        let changes = world.perceived_changes();
        assert!(
            matches!(
                changes[4..],
                [PerceivedChange { entity, kind: ChangeKind::Act(_), .. }] if entity == living
            ),
            "seed {seed}: {changes:?}"
        );

        // The entry drew nothing: the strike of the third character, on the
        // second, takes the fourth draw of the seed.
        assert_eq!(world.submit(Action::Attack(Direction::West)), Ok(()));
        let mut draws = strikes(seed).skip(3);
        let (fourth_draw, fifth_draw) = (draws.next(), draws.next());
        let outcome = if fourth_draw == Some(true) {
            Outcome::Hit(second)
        } else {
            Outcome::Missed(second)
        };
        assert_eq!(
            world.perceived_changes()[5],
            PerceivedChange {
                time: Time::ZERO,
                entity: third,
                kind: ChangeKind::Act(outcome),
            },
            "seed {seed}"
        );
        fourth_and_fifth_draws.push((fourth_draw, fifth_draw));
    }
    // The seeds tell a fourth draw from a fifth, both ways.
    for hits in [true, false] {
        assert!(
            fourth_and_fifth_draws.contains(&(Some(hits), Some(!hits))),
            "{fourth_and_fifth_draws:?}"
        );
    }
}

#[test]
fn a_lethal_hit_made_by_a_move_kills_like_an_explicit_attack() {
    let seed = seeds_of_two_hits().next().expect("a seed of two hits");
    let (by_attack, _) = ambush(seed, Action::Attack(Direction::West), false);
    // A1: the second character moves into the hostile creature instead.
    let (by_move, cast) = ambush(seed, Action::Move(Direction::West), false);

    // A8, A11: the same hit, then the same death, at the same time.
    assert_eq!(deaths(&by_move), [cast.creature]);
    assert_eq!(by_move.perceived_changes(), by_attack.perceived_changes());
    assert_eq!(embodied(&by_move), [cast.first, cast.second]);
}

#[test]
fn the_tile_of_the_dead_creature_is_free() {
    let seed = seeds_of_two_hits().next().expect("a seed of two hits");
    let (mut world, cast) = ambush(seed, Action::Attack(Direction::West), false);
    assert_eq!(embodied(&world), [cast.first, cast.second]);

    // A3: no entity stands east of the first character, so no attack is
    // offered there. Its move east is a step (R4), not a bump.
    assert_eq!(world.awaited(), cast.first);
    let offered: Vec<Action> = world
        .available_actions()
        .into_iter()
        .map(|offer| offer.action)
        .collect();
    assert!(
        !offered.contains(&Action::Attack(Direction::East)),
        "{offered:?}"
    );
    let seen = world.perceived_changes().len();
    assert_eq!(world.submit(Action::Move(Direction::East)), Ok(()));

    assert_eq!(
        world.perceived_changes()[seen],
        PerceivedChange {
            time: Time::new(100),
            entity: cast.first,
            kind: ChangeKind::Act(Outcome::Stepped(Direction::East)),
        }
    );
    assert_eq!(world.position(cast.first), at(2, 1));
}

#[test]
fn an_entity_created_after_a_death_gets_an_id_never_issued_before() {
    let seed = seeds_of_two_hits().next().expect("a seed of two hits");
    // F2: the dead creature is the last entity created, then not the last.
    // A count of the entities would give the id of the dead creature, then
    // the id of the third character.
    for with_third in [false, true] {
        let (mut world, cast) = ambush(seed, Action::Attack(Direction::West), with_third);
        let living: Vec<EntityId> = [cast.first, cast.second]
            .into_iter()
            .chain(cast.third)
            .collect();
        assert_eq!(embodied(&world), living);

        // The newcomer stands where the creature died.
        let wander = Controller::Wander(Disposition::Neutral);
        let newcomer = world.create_entity(Speed::new(2), at(2, 1), wander);
        let another = world.create_entity(Speed::new(2), at(6, 1), wander);

        let mut issued = living.clone();
        issued.extend([cast.creature, newcomer, another]);
        for (index, id) in issued.iter().enumerate() {
            assert!(
                !issued[..index].contains(id),
                "third {with_third}: {id:?} is issued twice in {issued:?}"
            );
        }
        let mut perceived = living;
        perceived.extend([newcomer, another]);
        assert_eq!(embodied(&world), perceived);
        assert_eq!(standing(&world, newcomer), Some(at(2, 1)));

        // The newcomers take their turns: the world runs past the entry of
        // the dead creature, to them.
        let mut turns = 0;
        while world.clock().units() < 200 {
            assert!(
                turns < HUNT_TURNS,
                "the clock is below 200 after {HUNT_TURNS} turns"
            );
            turns += 1;
            assert_eq!(world.submit(Action::Wait), Ok(()));
        }
        for creature in [newcomer, another] {
            assert!(
                world.perceived_changes().iter().any(|change| {
                    change.entity == creature && matches!(change.kind, ChangeKind::Act(_))
                }),
                "third {with_third}: {creature:?} never acts"
            );
        }
        // The dead creature is named by the two hits, the change of
        // condition after the first (A10) and the death, then never again.
        assert!(
            world.perceived_changes()[..4]
                .iter()
                .all(|&change| names(change, cast.creature))
        );
        assert!(
            world.perceived_changes()[4..]
                .iter()
                .all(|&change| !names(change, cast.creature))
        );
    }
}

#[test]
fn a_character_survives_three_hits() {
    // A7: 10 hit points, less 3 at each hit (A6).
    for seed in 0..8 {
        let (world, target) = strike_a_character(seed, 3);
        assert_eq!(standing(&world, target), Some(at(2, 1)), "seed {seed}");
        assert_eq!(deaths(&world), [], "seed {seed}");
    }
}

#[test]
#[should_panic(expected = "reaches zero hit points")]
fn the_fourth_hit_on_a_character_is_a_violated_invariant() {
    // Spec 0002, Multiplayer impact: the death of a character is out of
    // scope, so the hit that would kill one crashes its world.
    let _ = strike_a_character(SEED, 4);
}

#[test]
fn the_run_replays_identically() {
    let run = |seed| {
        let (mut world, _, [hostile, _]) = world_with_creatures(seed);
        hunt(&mut world, hostile);
        for _ in 0..10 {
            assert_eq!(world.submit(Action::Wait), Ok(()));
        }
        world
    };
    let first = run(SEED);
    let deaths = first
        .perceived_changes()
        .iter()
        .filter(|change| change.kind == ChangeKind::Death)
        .count();
    assert_eq!(deaths, 1);

    let (mut replay, ..) = world_with_creatures(SEED);
    for &action in first.command_log() {
        assert_eq!(replay.submit(action), Ok(()), "{action:?}");
    }
    assert_eq!(replay.perceived_changes(), first.perceived_changes());
    assert_eq!(replay, first);
}
