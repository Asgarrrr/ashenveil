//! What characters learn of the world (R15 to R18).

use alloc::vec::Vec;

use ash_core::entity::EntityId;
use ash_core::tile::{Direction, Terrain, TilePosition};
use ash_core::time::Time;

use crate::action::Offer;
use crate::combat::{Condition, Health};
use crate::controller::{Controller, Disposition};
use crate::death::Died;
use crate::map::Map;
use crate::world::World;

/// One change to `entity`, stamped with the time it happened (R17, A11).
/// Never an entity's speed, cost, next turn or hit points (R18).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PerceivedChange {
    pub time: Time,
    pub entity: EntityId,
    pub kind: ChangeKind,
}

/// What a perceived change says of its entity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChangeKind {
    /// What the entity did.
    Act(Outcome),
    /// The entity died and left the world (A8). Its lethal hit is the change
    /// before, at the same time.
    Death,
    /// A hit left the entity in another condition (A10). That hit is the
    /// change before, at the same time.
    Condition(Condition),
    /// A character's attack left the creature in this disposition (A4): the
    /// last change of that attack, at the same time (A11).
    Disposition(Disposition),
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
    /// An attack hit this entity (A6).
    Hit(EntityId),
    /// An attack missed this entity (A6).
    Missed(EntityId),
}

/// What a character perceives of the world: the only view a refusal reads
/// (R10, F4). It shows no entity's speed, cost or next turn (R18). Under
/// whole-map perception (R16), every character perceives the whole map,
/// where every entity stands, the disposition of every creature (A4) and the
/// condition of every entity (A10).
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

    /// The condition of `entity` (A10, R16): a kind, never its hit points
    /// (R18).
    pub(crate) fn condition(&self, entity: EntityId) -> Condition {
        self.world.health(entity).condition()
    }
}

/// What a character perceives of the hit points of an entity: its own as
/// numbers (A9), those of any other entity as a condition (A10, R18).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PerceivedHealth {
    /// The hit points of the character itself.
    HitPoints { current: u32, max: u32 },
    /// The condition of another entity.
    Condition(Condition),
}

/// An entity a character perceives: where it stands, its hit points or its
/// condition and, for a creature, its disposition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PerceivedEntity {
    pub entity: EntityId,
    pub position: TilePosition,
    /// A4: a kind, never a number (R18). `None` for a character: it has no
    /// disposition.
    pub disposition: Option<Disposition>,
    pub health: PerceivedHealth,
}

/// A corpse a character perceives (D4): where it lies, what died, and the
/// entity that died when the character recognizes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PerceivedCorpse {
    pub position: TilePosition,
    pub died: Died,
    /// `None` when the character does not recognize the corpse.
    pub entity: Option<EntityId>,
}

/// Everything one character perceives at `time` (R16).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub time: Time,
    /// The number of tiles in each row of `terrain`.
    pub width: u32,
    /// Level 0, row by row from the north, each row from the west.
    pub terrain: Vec<Terrain>,
    /// Every entity, in creation order.
    pub entities: Vec<PerceivedEntity>,
    /// Every corpse, in the order of the deaths (D2, D4).
    pub corpses: Vec<PerceivedCorpse>,
}

/// R16: the whole map, every entity and every corpse as `character`
/// perceives them, stamped with the clock. Hit points for `character` alone
/// (A9); a position, a disposition and a condition for the others: no
/// speed, cost, next turn or hit points (R18).
pub fn snapshot(world: &World, character: EntityId) -> Snapshot {
    // Read first: a `character` that is not embodied crashes here (F3),
    // instead of leaving a snapshot with no hit points.
    let Health { current, max } = world.health(character);
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
                health: if entity == character {
                    PerceivedHealth::HitPoints { current, max }
                } else {
                    PerceivedHealth::Condition(perception.condition(entity))
                },
            })
            .collect(),
        // D4: until a character can arrive after a death (spec 0003, S4),
        // every character perceived every dead entity alive.
        corpses: world
            .corpses()
            .into_iter()
            .map(|corpse| PerceivedCorpse {
                position: corpse.position,
                died: corpse.died,
                entity: Some(corpse.entity),
            })
            .collect(),
    }
}

/// R9: the offers of `character`. `None` when the world awaits another
/// entity: its costs go only to its own controller (R18).
pub fn turn(world: &World, character: EntityId) -> Option<Vec<Offer>> {
    (world.awaited() == character).then(|| world.offers())
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
        assert_eq!(turn(&world, other), Some(world.offers()));
    }

    /// F3: a snapshot for an id the world never issued crashes as such. It
    /// never comes back whole, with hit points for no entity.
    #[test]
    #[should_panic(expected = "this world never issued")]
    fn a_snapshot_for_an_id_the_world_never_issued_crashes_as_never_issued() {
        // The world issued one id, the character's: the next one is the
        // nearest id it never issued.
        let world = World::new(0, Speed::new(1));
        let _ = snapshot(&world, EntityId::new(1));
    }
}
