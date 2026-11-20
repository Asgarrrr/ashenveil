//! What the controller of a character learns, as protocol messages (R15 to
//! R18). The only channel from a world to its client.

use ash_core::entity::EntityId;
use ash_protocol::message::{
    Action, AvailableAction, PerceivedChange, PerceivedEntity, ServerMessage, Snapshot, Terrain,
};
use ash_sim::world::{Terrain as SimTerrain, World};

/// R16: the whole map and every entity, stamped with the clock. Positions
/// only: no speed, cost or next turn (R18).
pub fn snapshot(world: &World) -> ServerMessage {
    let terrain = world
        .terrain()
        .iter()
        .map(|tile| match tile {
            SimTerrain::Floor => Terrain::Floor,
            SimTerrain::Wall => Terrain::Wall,
        })
        .collect();
    let entities = world
        .entities()
        .map(|entity| PerceivedEntity {
            entity,
            position: world.position(entity),
        })
        .collect();
    ServerMessage::Snapshot(Snapshot {
        time: world.clock(),
        width: world.terrain_width(),
        terrain,
        entities,
    })
}

/// R9: the turn of `character`, with each available action and its cost.
/// `None` when the world awaits another entity: its costs go only to its
/// own controller (R18).
pub fn awaited(world: &World, character: EntityId) -> Option<ServerMessage> {
    (world.awaited() == character).then(|| ServerMessage::Awaited {
        time: world.clock(),
        entity: character,
        actions: world
            .available_actions()
            .into_iter()
            .map(|offered| AvailableAction {
                action: wire_action(offered.action),
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
            action: wire_action(change.action),
        })
    })
}

fn wire_action(action: ash_sim::action::Action) -> Action {
    match action {
        ash_sim::action::Action::Move(direction) => Action::Move(direction),
        ash_sim::action::Action::Wait => Action::Wait,
    }
}
