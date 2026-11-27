//! One simulated world: its clock, entities, schedule and history.

use alloc::vec::Vec;

use ash_core::entity::{EntityId, Speed};
use ash_core::rng::Rng;
use ash_core::tile::{Terrain, TilePosition};
use ash_core::time::Time;

use crate::action::{self, Action, AvailableAction, Refused};
use crate::combat::Health;
use crate::controller::{Controller, Disposition};
use crate::effect::{Effect, Effects};
use crate::map::Map;
use crate::perception::{PerceivedChange, Perception};
use crate::scheduler::Scheduler;
use crate::wander;

/// Where the character stands when the world begins: a floor tile of the
/// fixed map.
const CHARACTER_START: TilePosition = TilePosition { x: 1, y: 1, z: 0 };

/// Where the hostile creature stands when the world begins: a floor tile of
/// the fixed map, across the room from the character.
const HOSTILE_START: TilePosition = TilePosition { x: 7, y: 4, z: 0 };

/// Where the neutral creature stands when the world begins: a floor tile of
/// the fixed map, down the west side from the character.
const NEUTRAL_START: TilePosition = TilePosition { x: 1, y: 4, z: 0 };

#[derive(Clone, Debug, PartialEq, Eq)]
struct Entity {
    id: EntityId,
    speed: Speed,
    position: TilePosition,
    controller: Controller,
    health: Health,
}

