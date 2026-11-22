//! One simulated world: its clock, entities, schedule and history.

use alloc::vec::Vec;

use ash_core::entity::{EntityId, Speed};
use ash_core::rng::Rng;
use ash_core::tile::{Direction, Terrain, TilePosition};
use ash_core::time::Time;

use crate::action::{Action, AvailableAction, Refused};
use crate::controller::Controller;
use crate::map::Map;
use crate::perception::PerceivedChange;
use crate::scheduler::Scheduler;
use crate::wander;

/// Where the character stands when the world begins: a floor tile of the
/// fixed map.
const CHARACTER_START: TilePosition = TilePosition { x: 1, y: 1, z: 0 };

/// Where the creature stands when the world begins: a floor tile of the
/// fixed map, across the room from the character.
const CREATURE_START: TilePosition = TilePosition { x: 7, y: 4, z: 0 };

#[derive(Clone, Debug, PartialEq, Eq)]
struct Entity {
    id: EntityId,
    speed: Speed,
    position: TilePosition,
    controller: Controller,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct World {
    clock: Time,
    map: Map,
    entities: Vec<Entity>,
    scheduler: Scheduler,
    awaited: EntityId,
    rng: Rng,
    perceived_changes: Vec<PerceivedChange>,
    command_log: Vec<Action>,
}

impl World {
    /// The fixed map holding one character, due at time zero. A player
    /// connection controls it.
    pub fn new(seed: u64, character_speed: Speed) -> Self {
        let character = EntityId::new(0);
        let mut world = Self {
            clock: Time::ZERO,
            map: Map::skeleton(),
            entities: alloc::vec![Entity {
                id: character,
                speed: character_speed,
                position: CHARACTER_START,
                controller: Controller::Player,
            }],
            scheduler: Scheduler::default(),
            awaited: character,
            rng: Rng::new(seed),
            perceived_changes: Vec::new(),
            command_log: Vec::new(),
        };
        world.scheduler.schedule(character, Time::ZERO);
        world.advance();
        world
    }

    /// The world of the skeleton, from `seed`: the character, then a
    /// wandering creature of speed 2 (R6, R8). A replay of a world's accepted
    /// commands starts from it too.
    pub fn skeleton(seed: u64) -> Self {
        let mut world = Self::new(seed, Speed::new(1));
        world.create_entity(Speed::new(2), CREATURE_START, Controller::Wander);
        world
    }

    /// Creates an entity at `position`, due now: after every entity created
    /// before it (R8). A position off the floor, or already occupied,
    /// crashes through `invariant!`.
    pub fn create_entity(
        &mut self,
        speed: Speed,
        position: TilePosition,
        controller: Controller,
    ) -> EntityId {
        let Ok(raw) = u32::try_from(self.entities.len()) else {
            ash_core::invariant!("the world holds more entities than ids")
        };
        let id = EntityId::new(raw);
        if self.map.terrain(position) != Terrain::Floor || self.occupant(position).is_some() {
            ash_core::invariant!("{id:?} cannot be created on {position:?}");
        }
        self.entities.push(Entity {
            id,
            speed,
            position,
            controller,
        });
        self.scheduler.schedule(id, self.clock);
        id
    }

    pub fn clock(&self) -> Time {
        self.clock
    }

    /// The character whose controller the world waits for. Time stops until
    /// it acts (R2).
    pub fn awaited(&self) -> EntityId {
        self.awaited
    }

    /// The actions the awaited character can take, with their costs (R9).
    /// A move the rules would refuse is not offered (R14).
    pub fn available_actions(&self) -> Vec<AvailableAction> {
        self.actions_of(self.awaited)
    }

    /// The actions the rules allow `entity`, with their costs (R9, R14).
    fn actions_of(&self, entity: EntityId) -> Vec<AvailableAction> {
        let speed = self.speed(entity);
        let from = self.position(entity);
        let moves = Direction::ALL
            .into_iter()
            .filter(|&direction| self.map.allows_move(from, direction))
            .map(Action::Move);
        core::iter::once(Action::Wait)
            .chain(moves)
            .map(|action| AvailableAction {
                action,
                cost: action.cost(speed),
            })
            .collect()
    }

    /// The number of tiles in each row of the terrain.
    pub fn terrain_width(&self) -> u32 {
        self.map.width()
    }

    /// Every tile of level 0, row by row from the north, each row from the
    /// west. Under whole-map perception (R16), every character perceives it.
    pub fn terrain(&self) -> &[Terrain] {
        self.map.tiles()
    }

