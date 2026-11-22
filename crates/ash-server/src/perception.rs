//! What the controller of a character learns, as protocol messages: the
//! perception `ash_sim::perception` computes (R15 to R18), translated.

use ash_core::entity::EntityId;
use ash_protocol::message::{
    AvailableAction, PerceivedChange, PerceivedEntity, ServerMessage, Snapshot,
};
use ash_sim::perception;
use ash_sim::world::World;

use crate::action;

/// R16: what every character perceives, as a protocol message.
pub fn snapshot(world: &World) -> ServerMessage {
    let snapshot = perception::snapshot(world);
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
    world.perceived_changes().iter().skip(seen).map(|change| {
        ServerMessage::Perceived(PerceivedChange {
            time: change.time,
            entity: change.entity,
            action: action::to_wire(change.action),
            bumped: change.bumped,
        })
    })
}
