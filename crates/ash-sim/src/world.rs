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
        Self {
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
        }
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
        Vec::new()
    }

    /// The awaited character takes `action`, then time runs to the next due
    /// character.
    pub fn submit(&mut self, _action: Action) {}

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
}