    /// Every entity, in creation order. Under whole-map perception (R16),
    /// every character perceives each of them.
    pub fn entities(&self) -> impl Iterator<Item = EntityId> + '_ {
        self.entities.iter().map(|entity| entity.id)
    }

    /// Where `entity` stands. Under whole-map perception (R16), every
    /// character perceives it.
    pub fn position(&self, entity: EntityId) -> TilePosition {
        self.entity(entity).position
    }

    /// The awaited character takes `action`, then time runs to the next
    /// character that awaits a player connection. A refused move changes
    /// nothing (R10, R11).
    pub fn submit(&mut self, action: Action) -> Result<(), Refused> {
        self.act(self.awaited, action)?;
        self.command_log.push(action);
        self.advance();
        Ok(())
    }

    /// Every change, in the order it happened (R17). Under whole-map
    /// perception (R16), every character perceives every change.
    pub fn perceived_changes(&self) -> &[PerceivedChange] {
        &self.perceived_changes
    }

    /// The accepted commands of player connections, in order: replaying them
    /// on a world with the same seed and entities gives the same state. The
    /// wander needs no log: the RNG replays it.
    pub fn command_log(&self) -> &[Action] {
        &self.command_log
    }

    /// `entity` takes `action` under the rules every controller follows
    /// (R3), and becomes due once its cost is paid.
    fn act(&mut self, entity: EntityId, action: Action) -> Result<(), Refused> {
        let mut bumped = None;
        if let Action::Move(direction) = action {
            let from = self.position(entity);
            if !self.map.allows_move(from, direction) {
                return Err(Refused);
            }
            let to = from.step(direction);
            // R13: a move into an occupied tile is a bump. The mover stays
            // and pays the move; nothing else changes.
            match self.occupant(to) {
                Some(occupant) => bumped = Some(occupant),
                None => self.entity_mut(entity).position = to,
            }
        }
        // R7: the effect happens now; the cost delays the entity's next turn.
        self.perceived_changes.push(PerceivedChange {
            time: self.clock,
            entity,
            action,
            bumped,
        });
        let due = self.clock.after(action.cost(self.speed(entity)));
        self.scheduler.schedule(entity, due);
        Ok(())
    }

    /// Runs the clock entity by entity. A wandering entity acts at once; the
    /// clock stops at the first entity a player connection controls (R2).
    fn advance(&mut self) {
        loop {
            let Some((due, entity)) = self.scheduler.pop() else {
                ash_core::invariant!("no entity is scheduled")
            };
            self.clock = due;
            match self.entity(entity).controller {
                Controller::Player => {
                    self.awaited = entity;
                    return;
                }
                Controller::Wander => {
                    let offered = self.actions_of(entity);
                    let action = wander::choose(&mut self.rng, &offered);
                    if self.act(entity, action).is_err() {
                        ash_core::invariant!("the wander of {entity:?} chose a refused action");
                    }
                }
            }
        }
    }

    /// The entity standing at `position`, if any.
    fn occupant(&self, position: TilePosition) -> Option<EntityId> {
        self.entities
            .iter()
            .find(|entity| entity.position == position)
            .map(|entity| entity.id)
    }

    fn speed(&self, entity: EntityId) -> Speed {
        self.entity(entity).speed
    }

    fn entity(&self, entity: EntityId) -> &Entity {
        match self
            .entities
            .iter()
            .find(|candidate| candidate.id == entity)
        {
            Some(found) => found,
            None => ash_core::invariant!("{entity:?} is not in the world"),
        }
    }

    fn entity_mut(&mut self, entity: EntityId) -> &mut Entity {
        match self
            .entities
            .iter_mut()
            .find(|candidate| candidate.id == entity)
        {
            Some(found) => found,
            None => ash_core::invariant!("{entity:?} is not in the world"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "invariant violated")]
    fn an_entity_created_on_another_crashes() {
        let mut world = World::new(0, Speed::new(1));
        world.create_entity(Speed::new(2), CHARACTER_START, Controller::Wander);
    }

    #[test]
    #[should_panic(expected = "invariant violated")]
    fn an_entity_created_on_a_wall_crashes() {
        let mut world = World::new(0, Speed::new(1));
        let wall = TilePosition { x: 0, y: 0, z: 0 };
        world.create_entity(Speed::new(2), wall, Controller::Wander);
    }
}
