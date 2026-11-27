//! What characters learn of the world (R15 to R18).

use alloc::vec::Vec;

use ash_core::entity::EntityId;
use ash_core::tile::{Direction, Terrain, TilePosition};
use ash_core::time::Time;

use crate::action::AvailableAction;
use crate::controller::{Controller, Disposition};
use crate::map::Map;
use crate::world::World;

/// One change, stamped with the time it happened (R17): what `entity` did.
/// Never the entity's speed, cost or next turn (R18).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PerceivedChange {
    pub time: Time,
    pub entity: EntityId,
    pub outcome: Outcome,
}

/// What an action did (A11), naming its target where it has one. Never the
/// odds of a hit (A12).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// A move reached the neighboring tile in this direction.
    Stepped(Direction),
    /// A wait let time pass.
    Waited,
    /// A move ran into this entity (R13): the mover stayed in place.
    Bumped(EntityId),
    /// An attack struck this entity (A6).
    Hit(EntityId),
    /// An attack missed this entity (A6).
    Missed(EntityId),
}

/// What a character perceives of the world: the only view a refusal reads
/// (R10, F4). It shows no entity's speed, cost or next turn (R18). Under
/// whole-map perception (R16), every character perceives the whole map,
/// where every entity stands and the disposition of every creature (A4).
pub(crate) struct Perception<'w> {
    world: &'w World,
}

impl<'w> Perception<'w> {
    /// R16: every character perceives the whole of `world`.
    pub(crate) fn whole_map(world: &'w World) -> Self {
        Self { world }
    }

    /// The map, every tile of it (R16).
    pub(crate) fn map(&self) -> &'w Map {
        self.world.map()
    }

    /// Where `entity` stands (R16).
    pub(crate) fn position(&self, entity: EntityId) -> TilePosition {
        self.world.position(entity)
    }

    /// The entity standing at `position`, if any (R16).
    pub(crate) fn occupant(&self, position: TilePosition) -> Option<EntityId> {
        self.world.occupant(position)
    }

    /// The disposition of `entity` when it is a creature (A4, R16). `None`
    /// for a character: it has no disposition.
    pub(crate) fn disposition(&self, entity: EntityId) -> Option<Disposition> {
        match self.world.controller(entity) {
            Controller::Player => None,
            Controller::Wander(disposition) => Some(disposition),
        }
    }
}

/// An entity a character perceives: where it stands and, for a creature,
/// its disposition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PerceivedEntity {
    pub entity: EntityId,
    pub position: TilePosition,
    /// A4: a kind, never a number (R18). `None` for a character: it has no
    /// disposition.
    pub disposition: Option<Disposition>,
}

/// Everything a character perceives at `time` (R16).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub time: Time,
    /// The number of tiles in each row of `terrain`.
    pub width: u32,
    /// Level 0, row by row from the north, each row from the west.
    pub terrain: Vec<Terrain>,
    /// Every entity, in creation order.
    pub entities: Vec<PerceivedEntity>,
}

/// R16: the whole map and every entity, stamped with the clock. Positions
/// and dispositions only: no speed, cost or next turn (R18).
pub fn snapshot(world: &World) -> Snapshot {
    let perception = Perception::whole_map(world);
    Snapshot {
        time: world.clock(),
        width: world.terrain_width(),
        terrain: world.terrain().to_vec(),
        entities: world
            .entities()
            .map(|entity| PerceivedEntity {
                entity,
                position: perception.position(entity),
                disposition: perception.disposition(entity),
            })
            .collect(),
    }
}

/// R9: the actions `character` can take, with their costs. `None` when the
/// world awaits another entity: its costs go only to its own controller
/// (R18).
pub fn turn(world: &World, character: EntityId) -> Option<Vec<AvailableAction>> {
    (world.awaited() == character).then(|| world.available_actions())
}

#[cfg(test)]
mod tests {
    use ash_core::entity::Speed;

    use super::*;
    use crate::action::Action;
    use crate::controller::Controller;

    #[test]
    fn the_turn_of_another_entity_is_hidden_from_this_character() {
        let mut world = World::new(0, Speed::new(1));
        let character = world.awaited();
        let other = world.create_entity(
            Speed::new(2),
            TilePosition { x: 3, y: 1, z: 0 },
            Controller::Player,
        );
        assert_eq!(world.submit(Action::Wait), Ok(()));
        assert_eq!(world.awaited(), other);

        assert_eq!(turn(&world, character), None);
        assert_eq!(turn(&world, other), Some(world.available_actions()));
    }
}
