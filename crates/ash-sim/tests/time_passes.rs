//! Acceptance of spec 0001, slice S2: time passes; replay is green.

use ash_core::entity::{EntityId, Speed};
use ash_core::tile::Direction;
use ash_core::time::{ActionCost, Time};
use ash_sim::action::{Action, AvailableAction};
use ash_sim::perception::{Outcome, PerceivedChange};
use ash_sim::scheduler::Scheduler;
use ash_sim::world::World;

const SEED: u64 = 7;

fn waited(time: u64, entity: EntityId) -> PerceivedChange {
    PerceivedChange {
        time: Time::new(time),
        entity,
        outcome: Outcome::Waited,
    }
}

#[test]
fn scripted_waits_give_the_expected_clock_and_perceived_changes() {
    let mut world = World::new(SEED, Speed::new(1));
    let character = world.awaited();
    assert_eq!(world.clock(), Time::new(0));

    for _ in 0..3 {
        world.submit(Action::Wait).expect("a wait is never refused");
    }

    // The cost is paid after the effect (R7): each wait is stamped with the
    // time it was chosen, and the clock stops at the next turn (R2).
    assert_eq!(world.clock(), Time::new(150));
    assert_eq!(world.awaited(), character);
    assert_eq!(
        world.perceived_changes(),
        [
            waited(0, character),
            waited(50, character),
            waited(100, character),
        ]
    );
}

#[test]
fn speed_divides_the_cost_the_awaited_character_sees() {
    let mut world = World::new(SEED, Speed::new(2));
    let character = world.awaited();
    let offer = |action, cost| AvailableAction {
        action,
        cost: ActionCost::new(cost),
    };
    // From the start of the fixed map, only these three moves are allowed.
    assert_eq!(
        world.available_actions(),
        [
            offer(Action::Wait, 25),
            offer(Action::Move(Direction::East), 50),
            offer(Action::Move(Direction::SouthEast), 50),
            offer(Action::Move(Direction::South), 50),
        ]
    );

    world.submit(Action::Wait).expect("a wait is never refused");
    world.submit(Action::Wait).expect("a wait is never refused");

    assert_eq!(world.clock(), Time::new(50));
    assert_eq!(
        world.perceived_changes(),
        [waited(0, character), waited(25, character)]
    );
}

#[test]
fn same_time_tie_resolves_in_due_order() {
    let (first, second, early) = (EntityId::new(2), EntityId::new(1), EntityId::new(3));
    let mut scheduler = Scheduler::default();
    scheduler.schedule(first, Time::new(100));
    scheduler.schedule(early, Time::new(50));
    scheduler.schedule(second, Time::new(100));

    let order: Vec<_> = core::iter::from_fn(|| scheduler.pop()).collect();

    assert_eq!(
        order,
        [
            (Time::new(50), early),
            (Time::new(100), first),
            (Time::new(100), second),
        ]
    );
}

#[test]
fn replaying_the_command_log_gives_the_same_stream_and_state() {
    let mut first = World::new(SEED, Speed::new(2));
    for _ in 0..4 {
        first.submit(Action::Wait).expect("a wait is never refused");
    }
    assert_eq!(first.command_log(), [Action::Wait; 4]);

    let mut replay = World::new(SEED, Speed::new(2));
    for &action in first.command_log() {
        replay
            .submit(action)
            .expect("a logged command is never refused");
    }

    assert_eq!(replay.perceived_changes(), first.perceived_changes());
    assert_eq!(replay, first);
}
