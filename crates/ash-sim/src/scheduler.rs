//! Orders entities by the time they are due (R8).

use alloc::vec::Vec;

use ash_core::entity::EntityId;
use ash_core::time::Time;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Scheduler {
    /// Sorted from the last to act to the next to act, so `pop` takes the end.
    entries: Vec<(Time, EntityId)>,
}

impl Scheduler {
    /// Makes `entity` due at `due`, after every entity already due then.
    pub fn schedule(&mut self, entity: EntityId, due: Time) {
        // Before the entries with the same due time: they became due first.
        let position = self.entries.partition_point(|&(time, _)| time > due);
        self.entries.insert(position, (due, entity));
    }

    /// Removes and returns the entity that acts next, with its due time.
    pub fn pop(&mut self) -> Option<(Time, EntityId)> {
        self.entries.pop()
    }
}
