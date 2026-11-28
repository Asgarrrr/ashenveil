//! Acceptance of spec 0003, slice S3: a character dies and a stranger
//! arrives, in the simulation.
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

use ash_core::entity::{EntityId, Speed};
use ash_core::rng::Rng;
use ash_core::tile::{Direction, TilePosition};
use ash_core::time::{ActionCost, Time};
use ash_sim::action::{Action, Offer};
use ash_sim::combat::Condition;
use ash_sim::controller::{Controller, Disposition};
use ash_sim::death::Died;
use ash_sim::perception::{
    self, ChangeKind, Outcome, PerceivedChange, PerceivedCorpse, PerceivedEntity, PerceivedHealth,
    Snapshot,
};
use ash_sim::world::World;

/// Turns after which a fight that has not ended fails.
const MOST_TURNS: usize = 2_000;

/// Where the first character of every world starts (D8).
const START: TilePosition = TilePosition { x: 1, y: 1, z: 0 };

fn at(x: u32, y: u32) -> TilePosition {
    TilePosition { x, y, z: 0 }
}

/// A6: whether each successive draw of a world seeded with `seed` makes a
/// strike hit.
fn strikes(seed: u64) -> impl Iterator<Item = bool> {
    let mut rng = Rng::new(seed);
    core::iter::repeat_with(move || rng.next_u64() % 100 < 70)
}

