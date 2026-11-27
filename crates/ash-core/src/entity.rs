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
    /// How many ids were issued, at most `u32::MAX`: the id `u32::MAX` is
    /// never issued. The width is a mechanism.
    issued: u32,
}

impl EntityIds {
    /// An id never issued before. The last one is `u32::MAX - 1`: the call
    /// after it crashes through `invariant!`.
    pub fn issue(&mut self) -> EntityId {
        let id = EntityId(self.issued);
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
            let before = ids.clone();
            let id = ids.issue();
            assert!(!before.has_issued(id), "{id:?} is issued before its turn");
            assert!(!issued.contains(&id), "{id:?} is issued twice");
            issued.push(id);
        }
        assert!(issued.iter().all(|&id| ids.has_issued(id)));
    }

    /// F2: the boundary of the id space. The counter holds how many ids
    /// were issued, so it stops at `u32::MAX` ids, from 0 to `u32::MAX - 1`.
    #[test]
    fn the_last_id_issued_is_one_below_the_last_value_of_u32() {
        let mut ids = EntityIds {
            issued: u32::MAX - 1,
        };
        let last = ids.issue();
        assert_eq!(last, EntityId::new(u32::MAX - 1));
        assert!(ids.has_issued(last));
    }

    #[test]
    #[should_panic(expected = "invariant violated")]
    fn the_last_value_of_u32_is_never_issued() {
        let mut ids = EntityIds { issued: u32::MAX };
        let _ = ids.issue();
    }

    #[test]
    #[should_panic(expected = "invariant violated")]
    fn zero_speed_crashes() {
        let _ = Speed::new(0);
    }
}