/// F3: what a total lookup finds under an id the world issued. Never a bare
/// absence.
enum Lookup<'w> {
    /// The entity is embodied in this world.
    Embodied(&'w Entity),
    /// The world issued the id, and no entity is embodied under it: the
    /// entity died (A8).
    NotEmbodied,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct World {
    map: Map,
    entities: Vec<Entity>,
    /// Its next entry is the turn being played: the clock and the awaited
    /// entity derive from it.
    scheduler: Scheduler,
    rng: Rng,
    perceived_changes: Vec<PerceivedChange>,
    command_log: Vec<Action>,
}

impl World {
    /// The fixed map holding one character, due at time zero. A player
    /// connection controls it, so the world awaits it at once (R2).
    pub fn new(seed: u64, character_speed: Speed) -> Self {
        let mut world = Self {
            map: Map::skeleton(),
            entities: Vec::new(),
            scheduler: Scheduler::default(),
            rng: Rng::new(seed),
            perceived_changes: Vec::new(),
            command_log: Vec::new(),
        };
        world.create_entity(character_speed, CHARACTER_START, Controller::Player);
        world
    }

    /// The world of the skeleton, from `seed`: the character, then a
    /// hostile and a neutral creature (A4), both wandering at speed 2 (R6,
    /// R8). A replay of a world's accepted commands starts from it too.
    pub fn skeleton(seed: u64) -> Self {
        let mut world = Self::new(seed, Speed::new(1));
        world.create_entity(
            Speed::new(2),
            HOSTILE_START,
            Controller::Wander(Disposition::Hostile),
        );
        world.create_entity(
            Speed::new(2),
            NEUTRAL_START,
            Controller::Wander(Disposition::Neutral),
        );
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
        let id = self.next_id();
        if self.map.terrain(position) != Terrain::Floor || self.occupant(position).is_some() {
            ash_core::invariant!("{id:?} cannot be created on {position:?}");
        }
        self.apply(Effects {
            in_order: alloc::vec![Effect::Create {
                id,
                speed,
                position,
                controller,
                health: Health::full(controller),
            }],
            rng_after: self.rng.clone(),
        });
        id
    }

    /// The time of the turn being played (R1).
    pub fn clock(&self) -> Time {
        self.turn().0
    }

    /// The character whose controller the world waits for. Time stops until
    /// it acts (R2).
    pub fn awaited(&self) -> EntityId {
        self.turn().1
    }

    /// The actions the awaited character can take, with their costs (R9).
    /// A move the rules would refuse is not offered (R14).
    pub fn available_actions(&self) -> Vec<AvailableAction> {
        self.actions_of(self.awaited())
    }

    /// The actions the rules allow `entity`, with their costs (R9, R14).
    fn actions_of(&self, entity: EntityId) -> Vec<AvailableAction> {
        let speed = self.speed(entity);
        action::offers(&Perception::whole_map(self), entity)
            .map(|action| AvailableAction {
                action,
                cost: action.cost(speed),
            })
            .collect()
    }

    /// The map, which every character perceives whole (R16).
    pub(crate) fn map(&self) -> &Map {
        &self.map
    }

    /// The number of tiles in each row of the terrain.
    pub(crate) fn terrain_width(&self) -> u32 {
        self.map.width()
    }

    /// Every tile of level 0, row by row from the north, each row from the
    /// west. Under whole-map perception (R16), every character perceives it.
    pub(crate) fn terrain(&self) -> &[Terrain] {
        self.map.tiles()
    }

    /// Every entity, in creation order. Under whole-map perception (R16),
    /// every character perceives each of them.
    pub(crate) fn entities(&self) -> impl Iterator<Item = EntityId> + '_ {
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
        let perception = Perception::whole_map(self);
        let effects = action::resolve(self, &perception, self.awaited(), action, &self.rng)?;
        self.apply(effects);
        // Controller input, not world state (F6): replay evidence only.
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

    /// Runs the clock entity by entity. A wandering entity acts at once; the
    /// clock stops at the first entity a player connection controls (R2).
    fn advance(&mut self) {
        loop {
            let entity = self.turn().1;
            match self.controller(entity) {
                Controller::Player => return,
                Controller::Wander(_) => {
                    let offered = self.actions_of(entity);
                    let (action, rng) = wander::choose(&self.rng, &offered);
                    let perception = Perception::whole_map(self);
                    let Ok(effects) = action::resolve(self, &perception, entity, action, &rng)
                    else {
                        ash_core::invariant!("the wander of {entity:?} chose a refused action")
                    };
                    self.apply(effects);
                }
            }
        }
    }

    /// F4: the only write path. Writes each effect in order, then installs
    /// the RNG state after the draws.
    fn apply(&mut self, effects: Effects) {
        for effect in effects.in_order {
            match effect {
                Effect::Place { entity, position } => self.entity_mut(entity).position = position,
                Effect::Perceive(change) => self.perceived_changes.push(change),
                Effect::EndTurn { due } => {
                    let Some((_, entity)) = self.scheduler.pop() else {
                        ash_core::invariant!("no entity is scheduled")
                    };
                    self.scheduler.schedule(entity, due);
                }
                Effect::Unschedule => {
                    if self.scheduler.pop().is_none() {
                        ash_core::invariant!("no entity is scheduled");
                    }
                }
                Effect::Damage { entity, amount } => {
                    let entity = self.entity_mut(entity);
                    entity.health = entity.health.damaged(amount);
                }
                Effect::Remove { entity } => {
                    let embodied = |candidate: &Entity| candidate.id == entity;
                    let Some(row) = self.entities.iter().position(embodied) else {
                        ash_core::invariant!("{entity:?} is not embodied")
                    };
                    self.entities.remove(row);
                }
                Effect::Create {
                    id,
                    speed,
                    position,
                    controller,
                    health,
                } => {
                    self.entities.push(Entity {
                        id,
                        speed,
                        position,
                        controller,
                        health,
                    });
                    // The first entity has no turn to join: the world
                    // begins at time zero.
                    let now = self.scheduler.head().map_or(Time::ZERO, |(due, _)| due);
                    self.scheduler.schedule(id, now);
                }
            }
        }
        self.rng = effects.rng_after;
    }

    /// The scheduler's next entry: the time of the turn being played, and
    /// the entity it belongs to.
    pub(crate) fn turn(&self) -> (Time, EntityId) {
        match self.scheduler.head() {
            Some((due, entity)) => (due, entity),
            None => ash_core::invariant!("no entity is scheduled"),
        }
    }

    /// The id the next created entity gets.
    fn next_id(&self) -> EntityId {
        match u32::try_from(self.entities.len()) {
            Ok(raw) => EntityId::new(raw),
            Err(_) => ash_core::invariant!("the world holds more entities than ids"),
        }
    }

    /// The entity standing at `position`, if any.
    pub(crate) fn occupant(&self, position: TilePosition) -> Option<EntityId> {
        self.entities
            .iter()
            .find(|entity| entity.position == position)
            .map(|entity| entity.id)
    }

    pub(crate) fn speed(&self, entity: EntityId) -> Speed {
        self.entity(entity).speed
    }

    /// What drives `entity`: a player connection for a character, the
    /// simulation for a creature.
    pub(crate) fn controller(&self, entity: EntityId) -> Controller {
        self.entity(entity).controller
    }

    /// The hit points of `entity` (A7). Only its own controller may learn
    /// them (R18).
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "S3 signatures: the hit rule reads them in the next commit"
        )
    )]
    pub(crate) fn health(&self, entity: EntityId) -> Health {
        self.entity(entity).health
    }

    /// F3: the total lookup. An id this world never issued crashes through
    /// `invariant!`.
    fn lookup(&self, entity: EntityId) -> Lookup<'_> {
        match self
            .entities
            .iter()
            .find(|candidate| candidate.id == entity)
        {
            Some(found) => Lookup::Embodied(found),
            None => Lookup::NotEmbodied,
        }
    }

    /// `entity`, which is embodied: a rule names only entities it perceives
    /// or the one whose turn it is.
    fn entity(&self, entity: EntityId) -> &Entity {
        match self.lookup(entity) {
            Lookup::Embodied(found) => found,
            Lookup::NotEmbodied => ash_core::invariant!("{entity:?} is not embodied"),
        }
    }

    fn entity_mut(&mut self, entity: EntityId) -> &mut Entity {
        match self
            .entities
            .iter_mut()
            .find(|candidate| candidate.id == entity)
        {
            Some(found) => found,
            None => ash_core::invariant!("{entity:?} is not embodied"),
        }
    }
}

