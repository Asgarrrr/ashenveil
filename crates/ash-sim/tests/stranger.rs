//! Acceptance of spec 0003, slice S4, in the simulation: the stranger
//! arrives, and it knows nothing of the past (D10, D11, D4).
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
use ash_core::tile::{Direction, TilePosition};
use ash_core::time::Time;
use ash_sim::action::Action;
use ash_sim::controller::{Controller, Disposition};
use ash_sim::death::Died;
use ash_sim::perception::{self, ChangeKind, Outcome, PerceivedChange, PerceivedCorpse};
use ash_sim::world::World;

/// Turns after which a fight that has not ended fails.
const MOST_TURNS: usize = 2_000;

fn at(x: u32, y: u32) -> TilePosition {
    TilePosition { x, y, z: 0 }
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

#[test]
fn the_arrival_follows_the_death_and_begins_the_changes_of_the_stranger() {
    // The first character holds the start tile and strikes the victim east
    // of it. The victim's tile is then the nearest free one (D8): the
    // arrival names a tile other than the start tile.
    let mut world = World::new(0, Speed::new(1));
    let striker = world.awaited();
    let victim = world.create_entity(Speed::new(1), at(2, 1), Controller::Player);

    strike_to_death(&mut world, striker, Direction::East, victim);

    // D10: the lethal hit, the death, then the arrival, at the same time.
    let stranger = stranger_after(&world, victim);
    let all = world.perceived_changes().to_vec();
    let death = all
        .iter()
        .position(|change| change.kind == ChangeKind::Death)
        .expect("the victim died");
    let time = all[death].time;
    assert_eq!(
        all[death - 1..],
        [
            PerceivedChange {
                time,
                entity: striker,
                kind: ChangeKind::Act(Outcome::Hit(victim)),
            },
            PerceivedChange {
                time,
                entity: victim,
                kind: ChangeKind::Death,
            },
            PerceivedChange {
                time,
                entity: stranger,
                kind: ChangeKind::Arrival(at(2, 1)),
            },
        ]
    );
    assert_eq!(world.position(stranger), at(2, 1));

    // D11: the stranger perceives its arrival first. D5: the victim
    // perceived the changes of the world from its creation, its death last.
    // The striker perceives all three.
    let arrival = all[death + 1];
    assert_eq!(world.perceived_changes_of(stranger), [arrival]);
    assert_eq!(world.perceived_changes_of(victim), &all[..=death]);
    assert_eq!(world.perceived_changes_of(striker), &all[..]);

    // Time goes on: the stranger and the striker perceive what follows, the
    // dead character nothing more. R6, R8: the stranger waits for 50 at the
    // time of the death and 50 later; the striker, which struck for 100,
    // then waits.
    let striker_acted = |world: &World| {
        world.perceived_changes()[all.len()..]
            .iter()
            .any(|change| change.entity == striker)
    };
    play(&mut world, |_| Action::Wait, striker_acted);
    let waited = |units, entity| PerceivedChange {
        time: Time::new(time.units() + units),
        entity,
        kind: ChangeKind::Act(Outcome::Waited),
    };
    let later = [
        waited(0, stranger),
        waited(50, stranger),
        waited(100, striker),
    ];
    assert_eq!(world.perceived_changes()[all.len()..], later);
    assert_eq!(
        world.perceived_changes_of(stranger),
        [&[arrival][..], &later].concat()
    );
    assert_eq!(world.perceived_changes_of(victim), &all[..=death]);
    assert_eq!(
        world.perceived_changes_of(striker),
        world.perceived_changes()
    );
}

/// The cast of a world where two creatures die around the death of a
/// character. Each creature is walled in by characters, walls and R11, so
/// it can only wait. The victim, on the start tile, stands west of its
/// striker. The early killer stands west of the first creature, a
/// character east of it. The late killer stands north of the second
/// creature, in the narrow passage, and a character south of it.
struct Cast {
    world: World,
    victim: EntityId,
    striker: EntityId,
    early_killer: EntityId,
    late_killer: EntityId,
    first: EntityId,
    second: EntityId,
}

fn cast(seed: u64) -> Cast {
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
    let early_killer = character(&mut world, 4, 1);
    let first = creature(&mut world, 5, 1);
    character(&mut world, 6, 1);
    let late_killer = character(&mut world, 7, 2);
    let second = creature(&mut world, 7, 3);
    character(&mut world, 7, 4);
    Cast {
        world,
        victim,
        striker,
        early_killer,
        late_killer,
        first,
        second,
    }
}

fn corpse(position: TilePosition, died: Died, entity: Option<EntityId>) -> PerceivedCorpse {
    PerceivedCorpse {
        position,
        died,
        entity,
    }
}

#[test]
fn a_stranger_recognizes_the_corpses_of_the_deaths_after_its_arrival_alone() {
    let Cast {
        mut world,
        victim,
        striker,
        early_killer,
        late_killer,
        first,
        second,
    } = cast(0);

    // The first creature dies, then the victim, then the second creature:
    // one death before the arrival of the stranger, one after it.
    strike_to_death(&mut world, early_killer, Direction::East, first);
    strike_to_death(&mut world, striker, Direction::West, victim);
    let stranger = stranger_after(&world, victim);
    strike_to_death(&mut world, late_killer, Direction::South, second);
    assert_eq!(deaths(&world), [first, victim, second]);

    // D4, D11: the stranger perceived alive only the second creature. Every
    // corpse shows what died: the victim's says a character.
    let first_corpse = |entity| corpse(at(5, 1), Died::Creature, entity);
    let victim_corpse = |entity| corpse(at(1, 1), Died::Character, entity);
    let second_corpse = |entity| corpse(at(7, 3), Died::Creature, entity);
    assert_eq!(
        perception::snapshot(&world, stranger).corpses,
        [
            first_corpse(None),
            victim_corpse(None),
            second_corpse(Some(second))
        ]
    );
    // A character that no death created recognizes every corpse.
    let every_corpse_named = [
        first_corpse(Some(first)),
        victim_corpse(Some(victim)),
        second_corpse(Some(second)),
    ];
    for viewer in [striker, early_killer, late_killer] {
        assert_eq!(
            perception::snapshot(&world, viewer).corpses,
            every_corpse_named,
            "seen by {viewer:?}"
        );
    }

    // D13, D11: the stranger dies in turn. Its own stranger arrived after
    // every one of these deaths, so it recognizes none of them.
    strike_to_death(&mut world, striker, Direction::West, stranger);
    let second_stranger = stranger_after(&world, stranger);
    let stranger_corpse = |entity| corpse(at(1, 1), Died::Character, entity);
    assert_eq!(
        perception::snapshot(&world, second_stranger).corpses,
        [
            first_corpse(None),
            victim_corpse(None),
            second_corpse(None),
            stranger_corpse(None),
        ]
    );
    assert_eq!(
        perception::snapshot(&world, striker).corpses,
        [
            first_corpse(Some(first)),
            victim_corpse(Some(victim)),
            second_corpse(Some(second)),
            stranger_corpse(Some(stranger)),
        ]
    );
}
