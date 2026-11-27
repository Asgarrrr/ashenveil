//! What the controller of a character learns, as protocol messages: the
//! perception `ash_sim::perception` computes (R15 to R18), translated.

use ash_core::entity::EntityId;
use ash_protocol::message::{
    ChangeKind, Condition, Disposition, Offer, Outcome, PerceivedChange, PerceivedEntity,
    PerceivedHealth, ServerMessage, Snapshot,
};
use ash_sim::combat::Condition as SimCondition;
use ash_sim::controller::Disposition as SimDisposition;
use ash_sim::perception::{
    self, ChangeKind as SimKind, Outcome as SimOutcome, PerceivedChange as SimChange,
    PerceivedHealth as SimHealth,
};
use ash_sim::world::World;

use crate::action;

/// R16: what `character` perceives, as a protocol message. Hit points for
/// `character` alone (A9): every other entity shows a condition (A10).
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
                health: health_to_wire(perceived.health),
            })
            .collect(),
    })
}

/// R9: the turn of `character`, for its controller only (R18).
pub fn awaited(world: &World, character: EntityId) -> Option<ServerMessage> {
    perception::turn(world, character).map(|offers| ServerMessage::Awaited {
        time: world.clock(),
        entity: character,
        offers: offers
            .into_iter()
            .map(|offer| Offer {
                action: action::to_wire(offer.action),
                cost: offer.cost,
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
    PerceivedChange {
        time: change.time,
        entity: change.entity,
        kind: match change.kind {
            SimKind::Act(outcome) => ChangeKind::Act(outcome_to_wire(outcome)),
            SimKind::Death => ChangeKind::Death,
            SimKind::Condition(condition) => ChangeKind::Condition(condition_to_wire(condition)),
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

fn health_to_wire(health: SimHealth) -> PerceivedHealth {
    match health {
        SimHealth::HitPoints { current, max } => PerceivedHealth::HitPoints { current, max },
        SimHealth::Condition(condition) => PerceivedHealth::Condition(condition_to_wire(condition)),
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
    use ash_core::tile::{Direction, TilePosition};
    use ash_sim::action::Action;
    use ash_sim::controller::Controller;

    use super::*;

    /// Turns after which a fight that has not landed its next hit fails.
    const STRIKE_TURNS: usize = 2_000;

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

    /// No hosted world shows these: nothing strikes its character (A13),
    /// and a creature dies before it is badly wounded (A7).
    #[test]
    fn a_struck_character_gets_its_hit_points_and_shows_each_condition_to_another() {
        let mut world = World::new(0, Speed::new(1));
        let attacker = world.awaited();
        let east = TilePosition { x: 2, y: 1, z: 0 };
        let target = world.create_entity(Speed::new(2), east, Controller::Player);
        let health = |world: &World, viewer: EntityId| match snapshot(world, viewer) {
            ServerMessage::Snapshot(snapshot) => snapshot
                .entities
                .iter()
                .find(|perceived| perceived.entity == target)
                .map(|perceived| perceived.health),
            _ => None,
        };

        // A6, A7, A10: 10 hit points, less 3 at each hit.
        let hits = [
            (7, Condition::Wounded),
            (4, Condition::BadlyWounded),
            (1, Condition::Dying),
        ];
        for (left, condition) in hits {
            // The attacker strikes until a hit, which changes the condition.
            let mut turns = 0;
            let changed = loop {
                assert!(
                    turns < STRIKE_TURNS,
                    "no hit leaves {left} hit points in {STRIKE_TURNS} turns"
                );
                turns += 1;
                if world.awaited() == target {
                    assert_eq!(world.submit(Action::Wait), Ok(()));
                    continue;
                }
                let (time, seen) = (world.clock(), world.perceived_changes().len());
                assert_eq!(world.submit(Action::Attack(Direction::East)), Ok(()));
                if let [_, ServerMessage::Perceived(change)] =
                    changes_since(&world, seen).collect::<Vec<_>>()[..]
                {
                    break (time, change);
                }
            };
            assert_eq!(
                changed.1,
                PerceivedChange {
                    time: changed.0,
                    entity: target,
                    kind: ChangeKind::Condition(condition),
                }
            );
            assert_eq!(
                health(&world, target),
                Some(PerceivedHealth::HitPoints {
                    current: left,
                    max: 10,
                })
            );
            assert_eq!(
                health(&world, attacker),
                Some(PerceivedHealth::Condition(condition))
            );
        }
    }
}
