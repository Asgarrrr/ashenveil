//! Orders entities by the time they are due (R8).

use alloc::vec::Vec;

use ash_core::entity::EntityId;
use ash_core::time::Time;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Scheduler {
    entries: Vec<(Time, EntityId)>,
}

impl Scheduler {
    /// Makes `entity` due at `due`.
    pub fn schedule(&mut self, _entity: EntityId, _due: Time) {}

    /// Removes and returns the entity that acts next, with its due time.
    pub fn pop(&mut self) -> Option<(Time, EntityId)> {
        self.entries.pop()
    }
}
