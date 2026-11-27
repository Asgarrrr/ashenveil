//! What the controller of a character learns, as protocol messages: the
//! perception `ash_sim::perception` computes (R15 to R18), translated.

use ash_core::entity::EntityId;
use ash_protocol::message::{
    AvailableAction, Condition, Disposition, Outcome, PerceivedChange, PerceivedEntity,
    ServerMessage, Snapshot,
};
use ash_sim::combat::Condition as SimCondition;
use ash_sim::controller::Disposition as SimDisposition;
use ash_sim::perception::{self, Outcome as SimOutcome, PerceivedChange as SimChange};
use ash_sim::world::World;

use crate::action;

/// R16: what `character` perceives, as a protocol message.
pub fn snapshot(world: &World, character: EntityId) -> ServerMessage {
    let snapshot = perception::snapshot(world, character);
    ServerMessage::Snapshot(Snapshot {
        time: snapshot.time,
        width: snapshot.width,
        terrain: snapshot.terrain,
        entities: snapshot
            .entities
            .into_iter()
            .map(|perceived| PerceivedEntity {
                entity: perceived.entity,
                position: perceived.position,
                disposition: perceived.disposition.map(disposition_to_wire),
            })
            .collect(),
    })
}

/// R9: the turn of `character`, for its controller only (R18).
pub fn awaited(world: &World, character: EntityId) -> Option<ServerMessage> {
    perception::turn(world, character).map(|actions| ServerMessage::Awaited {
        time: world.clock(),
        entity: character,
        actions: actions
            .into_iter()
            .map(|offered| AvailableAction {
                action: action::to_wire(offered.action),
                cost: offered.cost,
            })
            .collect(),
    })
}

/// R17: the changes after the first `seen`, in the order they happened.
pub fn changes_since(world: &World, seen: usize) -> impl Iterator<Item = ServerMessage> + '_ {
    world
        .perceived_changes()
        .iter()
        .skip(seen)
        .map(|&change| ServerMessage::Perceived(change_to_wire(change)))
}

fn change_to_wire(change: SimChange) -> PerceivedChange {
    match change {
        SimChange::Act {
            time,
            entity,
            outcome,
        } => PerceivedChange::Act {
            time,
            entity,
            outcome: outcome_to_wire(outcome),
        },
        SimChange::Death { time, entity } => PerceivedChange::Death { time, entity },
        SimChange::Condition {
            time,
            entity,
            condition,
        } => PerceivedChange::Condition {
            time,
            entity,
            condition: condition_to_wire(condition),
        },
    }
}

fn outcome_to_wire(outcome: SimOutcome) -> Outcome {
    match outcome {
        SimOutcome::Stepped(direction) => Outcome::Stepped(direction),
        SimOutcome::Waited => Outcome::Waited,
        SimOutcome::Bumped(target) => Outcome::Bumped(target),
        SimOutcome::Hit(target) => Outcome::Hit(target),
        SimOutcome::Missed(target) => Outcome::Missed(target),
    }
}

fn condition_to_wire(condition: SimCondition) -> Condition {
    match condition {
        SimCondition::Unhurt => Condition::Unhurt,
        SimCondition::Wounded => Condition::Wounded,
        SimCondition::BadlyWounded => Condition::BadlyWounded,
        SimCondition::Dying => Condition::Dying,
    }
}

fn disposition_to_wire(disposition: SimDisposition) -> Disposition {
    match disposition {
        SimDisposition::Hostile => Disposition::Hostile,
        SimDisposition::Neutral => Disposition::Neutral,
    }
}

#[cfg(test)]
pub mod tests {
    use ash_core::entity::Speed;
    use ash_core::tile::TilePosition;
    use ash_sim::action::Action;
    use ash_sim::controller::Controller;

    use super::*;

    /// A world whose character has acted, so it awaits a second entity that
    /// another player connection controls. Returns both entities.
    pub fn world_awaiting_another_player() -> (World, EntityId, EntityId) {
        let mut world = World::new(0, Speed::new(1));
        let character = world.awaited();
        let other = world.create_entity(
            Speed::new(2),
            TilePosition { x: 3, y: 1, z: 0 },
            Controller::Player,
        );
        assert_eq!(world.submit(Action::Wait), Ok(()));
        assert_eq!(world.awaited(), other);
        (world, character, other)
    }

    #[test]
    fn the_turn_of_another_entity_never_reaches_this_controller() {
        let (world, character, other) = world_awaiting_another_player();
        assert_eq!(awaited(&world, character), None);
        assert!(matches!(
            awaited(&world, other),
            Some(ServerMessage::Awaited { entity, .. }) if entity == other
        ));
    }
}