#[cfg(test)]
mod tests {
    use ash_core::tile::Direction;

    use super::*;

    #[test]
    #[should_panic(expected = "invariant violated")]
    fn an_entity_created_on_another_crashes() {
        let mut world = World::new(0, Speed::new(1));
        world.create_entity(
            Speed::new(2),
            CHARACTER_START,
            Controller::Wander(Disposition::Neutral),
        );
    }

    #[test]
    #[should_panic(expected = "invariant violated")]
    fn an_entity_created_on_a_wall_crashes() {
        let mut world = World::new(0, Speed::new(1));
        let wall = TilePosition { x: 0, y: 0, z: 0 };
        world.create_entity(
            Speed::new(2),
            wall,
            Controller::Wander(Disposition::Neutral),
        );
    }

    #[test]
    fn a_removed_entity_is_issued_but_not_embodied_and_stays_scheduled() {
        let mut world = World::new(0, Speed::new(1));
        let character = world.awaited();
        let creature = world.create_entity(
            Speed::new(2),
            TilePosition { x: 2, y: 1, z: 0 },
            Controller::Wander(Disposition::Hostile),
        );
        let scheduled = world.scheduler.clone();

        world.apply(Effects {
            in_order: alloc::vec![Effect::Remove { entity: creature }],
            rng_after: world.rng.clone(),
        });

        // F3: the two cases of a total lookup.
        assert!(matches!(world.lookup(creature), Lookup::NotEmbodied));
        assert!(matches!(world.lookup(character), Lookup::Embodied(_)));
        // The entry outlives the embodiment: `advance` skips it later.
        assert_eq!(world.scheduler, scheduled);
    }

    #[test]
    #[should_panic(expected = "invariant violated")]
    fn a_lookup_of_an_id_the_world_never_issued_crashes() {
        // The world issued one id, the character's: the next one is the
        // nearest id it never issued.
        let world = World::new(0, Speed::new(1));
        let _ = world.lookup(EntityId::new(1));
    }

    #[test]
    fn a_wandering_creature_next_to_an_entity_is_offered_the_attack() {
        // R3: the offers do not depend on the controller. The wander sets
        // the attack aside itself (A13).
        let mut world = World::new(0, Speed::new(1));
        let east = TilePosition { x: 2, y: 1, z: 0 };
        let creature = world.create_entity(
            Speed::new(2),
            east,
            Controller::Wander(Disposition::Neutral),
        );
        let attacks: Vec<Action> = world
            .actions_of(creature)
            .into_iter()
            .map(|offered| offered.action)
            .filter(|action| matches!(action, Action::Attack(_)))
            .collect();
        assert_eq!(attacks, [Action::Attack(Direction::West)]);
    }
}
