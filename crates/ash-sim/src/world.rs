//! One simulated world: its clock, entities, schedule and history.

use alloc::vec::Vec;

use ash_core::entity::{EntityId, Speed};
use ash_core::rng::Rng;
use ash_core::time::Time;

use crate::action::{Action, AvailableAction};
use crate::perception::PerceivedChange;
use crate::scheduler::Scheduler;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Entity {
    id: EntityId,
    speed: Speed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct World {
    clock: Time,
    entities: Vec<Entity>,
    scheduler: Scheduler,
    awaited: EntityId,
    rng: Rng,
    perceived_changes: Vec<PerceivedChange>,
    command_log: Vec<Action>,
}

impl World {
    /// A world holding one character, due at time zero.
    pub fn new(seed: u64, character_speed: Speed) -> Self {
        let character = EntityId::new(0);
        let mut world = Self {
            clock: Time::ZERO,
            entities: alloc::vec![Entity {
                id: character,
                speed: character_speed,
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

    pub fn clock(&self) -> Time {
        self.clock
    }

    /// The character whose controller the world waits for. Time stops until
    /// it acts (R2).
    pub fn awaited(&self) -> EntityId {
        self.awaited
    }

    /// The actions the awaited character can take, with their costs (R9).
    pub fn available_actions(&self) -> Vec<AvailableAction> {
        let speed = self.speed(self.awaited);
        [Action::Wait]
            .into_iter()
            .map(|action| AvailableAction {
                action,
                cost: action.cost(speed),
            })
            .collect()
    }

    /// The awaited character takes `action`, then time runs to the next due
    /// character.
    pub fn submit(&mut self, action: Action) {
        let entity = self.awaited;
        // R7: the effect happens now; the cost delays the entity's next turn.
        self.perceived_changes.push(PerceivedChange {
            time: self.clock,
            entity,
            action,
        });
        self.command_log.push(action);
        let due = self.clock.after(action.cost(self.speed(entity)));
        self.scheduler.schedule(entity, due);
        self.advance();
    }

    /// Every change, in the order it happened (R17). Under whole-map
    /// perception (R16), every character perceives every change.
    pub fn perceived_changes(&self) -> &[PerceivedChange] {
        &self.perceived_changes
    }

    /// The accepted commands, in order: replaying them on a world with the
    /// same seed gives the same state.
    pub fn command_log(&self) -> &[Action] {
        &self.command_log
    }

    /// Runs the clock to the next due entity and waits for its controller
    /// (R2). Every entity of this slice is a character.
    fn advance(&mut self) {
        match self.scheduler.pop() {
            Some((due, entity)) => {
                self.clock = due;
                self.awaited = entity;
            }
            None => ash_core::invariant!("no entity is scheduled"),
        }
    }

    fn speed(&self, entity: EntityId) -> Speed {
        match self
            .entities
            .iter()
            .find(|candidate| candidate.id == entity)
        {
            Some(found) => found.speed,
            None => ash_core::invariant!("{entity:?} is not in the world"),
        }
    }
}