/// The first seed whose first `hits` strikes all hit.
fn seed_of_hits(hits: usize) -> u64 {
    (0..1_000)
        .find(|&seed| strikes(seed).take(hits).all(|hit| hit))
        .expect("a seed of consecutive hits")
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

/// D6: the stranger that follows the dead character `dead`.
fn stranger_after(world: &World, dead: EntityId) -> EntityId {
    let stranger = world.stranger_of(dead);
    assert_ne!(stranger, None, "no stranger follows {dead:?}");
    stranger.expect("asserted above")
}

fn change(time: u64, entity: EntityId, kind: ChangeKind) -> PerceivedChange {
    PerceivedChange {
        time: Time::new(time),
        entity,
        kind,
    }
}

fn corpse(died: Died, entity: EntityId, position: TilePosition) -> PerceivedCorpse {
    PerceivedCorpse {
        position,
        died,
        entity: Some(entity),
    }
}

/// D6, A7: a stranger as another character perceives it, unhurt.
fn unhurt_character(entity: EntityId, position: TilePosition) -> PerceivedEntity {
    PerceivedEntity {
        entity,
        position,
        disposition: None,
        health: PerceivedHealth::Condition(Condition::Unhurt),
    }
}

/// Submits `choose` for the awaited entity, turn after turn, until `done`.
fn play(
    world: &mut World,
    mut choose: impl FnMut(&World) -> Action,
    done: impl Fn(&World) -> bool,
) {
    for _ in 0..MOST_TURNS {
        if done(world) {
            return;
        }
        let action = choose(world);
        assert_eq!(world.submit(action), Ok(()), "{action:?}");
    }
    panic!("not done after {MOST_TURNS} turns");
}

/// `striker` attacks toward `direction` on each of its turns, and every
/// other character waits, until `target` dies.
fn strike_to_death(world: &mut World, striker: EntityId, direction: Direction, target: EntityId) {
    play(
        world,
        |world| {
            if world.awaited() == striker {
                Action::Attack(direction)
            } else {
                Action::Wait
            }
        },
        |world| deaths(world).contains(&target),
    );
}

/// The first character stands on the start tile and waits. The second
/// stands east of it and strikes it. Both have speed 1, and the first is
/// due first (R8).
struct Duel {
    world: World,
    victim: EntityId,
    striker: EntityId,
}

fn duel(seed: u64) -> Duel {
    let mut world = World::new(seed, Speed::new(1));
    let victim = world.awaited();
    let striker = world.create_entity(Speed::new(1), at(2, 1), Controller::Player);
    Duel {
        world,
        victim,
        striker,
    }
}

#[test]
fn a_character_struck_to_death_leaves_a_character_corpse_and_a_stranger_on_the_start_tile() {
    // The four strikes of the striker hit: 10, 7, 4, 1, then 0 hit points.
    let seed = seed_of_hits(4);
    let Duel {
        mut world,
        victim,
        striker,
    } = duel(seed);

    strike_to_death(&mut world, striker, Direction::West, victim);

    // A6, A10, A11, R8: the victim waits for 50 and the striker strikes for
    // 100. At a tie, the entity due first acts first. The lethal hit is
    // followed by the death, at the same time, and nothing comes after.
    let hit = ChangeKind::Act(Outcome::Hit(victim));
    let waited = ChangeKind::Act(Outcome::Waited);
    assert_eq!(
        world.perceived_changes(),
        [
            change(0, victim, waited),
            change(0, striker, hit),
            change(0, victim, ChangeKind::Condition(Condition::Wounded)),
            change(50, victim, waited),
            change(100, striker, hit),
            change(100, victim, ChangeKind::Condition(Condition::BadlyWounded)),
            change(100, victim, waited),
            change(150, victim, waited),
            change(200, striker, hit),
            change(200, victim, ChangeKind::Condition(Condition::Dying)),
            change(200, victim, waited),
            change(250, victim, waited),
            change(300, striker, hit),
            change(300, victim, ChangeKind::Death),
        ],
        "seed {seed}"
    );

    // D6, F2: the world awaits a stranger, at the time of the death, with an
    // id it never issued before.
    let stranger = world.awaited();
    assert_eq!(world.stranger_of(victim), Some(stranger));
    assert!(![victim, striker].contains(&stranger), "{stranger:?}");
    assert_eq!(world.clock(), Time::new(300));

    // D2, D4: the corpse of the victim says a character, in the snapshot of
    // the striker. D8: the victim died on the start tile and left it free,
    // so the stranger stands there.
    let seen_by_striker = perception::snapshot(&world, striker);
    assert_eq!(
        seen_by_striker.corpses,
        [corpse(Died::Character, victim, START)]
    );
    assert_eq!(
        seen_by_striker.entities,
        [
            PerceivedEntity {
                entity: striker,
                position: at(2, 1),
                disposition: None,
                health: PerceivedHealth::HitPoints {
                    current: 10,
                    max: 10
                },
            },
            unhurt_character(stranger, START),
        ]
    );
    // A7: the stranger has 10 hit points of 10. R6: at speed 1, its wait
    // costs 50.
    let own = perception::snapshot(&world, stranger)
        .entities
        .into_iter()
        .find(|perceived| perceived.entity == stranger)
        .map(|perceived| perceived.health);
    assert_eq!(
        own,
        Some(PerceivedHealth::HitPoints {
            current: 10,
            max: 10
        })
    );
    assert!(
        world.offers().contains(&Offer {
            action: Action::Wait,
            cost: ActionCost::new(50),
        }),
        "{:?}",
        world.offers()
    );
}

#[test]
fn an_entity_due_at_the_time_of_the_death_acts_before_the_stranger() {
    // The first character strikes at speed 2, for 50. The victim, east of
    // it, bumps into it for 100. A third character waits for 50. All are due
    // at time zero in creation order (R8).
    let seed = seed_of_hits(4);
    let mut world = World::new(seed, Speed::new(2));
    let striker = world.awaited();
    let victim = world.create_entity(Speed::new(1), at(2, 1), Controller::Player);
    let third = world.create_entity(Speed::new(1), at(4, 1), Controller::Player);

    let mut seen = 0;
    play(
        &mut world,
        |world| {
            seen = world.perceived_changes().len();
            match world.awaited() {
                awaited if awaited == striker => Action::Attack(Direction::East),
                awaited if awaited == victim => Action::Move(Direction::West),
                _ => Action::Wait,
            }
        },
        |world| deaths(world).contains(&victim),
    );

    // The fourth hit lands at 150. The victim last bumped at 100, so its
    // own entry is due at 200: a stranger that took that entry would come
    // later. The third character is due at 150 since its wait at 100.
    assert_eq!(
        world.perceived_changes()[seen..],
        [
            change(150, striker, ChangeKind::Act(Outcome::Hit(victim))),
            change(150, victim, ChangeKind::Death),
        ]
    );
    assert!(
        world.perceived_changes().contains(&change(
            100,
            victim,
            ChangeKind::Act(Outcome::Bumped(striker))
        )),
        "{:?}",
        world.perceived_changes()
    );

    // D9, R8: the third character was due at the time of the death before
    // the stranger arrived. It acts first.
    assert_eq!((world.awaited(), world.clock()), (third, Time::new(150)));
    let seen = world.perceived_changes().len();
    assert_eq!(world.submit(Action::Wait), Ok(()));
    assert_eq!(
        world.perceived_changes()[seen..],
        [change(150, third, ChangeKind::Act(Outcome::Waited))]
    );

    // R2: the world then awaits the stranger, still at the time of the
    // death.
    let stranger = stranger_after(&world, victim);
    assert_eq!((world.awaited(), world.clock()), (stranger, Time::new(150)));
}

#[test]
fn with_no_other_entity_due_at_the_time_of_the_death_the_world_awaits_the_stranger_at_once() {
    // The striker strikes at speed 2, for 50. The victim, east of it, bumps
    // into it for 100. After the fourth hit, at 150, the striker and the
    // entry of the victim are both due at 200.
    let seed = seed_of_hits(4);
    let mut world = World::new(seed, Speed::new(2));
    let striker = world.awaited();
    let victim = world.create_entity(Speed::new(1), at(2, 1), Controller::Player);

    play(
        &mut world,
        |world| {
            if world.awaited() == striker {
                Action::Attack(Direction::East)
            } else {
                Action::Move(Direction::West)
            }
        },
        |world| deaths(world).contains(&victim),
    );
    assert_eq!(
        world.perceived_changes().last(),
        Some(&change(150, victim, ChangeKind::Death))
    );

    // D9, R2: the stranger is due at the time of the death, before both.
    let stranger = stranger_after(&world, victim);
    assert_eq!((world.awaited(), world.clock()), (stranger, Time::new(150)));
}

#[test]
fn a_stranger_has_the_speed_of_a_character_whatever_the_speed_of_the_dead_or_the_striker() {
    // The victim and the striker both have speed 2.
    let seed = seed_of_hits(4);
    let mut world = World::new(seed, Speed::new(2));
    let victim = world.awaited();
    let striker = world.create_entity(Speed::new(2), at(2, 1), Controller::Player);

    strike_to_death(&mut world, striker, Direction::West, victim);
    let stranger = stranger_after(&world, victim);
    play(
        &mut world,
        |_| Action::Wait,
        |world| world.awaited() == stranger,
    );

    // D6, R6: at speed 1, the wait of the stranger costs 50.
    let wait = world
        .offers()
        .into_iter()
        .find(|offer| offer.action == Action::Wait)
        .map(|offer| offer.cost);
    assert_eq!(wait, Some(ActionCost::new(50)));
}

/// Six characters and three creatures, each creature walled in by
/// characters, walls and R11: a creature can only wait. The victim, on
/// the start tile, stands west of the striker. A third character stands
/// west of a creature, a fourth east of it. A fifth stands north of a
/// second creature, in the narrow passage; a third creature south of the
/// passage is walled in by the second and by a sixth character west of it.
struct Crowd {
    world: World,
    victim: EntityId,
    striker: EntityId,
    west_of_first: EntityId,
    north_of_second: EntityId,
    first: EntityId,
    second: EntityId,
    third: EntityId,
}

fn crowd(seed: u64) -> Crowd {
    let mut world = World::new(seed, Speed::new(1));
    let character =
        |world: &mut World, x, y| world.create_entity(Speed::new(1), at(x, y), Controller::Player);
    let creature = |world: &mut World, x, y| {
        world.create_entity(
            Speed::new(2),
            at(x, y),
            Controller::Wander(Disposition::Neutral),
        )
    };
    let victim = world.awaited();
    let striker = character(&mut world, 2, 1);
    let west_of_first = character(&mut world, 4, 1);
    let first = creature(&mut world, 5, 1);
    character(&mut world, 6, 1);
    let north_of_second = character(&mut world, 7, 2);
    let second = creature(&mut world, 7, 3);
    let third = creature(&mut world, 7, 4);
    character(&mut world, 6, 4);
    Crowd {
        world,
        victim,
        striker,
        west_of_first,
        north_of_second,
        first,
        second,
        third,
    }
}

/// What a character perceived before the death of `victim`, with the death
/// written in: the victim gone, its corpse after the earlier ones, and the
/// stranger after every other entity (D2, D6).
fn with_death(
    mut before: Snapshot,
    victim: EntityId,
    stranger: EntityId,
    tile: TilePosition,
) -> Snapshot {
    before
        .entities
        .retain(|perceived| perceived.entity != victim);
    before.entities.push(unhurt_character(stranger, tile));
    before.corpses.push(corpse(Died::Character, victim, tile));
    before
}

#[test]
fn the_death_of_a_character_changes_nothing_else() {
    let seed = 7;
    let Crowd {
        mut world,
        victim,
        striker,
        west_of_first,
        north_of_second,
        first,
        second,
        third,
    } = crowd(seed);
    let living = |world: &World, entity| !deaths(world).contains(&entity);
    let struck = |world: &World, by, target| {
        world.perceived_changes().iter().any(|change| {
            (change.entity, change.kind) == (by, ChangeKind::Act(Outcome::Hit(target)))
        })
    };

    // Before the death, the world holds an earlier corpse, a wounded
    // character, a wounded hostile creature and an untouched neutral one:
    // the victim hits the striker once, a character kills the first
    // creature, and another hits the second creature once (A4, A6).
    let ready = |world: &World| {
        struck(world, victim, striker)
            && !living(world, first)
            && struck(world, north_of_second, second)
    };
    let mut before = Vec::new();
    let mut seen = 0;
    play(
        &mut world,
        |world| {
            let awaited = world.awaited();
            if (awaited == victim && !struck(world, victim, striker))
                || (awaited == west_of_first && living(world, first))
            {
                Action::Attack(Direction::East)
            } else if awaited == north_of_second && !struck(world, north_of_second, second) {
                Action::Attack(Direction::South)
            } else if awaited == striker && ready(world) {
                // Every character, as it perceives the world before this
                // strike, which may be the lethal one.
                seen = world.perceived_changes().len();
                before = perception::snapshot(world, striker)
                    .entities
                    .iter()
                    .filter(|perceived| perceived.disposition.is_none())
                    .map(|perceived| {
                        (
                            perceived.entity,
                            perception::snapshot(world, perceived.entity),
                        )
                    })
                    .collect();
                Action::Attack(Direction::West)
            } else {
                Action::Wait
            }
        },
        |world| !living(world, victim),
    );
    let time = world.perceived_changes()[seen].time;

    // The lethal hit and the death, then only the waits of the walled-in
    // creatures due at that time.
    let after = &world.perceived_changes()[seen..];
    assert_eq!(
        after[..2],
        [
            change(time.units(), striker, ChangeKind::Act(Outcome::Hit(victim))),
            change(time.units(), victim, ChangeKind::Death),
        ]
    );
    assert!(
        after[2..].iter().all(|change| {
            [second, third].contains(&change.entity)
                && change.kind == ChangeKind::Act(Outcome::Waited)
        }),
        "{after:?}"
    );

    // D12: the clock, every other entity, its hit points or its condition,
    // its disposition and the earlier corpse stay as each character
    // perceived them. The stranger arrives on the start tile, where the
    // victim died.
    let stranger = stranger_after(&world, victim);
    assert_eq!(world.clock(), time);
    let viewers: Vec<EntityId> = before.iter().map(|&(viewer, _)| viewer).collect();
    assert_eq!(viewers.len(), 6, "{viewers:?}");
    for (viewer, seen_before) in before {
        if viewer == victim {
            continue;
        }
        assert_eq!(
            perception::snapshot(&world, viewer),
            with_death(seen_before, victim, stranger, START),
            "seen by {viewer:?}"
        );
    }
    // The preconditions of that claim: what stays is not what a world
    // begins with.
    let seen_by_striker = perception::snapshot(&world, striker);
    assert_eq!(
        seen_by_striker.corpses[0],
        corpse(Died::Creature, first, at(5, 1))
    );
    let health = |entity| {
        seen_by_striker
            .entities
            .iter()
            .find(|perceived| perceived.entity == entity)
            .map(|perceived| (perceived.disposition, perceived.health))
    };
    assert_eq!(
        health(striker),
        Some((
            None,
            PerceivedHealth::HitPoints {
                current: 7,
                max: 10
            }
        ))
    );
    assert_eq!(
        health(second),
        Some((
            Some(Disposition::Hostile),
            PerceivedHealth::Condition(Condition::Wounded)
        ))
    );
    assert_eq!(
        health(third),
        Some((
            Some(Disposition::Neutral),
            PerceivedHealth::Condition(Condition::Unhurt)
        ))
    );

    // D6: no other entity has a stranger, living or dead.
    for other in viewers.into_iter().chain([first, second, third]) {
        if other != victim {
            assert_eq!(world.stranger_of(other), None, "{other:?}");
        }
    }
}

/// D8: the start tile, then the floor tiles one move from it, in row order
/// from the north, each row from the west. Then a tile two moves away,
/// reached through tiles that entities hold: moves ignore every entity.
#[test]
fn the_stranger_takes_the_nearest_free_floor_tile_in_row_order() {
    let held_around_the_start = [at(2, 1), at(1, 2), at(2, 2)];
    for held in 0..=held_around_the_start.len() {
        // The first character holds the start tile. The victim dies far
        // from it, south of the striker.
        let mut world = World::new(seed_of_hits(4), Speed::new(1));
        for &tile in &held_around_the_start[..held] {
            world.create_entity(Speed::new(1), tile, Controller::Player);
        }
        let victim = world.create_entity(Speed::new(1), at(4, 4), Controller::Player);
        let striker = world.create_entity(Speed::new(1), at(4, 3), Controller::Player);

        strike_to_death(&mut world, striker, Direction::South, victim);

        let stranger = stranger_after(&world, victim);
        let expected = [at(2, 1), at(1, 2), at(2, 2), at(3, 1)][held];
        assert_eq!(world.position(stranger), expected, "{held} held");
        // D2: the corpse lies where the victim died, not where its stranger
        // arrives.
        assert_eq!(
            perception::snapshot(&world, striker).corpses,
            [corpse(Died::Character, victim, at(4, 4))],
            "{held} held"
        );
    }
}

/// D8: the dead character holds no tile, so its own tile may be the
/// nearest free one.
#[test]
fn the_stranger_takes_the_tile_of_the_death_when_it_is_the_nearest_free_one() {
    // The first character holds the start tile. The victim stands east of
    // it, the first tile one move away in row order, and is struck from
    // the east.
    let mut world = World::new(seed_of_hits(4), Speed::new(1));
    let victim = world.create_entity(Speed::new(1), at(2, 1), Controller::Player);
    let striker = world.create_entity(Speed::new(1), at(3, 1), Controller::Player);

    strike_to_death(&mut world, striker, Direction::West, victim);

    let stranger = stranger_after(&world, victim);
    assert_eq!(world.position(stranger), at(2, 1));
}

/// D8: the search starts from the tile where the first character started,
/// not from where it stands.
#[test]
fn the_stranger_arrives_on_the_start_tile_after_the_first_character_walked_away() {
    // From (3, 1), the first free tile one move away in row order would be
    // (2, 1).
    let away = at(3, 1);
    let mut world = World::new(seed_of_hits(4), Speed::new(1));
    let first = world.awaited();
    let victim = world.create_entity(Speed::new(1), at(4, 4), Controller::Player);
    let striker = world.create_entity(Speed::new(1), at(4, 3), Controller::Player);

    play(
        &mut world,
        |world| match world.awaited() {
            awaited if awaited == first && world.position(first) != away => {
                Action::Move(Direction::East)
            }
            awaited if awaited == striker => Action::Attack(Direction::South),
            _ => Action::Wait,
        },
        |world| deaths(world).contains(&victim),
    );

    assert_eq!(world.position(first), away);
    let stranger = stranger_after(&world, victim);
    assert_eq!(world.position(stranger), START);
}

/// R4, R11, D8: the tiles rank by the count of moves, not by the distance
/// in a straight line.
#[test]
fn the_stranger_takes_the_tile_fewest_moves_away_not_the_nearest_in_a_straight_line() {
    // With the first character, characters hold every floor tile within 5
    // moves of the start tile.
    let within_five_moves = [
        at(2, 1),
        at(3, 1),
        at(4, 1),
        at(5, 1),
        at(6, 1),
        at(1, 2),
        at(2, 2),
        at(3, 2),
        at(4, 2),
        at(1, 3),
        at(2, 3),
        at(3, 3),
        at(4, 3),
        at(5, 3),
        at(1, 4),
        at(2, 4),
        at(3, 4),
        at(4, 4),
        at(5, 4),
        at(6, 4),
    ];
    let mut world = World::new(seed_of_hits(4), Speed::new(1));
    for tile in within_five_moves {
        world.create_entity(Speed::new(1), tile, Controller::Player);
    }
    let striker = world.create_entity(Speed::new(1), at(7, 2), Controller::Player);
    let victim = world.create_entity(Speed::new(1), at(7, 3), Controller::Player);

    strike_to_death(&mut world, striker, Direction::South, victim);

    // (6, 2) is the nearest free tile in a straight line. R11 forbids the
    // diagonal past the wall at (5, 2), so it is 6 moves away, as (7, 1)
    // is, and (7, 1) comes first in row order.
    let stranger = stranger_after(&world, victim);
    assert_eq!(world.position(stranger), at(7, 1));
}

#[test]
fn a_stranger_dies_in_turn_and_a_second_stranger_arrives() {
    let seed = seed_of_hits(4);
    let run = || {
        let Duel {
            mut world,
            victim,
            striker,
        } = duel(seed);
        strike_to_death(&mut world, striker, Direction::West, victim);
        let first = stranger_after(&world, victim);
        let seen = world.perceived_changes().len();
        strike_to_death(&mut world, striker, Direction::West, first);
        (world, victim, striker, first, seen)
    };
    let (world, victim, striker, first, seen) = run();

    // D13: the stranger dies like any character. Its lethal hit is followed
    // by its death, at the same time.
    let second = stranger_after(&world, first);
    let lethal = world
        .perceived_changes()
        .iter()
        .rposition(|change| change.kind == ChangeKind::Act(Outcome::Hit(first)))
        .expect("the striker hit the stranger");
    let time = world.perceived_changes()[lethal].time;
    assert_eq!(
        world.perceived_changes()[lethal..],
        [
            change(time.units(), striker, ChangeKind::Act(Outcome::Hit(first))),
            change(time.units(), first, ChangeKind::Death),
        ]
    );
    assert!(lethal >= seen);
    assert_eq!(deaths(&world), [victim, first]);

    // F3, D6: `stranger_of` answers each one. F2: every id is new.
    assert_eq!(world.stranger_of(victim), Some(first));
    assert_eq!(world.stranger_of(first), Some(second));
    assert_eq!(world.stranger_of(second), None);
    assert_eq!(world.stranger_of(striker), None);
    let ids = [victim, striker, first, second];
    for (index, id) in ids.iter().enumerate() {
        assert!(!ids[..index].contains(id), "{id:?} is issued twice");
    }
    assert_eq!((world.awaited(), world.clock()), (second, time));
    assert_eq!(world.position(second), START);

    // D2, D4: both corpses say a character, in the order of the deaths, in
    // the snapshot of the striker.
    assert_eq!(
        perception::snapshot(&world, striker).corpses,
        [
            corpse(Died::Character, victim, START),
            corpse(Died::Character, first, START),
        ]
    );

    // Issue #40: the seed and the command log replay both strangers.
    let mut replay = duel(seed).world;
    for &action in world.command_log() {
        assert_eq!(replay.submit(action), Ok(()), "{action:?}");
    }
    assert_eq!(replay.stranger_of(first), Some(second));
    assert_eq!(replay, world);
}
