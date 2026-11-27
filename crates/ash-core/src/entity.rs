//! Entity identity and speed.

use core::num::NonZeroU64;

/// Names one entity of a world: its identity (ADR 0001, F2). Its bits and
/// its order carry no meaning.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EntityId(u32);

impl EntityId {
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    pub const fn get(self) -> u32 {
        self.0
    }
}

/// Issues the ids of one world, each once (F2): an id is never issued
/// again, whatever became of its entity.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EntityIds {
    /// How many ids were issued. The width is a mechanism.
    issued: u32,
}

impl EntityIds {
    /// The id the next `issue` returns.
    pub fn peek(&self) -> EntityId {
        EntityId(self.issued)
    }

    /// An id never issued before. Once every id is issued, the next call
    /// crashes through `invariant!`.
    pub fn issue(&mut self) -> EntityId {
        let id = self.peek();
        match self.issued.checked_add(1) {
            Some(issued) => self.issued = issued,
            None => crate::invariant!("every entity id of the world is issued"),
        }
        id
    }

    /// Whether `id` was issued here.
    pub fn has_issued(&self, id: EntityId) -> bool {
        id.0 < self.issued
    }
}

/// R6: the divisor of an entity's base costs. Never zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Speed(NonZeroU64);

impl Speed {
    /// A zero speed crashes through `invariant!`.
    pub fn new(value: u64) -> Self {
        match NonZeroU64::new(value) {
            Some(value) => Self(value),
            None => crate::invariant!("an entity speed is zero"),
        }
    }

    pub(crate) const fn get(self) -> u64 {
        self.0.get()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_issued_id_is_never_issued_again() {
        let mut ids = EntityIds::default();
        let mut issued = Vec::new();
        for _ in 0..5 {
            let next = ids.peek();
            assert!(!ids.has_issued(next), "{next:?} is issued before its turn");
            let id = ids.issue();
            assert_eq!(id, next);
            assert!(!issued.contains(&id), "{id:?} is issued twice");
            issued.push(id);
        }
        assert!(issued.iter().all(|&id| ids.has_issued(id)));
        assert!(!ids.has_issued(ids.peek()));
    }

    /// F2: the boundary of the id space. `u32::MAX` ids fit the counter.
    #[test]
    fn the_last_id_is_issued() {
        let mut ids = EntityIds {
            issued: u32::MAX - 1,
        };
        let last = ids.issue();
        assert_eq!(last, EntityId::new(u32::MAX - 1));
        assert!(ids.has_issued(last));
    }

    #[test]
    #[should_panic(expected = "invariant violated")]
    fn an_id_past_the_last_crashes() {
        let mut ids = EntityIds { issued: u32::MAX };
        let _ = ids.issue();
    }

    #[test]
    #[should_panic(expected = "invariant violated")]
    fn zero_speed_crashes() {
        let _ = Speed::new(0);
    }
}
